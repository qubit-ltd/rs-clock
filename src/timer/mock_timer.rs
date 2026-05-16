/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
#[cfg(feature = "tokio")]
use std::future::Future;
#[cfg(feature = "tokio")]
use std::pin::Pin;
use std::sync::{
    Arc,
    Condvar,
    Mutex,
};
use std::time::Duration;

#[cfg(feature = "tokio")]
use tokio::sync::watch;

#[cfg(feature = "tokio")]
use crate::timer::AsyncTimer;
use crate::timer::{
    BlockingTimer,
    MonotonicTimer,
    TimerDomainId,
    TimerError,
    TimerInstant,
    TimerWaitOutcome,
};

/// A manually controlled monotonic timer for deterministic tests.
///
/// Clones share the same timer domain, elapsed time, and notification state.
/// Waiting threads or tasks only make progress when test code advances the
/// elapsed time, sets it directly, resets it, or sends an explicit notification.
#[derive(Clone)]
pub struct MockTimer {
    domain_id: TimerDomainId,
    state: Arc<(Mutex<(Duration, u64)>, Condvar)>,
    #[cfg(feature = "tokio")]
    async_generation_sender: watch::Sender<u64>,
}

impl MockTimer {
    /// Creates a new mock timer whose elapsed time starts at zero.
    ///
    /// # Returns
    ///
    /// A new [`MockTimer`] with a unique timer domain and zero elapsed time.
    pub fn new() -> Self {
        #[cfg(feature = "tokio")]
        let (async_generation_sender, _) = watch::channel(0);

        Self {
            domain_id: TimerDomainId::new_unique(),
            state: Arc::new((Mutex::new((Duration::ZERO, 0)), Condvar::new())),
            #[cfg(feature = "tokio")]
            async_generation_sender,
        }
    }

    /// Sets the elapsed time since this timer's zero point.
    ///
    /// This wakes both blocking and asynchronous waiters. Waiters whose
    /// deadlines are not reached observe a notification instead.
    ///
    /// # Arguments
    ///
    /// * `elapsed` - The new elapsed time for this timer domain.
    pub fn set_elapsed(&self, elapsed: Duration) {
        self.update_elapsed(|_| elapsed);
    }

    /// Advances the elapsed time by the specified duration.
    ///
    /// This wakes both blocking and asynchronous waiters. The elapsed time
    /// saturates at [`Duration::MAX`] on overflow.
    ///
    /// # Arguments
    ///
    /// * `duration` - The amount of time to add to the current elapsed time.
    pub fn advance(&self, duration: Duration) {
        self.update_elapsed(|elapsed| elapsed.saturating_add(duration));
    }

    /// Resets the elapsed time to zero and wakes waiters.
    ///
    /// Equivalent to [`set_elapsed`](Self::set_elapsed) with [`Duration::ZERO`].
    pub fn reset(&self) {
        self.set_elapsed(Duration::ZERO);
    }

    /// Returns the current elapsed time and notification generation.
    ///
    /// # Returns
    ///
    /// A tuple of `(elapsed, generation)` read under the timer's internal lock.
    fn current_state(&self) -> (Duration, u64) {
        let (state_lock, _) = self.state.as_ref();
        let guard = state_lock
            .lock()
            .expect("mock timer state should not be poisoned");
        let (elapsed, generation) = *guard;
        (elapsed, generation)
    }

    /// Updates elapsed time, increments the generation, and wakes waiters.
    ///
    /// # Arguments
    ///
    /// * `update` - A function that receives the current elapsed time and returns
    ///   the new elapsed time.
    fn update_elapsed(&self, update: impl FnOnce(Duration) -> Duration) {
        let (state_lock, condition) = self.state.as_ref();
        let generation = {
            let mut guard = state_lock
                .lock()
                .expect("mock timer state should not be poisoned");
            let (elapsed, generation) = &mut *guard;
            *elapsed = update(*elapsed);
            *generation = generation.wrapping_add(1);
            *generation
        };

        condition.notify_all();
        self.notify_async_waiters(generation);
    }

    /// Updates the async generation when Tokio support is enabled.
    ///
    /// # Arguments
    ///
    /// * `generation` - The notification generation to publish to async waiters.
    fn notify_async_waiters(&self, generation: u64) {
        #[cfg(feature = "tokio")]
        self.async_generation_sender.send_replace(generation);
        #[cfg(not(feature = "tokio"))]
        let _ = generation;
    }
}

