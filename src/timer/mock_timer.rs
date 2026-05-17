/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

#[cfg(feature = "tokio")]
use tokio::sync::watch;

#[cfg(feature = "tokio")]
use crate::timer::{AsyncSleeper, AsyncTimerResult, AsyncWaiter};
use crate::timer::{
    BlockingSleeper, BlockingWaiter, TimerDomain, TimerInstant, TimerResult, TimerWaitOutcome,
    WaitNotifier, next_timer_domain_id,
};

/// A manually controlled monotonic timer for deterministic tests.
///
/// Clones share the same timer domain, elapsed time, and notification state.
/// Waiting threads or tasks only make progress when test code advances the
/// elapsed time, sets it directly, resets it, or sends an explicit notification.
#[derive(Clone)]
pub struct MockTimer {
    domain_id: u64,
    shared: Arc<MockTimerShared>,
    #[cfg(feature = "tokio")]
    async_time_epoch_sender: watch::Sender<u64>,
    #[cfg(feature = "tokio")]
    async_notification_epoch_sender: watch::Sender<u64>,
}

/// Shared state and condition variables for cloned mock timers.
struct MockTimerShared {
    state: Mutex<MockTimerState>,
    time_changed: Condvar,
    wait_changed: Condvar,
}

/// Mutable mock timer state guarded by [`MockTimerShared::state`].
struct MockTimerState {
    elapsed: Duration,
    time_epoch: u64,
    notification_epoch: u64,
}

/// Snapshot of mock timer state read under the internal lock.
struct MockTimerSnapshot {
    elapsed: Duration,
    #[cfg(feature = "tokio")]
    time_epoch: u64,
    #[cfg(feature = "tokio")]
    notification_epoch: u64,
}

impl MockTimer {
    /// Creates a new mock timer whose elapsed time starts at zero.
    ///
    /// # Returns
    ///
    /// A new [`MockTimer`] with a unique timer domain and zero elapsed time.
    pub fn new() -> Self {
        #[cfg(feature = "tokio")]
        let (async_time_epoch_sender, _) = watch::channel(0);
        #[cfg(feature = "tokio")]
        let (async_notification_epoch_sender, _) = watch::channel(0);

        Self {
            domain_id: next_timer_domain_id(),
            shared: Arc::new(MockTimerShared {
                state: Mutex::new(MockTimerState {
                    elapsed: Duration::ZERO,
                    time_epoch: 0,
                    notification_epoch: 0,
                }),
                time_changed: Condvar::new(),
                wait_changed: Condvar::new(),
            }),
            #[cfg(feature = "tokio")]
            async_time_epoch_sender,
            #[cfg(feature = "tokio")]
            async_notification_epoch_sender,
        }
    }

    /// Sets the elapsed time since this timer's zero point.
    ///
    /// This wakes sleepers and waiters so they can re-check their deadlines.
    /// Waiters whose deadlines are still pending continue waiting.
    ///
    /// # Arguments
    ///
    /// * `elapsed` - The new elapsed time for this timer domain.
    pub fn set_elapsed(&self, elapsed: Duration) {
        self.update_elapsed(|_| elapsed);
    }

    /// Advances the elapsed time by the specified duration.
    ///
    /// This wakes sleepers and waiters so they can re-check their deadlines.
    /// The elapsed time saturates at [`Duration::MAX`] on overflow.
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

    /// Returns the current elapsed time and epochs.
    ///
    /// # Returns
    ///
    /// A snapshot read under the timer's internal lock.
    fn current_state(&self) -> MockTimerSnapshot {
        let state = self
            .shared
            .state
            .lock()
            .expect("mock timer state should not be poisoned");
        Self::snapshot(&state)
    }

    /// Creates an immutable snapshot from a locked mock timer state.
    ///
    /// # Arguments
    ///
    /// * `state` - The locked state to read.
    ///
    /// # Returns
    ///
    /// The current elapsed time, time epoch, and notification epoch.
    fn snapshot(state: &MockTimerState) -> MockTimerSnapshot {
        MockTimerSnapshot {
            elapsed: state.elapsed,
            #[cfg(feature = "tokio")]
            time_epoch: state.time_epoch,
            #[cfg(feature = "tokio")]
            notification_epoch: state.notification_epoch,
        }
    }

