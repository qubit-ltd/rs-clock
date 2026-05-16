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
    domain: TimerDomainId,
    state: Arc<(Mutex<(Duration, u64)>, Condvar)>,
    #[cfg(feature = "tokio")]
    async_generation_sender: watch::Sender<u64>,
}

impl MockTimer {
    /// Creates a new mock timer whose elapsed time starts at zero.
    pub fn new() -> Self {
        #[cfg(feature = "tokio")]
        let (async_generation_sender, _) = watch::channel(0);

        Self {
            domain: TimerDomainId::new_unique(),
            state: Arc::new((Mutex::new((Duration::ZERO, 0)), Condvar::new())),
            #[cfg(feature = "tokio")]
            async_generation_sender,
        }
    }

    /// Sets the elapsed time since this timer's zero point.
    ///
    /// This wakes both blocking and asynchronous waiters. Waiters whose
    /// deadlines are not reached observe a notification instead.
    pub fn set_elapsed(&self, elapsed: Duration) {
        self.update_elapsed(|_| elapsed);
    }

    /// Advances the elapsed time by the specified duration.
    ///
    /// This wakes both blocking and asynchronous waiters. The elapsed time
    /// saturates at [`Duration::MAX`] on overflow.
    pub fn advance(&self, duration: Duration) {
        self.update_elapsed(|elapsed| elapsed.saturating_add(duration));
    }

    /// Resets the elapsed time to zero and wakes waiters.
    pub fn reset(&self) {
        self.set_elapsed(Duration::ZERO);
    }

    /// Returns the current elapsed time and notification generation.
    fn current_state(&self) -> (Duration, u64) {
        let (state_lock, _) = self.state.as_ref();
        let guard = state_lock
            .lock()
            .expect("mock timer state should not be poisoned");
        let (elapsed, generation) = *guard;
        (elapsed, generation)
    }

    /// Updates elapsed time, increments the generation, and wakes waiters.
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
    fn notify_async_waiters(&self, generation: u64) {
        #[cfg(feature = "tokio")]
        self.async_generation_sender.send_replace(generation);
        #[cfg(not(feature = "tokio"))]
        let _ = generation;
    }
}

impl Default for MockTimer {
    /// Creates a new mock timer.
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicTimer for MockTimer {
    /// Returns the timer domain owned by this mock timer.
    fn timer_domain(&self) -> TimerDomainId {
        self.domain
    }

    /// Returns the current mock instant.
    fn now(&self) -> TimerInstant {
        let (elapsed, _) = self.current_state();
        TimerInstant::new(self.domain, elapsed)
    }
}

impl BlockingTimer for MockTimer {
    /// Blocks until the mock elapsed time reaches the deadline or waiters are
    /// notified.
    fn wait_until(&self, deadline: TimerInstant) -> Result<TimerWaitOutcome, TimerError> {
        deadline.ensure_domain(self.domain)?;
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
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> Pin<Box<dyn Future<Output = Result<TimerWaitOutcome, TimerError>> + Send + 'a>> {
        Box::pin(async move {
            deadline.ensure_domain(self.domain)?;
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
    fn outcome_after_async_wake(&self, deadline_elapsed: Duration) -> TimerWaitOutcome {
        let (elapsed, _) = self.current_state();
        if elapsed >= deadline_elapsed {
            TimerWaitOutcome::DeadlineReached
        } else {
            TimerWaitOutcome::Notified
        }
    }
}