impl Default for MockTimer {
    /// Creates a new mock timer.
    ///
    /// # Returns
    ///
    /// A timer equivalent to [`MockTimer::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicTimer for MockTimer {
    /// Returns the timer domain ID owned by this mock timer.
    ///
    /// # Returns
    ///
    /// The [`TimerDomainId`] assigned when this timer was first created.
    fn timer_domain_id(&self) -> TimerDomainId {
        self.domain_id
    }

    /// Returns the current mock instant.
    ///
    /// # Returns
    ///
    /// A [`TimerInstant`] reflecting the mock elapsed time under this timer domain ID.
    fn now(&self) -> TimerInstant {
        let (elapsed, _) = self.current_state();
        TimerInstant::new(self.domain_id, elapsed)
    }
}

impl BlockingTimer for MockTimer {
    /// Blocks until the mock elapsed time reaches the deadline or waiters are
    /// notified.
    ///
    /// Progress requires test code to call [`advance`](MockTimer::advance),
    /// [`set_elapsed`](MockTimer::set_elapsed), [`reset`](MockTimer::reset), or
    /// [`notify_waiters`](BlockingTimer::notify_waiters).
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// The same outcomes as [`BlockingTimer::wait_until`].
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` belongs to
    /// another timer domain.
    fn wait_until(&self, deadline: TimerInstant) -> Result<TimerWaitOutcome, TimerError> {
        deadline.ensure_domain_id(self.domain_id)?;
        let deadline_elapsed = deadline.elapsed_since_timer_start();
        let (state_lock, condition) = self.state.as_ref();
        let mut guard = state_lock
            .lock()
            .expect("mock timer state should not be poisoned");
        let (_, observed_generation) = *guard;

        loop {
            let (elapsed, generation) = *guard;
            if elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }
            if generation != observed_generation {
                return Ok(TimerWaitOutcome::Notified);
            }
            guard = condition
                .wait(guard)
                .expect("mock timer state should not be poisoned");
        }
    }

    /// Wakes current waiters without changing the mock elapsed time.
    ///
    /// Blocking and, when the `tokio` feature is enabled, asynchronous waiters
    /// return [`TimerWaitOutcome::Notified`] unless the deadline has already been
    /// reached.
    fn notify_waiters(&self) {
        let (state_lock, condition) = self.state.as_ref();
        let generation = {
            let mut guard = state_lock
                .lock()
                .expect("mock timer state should not be poisoned");
            let (_, generation) = &mut *guard;
            *generation = generation.wrapping_add(1);
            *generation
        };

        condition.notify_all();
        self.notify_async_waiters(generation);
    }
}

#[cfg(feature = "tokio")]
impl AsyncTimer for MockTimer {
    /// Waits asynchronously until mock elapsed time reaches the deadline or
    /// waiters are notified.
    ///
    /// Progress requires test code to advance or notify the mock timer, as for
    /// the blocking [`wait_until`](BlockingTimer::wait_until) implementation.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// A future with the same outcomes as [`AsyncTimer::wait_until_async`].
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// belongs to another timer domain.
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> Pin<Box<dyn Future<Output = Result<TimerWaitOutcome, TimerError>> + Send + 'a>> {
        Box::pin(async move {
            deadline.ensure_domain_id(self.domain_id)?;
            let deadline_elapsed = deadline.elapsed_since_timer_start();
            let (elapsed, observed_generation) = self.current_state();
            if elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }

            let mut generation_receiver = self.async_generation_sender.subscribe();
            if *generation_receiver.borrow() != observed_generation {
                return Ok(self.outcome_after_async_wake(deadline_elapsed));
            }

            if generation_receiver.changed().await.is_err() {
                return Ok(TimerWaitOutcome::Notified);
            }

            Ok(self.outcome_after_async_wake(deadline_elapsed))
        })
    }
}

#[cfg(feature = "tokio")]
impl MockTimer {
    /// Returns the wait outcome after an async notification has been observed.
    ///
    /// # Arguments
    ///
    /// * `deadline_elapsed` - The elapsed duration of the waited-for deadline.
    ///
    /// # Returns
    ///
    /// [`TimerWaitOutcome::DeadlineReached`] when mock elapsed time has reached
    /// `deadline_elapsed`, otherwise [`TimerWaitOutcome::Notified`].
    fn outcome_after_async_wake(&self, deadline_elapsed: Duration) -> TimerWaitOutcome {
        let (elapsed, _) = self.current_state();
        if elapsed >= deadline_elapsed {
            TimerWaitOutcome::DeadlineReached
        } else {
            TimerWaitOutcome::Notified
        }
    }
}