    /// Locks the mock timer state.
    ///
    /// # Returns
    ///
    /// A guard for the timer state.
    fn lock_state(&self) -> MutexGuard<'_, MockTimerState> {
        self.shared
            .state
            .lock()
            .expect("mock timer state should not be poisoned")
    }

    /// Updates elapsed time, advances the time epoch, and wakes time observers.
    ///
    /// # Arguments
    ///
    /// * `update` - A function that receives the current elapsed time and returns
    ///   the new elapsed time.
    fn update_elapsed(&self, update: impl FnOnce(Duration) -> Duration) {
        let time_epoch = {
            let mut state = self.lock_state();
            state.elapsed = update(state.elapsed);
            state.time_epoch = state.time_epoch.wrapping_add(1);
            state.time_epoch
        };

        self.shared.time_changed.notify_all();
        self.shared.wait_changed.notify_all();
        self.notify_async_time_changed(time_epoch);
    }

    /// Updates the async time epoch when Tokio support is enabled.
    ///
    /// # Arguments
    ///
    /// * `time_epoch` - The time epoch to publish to async sleepers and waiters.
    fn notify_async_time_changed(&self, time_epoch: u64) {
        #[cfg(feature = "tokio")]
        self.async_time_epoch_sender.send_replace(time_epoch);
        #[cfg(not(feature = "tokio"))]
        let _ = time_epoch;
    }

    /// Updates the async notification epoch when Tokio support is enabled.
    ///
    /// # Arguments
    ///
    /// * `notification_epoch` - The notification epoch to publish to async waiters.
    fn notify_async_waiters(&self, notification_epoch: u64) {
        #[cfg(feature = "tokio")]
        self.async_notification_epoch_sender
            .send_replace(notification_epoch);
        #[cfg(not(feature = "tokio"))]
        let _ = notification_epoch;
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

impl TimerDomain for MockTimer {
    /// Returns the timer domain ID owned by this mock timer.
    ///
    /// # Returns
    ///
    /// The numeric ID assigned when this timer was first created.
    fn id(&self) -> u64 {
        self.domain_id
    }

    /// Returns the current mock instant.
    ///
    /// # Returns
    ///
    /// A [`TimerInstant`] reflecting the mock elapsed time under this timer domain ID.
    fn now(&self) -> TimerInstant {
        TimerInstant::new(self.domain_id, self.current_state().elapsed)
    }
}

impl BlockingSleeper for MockTimer {
    /// Blocks until the mock elapsed time reaches the deadline.
    ///
    /// Progress requires test code to call [`advance`](MockTimer::advance),
    /// [`set_elapsed`](MockTimer::set_elapsed), or [`reset`](MockTimer::reset).
    /// Notifications do not complete this sleep.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` once mock elapsed time reaches `deadline`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` belongs to
    /// another timer domain.
    fn sleep_until(&self, deadline: TimerInstant) -> TimerResult<()> {
        deadline.ensure_domain_id(self.domain_id)?;
        let deadline_elapsed = deadline.elapsed_since_timer_start();
        let mut state = self.lock_state();
        loop {
            if state.elapsed >= deadline_elapsed {
                return Ok(());
            }
            state = self
                .shared
                .time_changed
                .wait(state)
                .expect("mock timer state should not be poisoned");
        }
    }
}

impl WaitNotifier for MockTimer {
    /// Wakes current waiters without changing the mock elapsed time.
    ///
    /// Blocking and, when the `tokio` feature is enabled, asynchronous waiters
    /// return [`TimerWaitOutcome::Notified`] unless the deadline has already been
    /// reached. Sleepers are not notified.
    fn notify_all_waiters(&self) {
        let notification_epoch = {
            let mut state = self.lock_state();
            state.notification_epoch = state.notification_epoch.wrapping_add(1);
            state.notification_epoch
        };

        self.shared.wait_changed.notify_all();
        self.notify_async_waiters(notification_epoch);
    }
}

impl BlockingWaiter for MockTimer {
    /// Blocks until the mock elapsed time reaches the deadline or waiters are
    /// notified.
    ///
    /// Progress requires test code to call [`advance`](MockTimer::advance),
    /// [`set_elapsed`](MockTimer::set_elapsed), [`reset`](MockTimer::reset), or
    /// [`notify_all_waiters`](WaitNotifier::notify_all_waiters).
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// The same outcomes as [`BlockingWaiter::wait_until`].
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` belongs to
    /// another timer domain.
    fn wait_until(&self, deadline: TimerInstant) -> TimerResult<TimerWaitOutcome> {
        deadline.ensure_domain_id(self.domain_id)?;
        let deadline_elapsed = deadline.elapsed_since_timer_start();
        let mut state = self.lock_state();
        let observed_notification_epoch = state.notification_epoch;

        loop {
            if state.elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }
            if state.notification_epoch != observed_notification_epoch {
                return Ok(TimerWaitOutcome::Notified);
            }
            state = self
                .shared
                .wait_changed
                .wait(state)
                .expect("mock timer state should not be poisoned");
        }
    }
}

#[cfg(feature = "tokio")]
impl AsyncSleeper for MockTimer {
    /// Waits asynchronously until mock elapsed time reaches the deadline.
    ///
    /// Progress requires test code to advance the mock timer. Notifications do
    /// not complete this sleep.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// A future that resolves once mock elapsed time reaches `deadline`.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// belongs to another timer domain.
    fn sleep_until_async<'a>(&'a self, deadline: TimerInstant) -> AsyncTimerResult<'a, ()> {
        Box::pin(async move {
            deadline.ensure_domain_id(self.domain_id)?;
            let deadline_elapsed = deadline.elapsed_since_timer_start();
            let snapshot = self.current_state();
            if snapshot.elapsed >= deadline_elapsed {
                return Ok(());
            }

            let mut time_receiver = self.async_time_epoch_sender.subscribe();
            if *time_receiver.borrow() != snapshot.time_epoch
                && self.current_state().elapsed >= deadline_elapsed
            {
                return Ok(());
            }

            loop {
                if self.current_state().elapsed >= deadline_elapsed {
                    return Ok(());
                }
                if time_receiver.changed().await.is_err() {
                    return Ok(());
                }
            }
        })
    }
}

#[cfg(feature = "tokio")]
impl AsyncWaiter for MockTimer {
    /// Waits asynchronously until mock elapsed time reaches the deadline or
    /// waiters are notified.
    ///
    /// Progress requires test code to advance or notify the mock timer, as for
    /// the blocking [`wait_until`](BlockingWaiter::wait_until) implementation.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// A future with the same outcomes as [`AsyncWaiter::wait_until_async`].
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// belongs to another timer domain.
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> AsyncTimerResult<'a, TimerWaitOutcome> {
        let snapshot = self.current_state();
        let mut time_receiver = self.async_time_epoch_sender.subscribe();
        let mut notification_receiver = self.async_notification_epoch_sender.subscribe();
        Box::pin(async move {
            deadline.ensure_domain_id(self.domain_id)?;
            let deadline_elapsed = deadline.elapsed_since_timer_start();
            if snapshot.elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }

            if *notification_receiver.borrow() != snapshot.notification_epoch {
                return Ok(self.outcome_after_async_notification(deadline_elapsed));
            }
            if *time_receiver.borrow() != snapshot.time_epoch
                && self.current_state().elapsed >= deadline_elapsed
            {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }

            loop {
                if self.current_state().elapsed >= deadline_elapsed {
                    return Ok(TimerWaitOutcome::DeadlineReached);
                }
                tokio::select! {
                    result = notification_receiver.changed() => {
                        if result.is_err() {
                            return Ok(TimerWaitOutcome::Notified);
                        }
                        return Ok(self.outcome_after_async_notification(deadline_elapsed));
                    }
                    result = time_receiver.changed() => {
                        if result.is_err() {
                            return Ok(TimerWaitOutcome::DeadlineReached);
                        }
                    }
                }
            }
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
    fn outcome_after_async_notification(&self, deadline_elapsed: Duration) -> TimerWaitOutcome {
        if self.current_state().elapsed >= deadline_elapsed {
            TimerWaitOutcome::DeadlineReached
        } else {
            TimerWaitOutcome::Notified
        }
    }
}
