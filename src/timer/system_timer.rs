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
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use qubit_lock::{
    ArcMonitor,
    WaitTimeoutResult,
};
#[cfg(feature = "tokio")]
use tokio::sync::Notify;

#[cfg(feature = "tokio")]
use crate::timer::{
    AsyncSleeper,
    AsyncTimerResult,
    AsyncWaiter,
};
use crate::timer::{
    BlockingSleeper,
    BlockingWaiter,
    TimerDomain,
    TimerInstant,
    TimerResult,
    TimerWaitOutcome,
    WaitNotifier,
    next_timer_domain_id,
};

/// A real monotonic timer backed by [`std::time::Instant`].
///
/// Clones share the same timer domain and notification state. Deadlines are
/// measured relative to the instant at which the original timer was created.
#[derive(Clone)]
pub struct SystemTimer {
    domain_id: u64,
    origin: Instant,
    notification_epoch: ArcMonitor<u64>,
    #[cfg(feature = "tokio")]
    async_notifier: Arc<Notify>,
}

impl SystemTimer {
    /// Creates a new system timer with its own timer domain.
    ///
    /// The domain's zero point is the [`std::time::Instant`] captured at
    /// construction. Clones share the same domain, origin, and notification state.
    ///
    /// # Returns
    ///
    /// A new [`SystemTimer`] backed by the system monotonic clock.
    pub fn new() -> Self {
        Self {
            domain_id: next_timer_domain_id(),
            origin: Instant::now(),
            notification_epoch: ArcMonitor::new(0),
            #[cfg(feature = "tokio")]
            async_notifier: Arc::new(Notify::new()),
        }
    }

    /// Advances the blocking notification epoch and wakes blocking waiters.
    ///
    /// Waiters blocked in [`BlockingWaiter::wait_until`] observe an epoch change
    /// and return [`TimerWaitOutcome::Notified`].
    fn wake_blocking_waiters(&self) {
        self.notification_epoch.write_notify_all(|epoch| {
            *epoch = epoch.wrapping_add(1);
        });
    }
}

impl TimerDomain for SystemTimer {
    /// Returns the timer domain ID owned by this system timer.
    ///
    /// # Returns
    ///
    /// The numeric ID assigned when this timer was first created.
    fn id(&self) -> u64 {
        self.domain_id
    }

    /// Returns the current monotonic instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// Elapsed time since the shared origin [`std::time::Instant`], wrapped as a
    /// [`TimerInstant`].
    fn now(&self) -> TimerInstant {
        TimerInstant::new(self.domain_id, self.origin.elapsed())
    }
}

impl BlockingSleeper for SystemTimer {
    /// Blocks the current thread until the deadline has been reached.
    ///
    /// This sleep uses the system scheduler directly and does not observe timer
    /// notifications.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the system timer has reached or passed `deadline`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` belongs to
    /// another timer domain.
    fn sleep_until(&self, deadline: TimerInstant) -> TimerResult<()> {
        deadline.ensure_domain_id(self.domain_id)?;
        let deadline_elapsed = deadline.elapsed_since_timer_start();
        loop {
            let now_elapsed = self.origin.elapsed();
            if now_elapsed >= deadline_elapsed {
                return Ok(());
            }
            thread::sleep(deadline_elapsed - now_elapsed);
        }
    }
}

impl WaitNotifier for SystemTimer {
    /// Wakes all current waiters without changing the timer's monotonic time.
    ///
    /// Blocking waiters and, when the `tokio` feature is enabled, asynchronous
    /// waiters registered on this timer are notified. Sleepers are not notified.
    fn notify_all_waiters(&self) {
        self.wake_blocking_waiters();
        #[cfg(feature = "tokio")]
        self.async_notifier.notify_waiters();
    }
}

impl BlockingWaiter for SystemTimer {
    /// Blocks until the deadline is reached or waiters are notified.
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
        let now_elapsed = self.origin.elapsed();
        if now_elapsed >= deadline_elapsed {
            return Ok(TimerWaitOutcome::DeadlineReached);
        }

        let remaining = deadline_elapsed - now_elapsed;
        // Capture the baseline inside the monitor wait so notifications cannot
        // be lost between a separate epoch read and wait registration.
        let mut observed_epoch = None;
        let result = self.notification_epoch.wait_timeout_until(
            remaining,
            |epoch| match observed_epoch {
                Some(observed_epoch) => *epoch != observed_epoch,
                None => {
                    observed_epoch = Some(*epoch);
                    false
                }
            },
            |_| TimerWaitOutcome::Notified,
        );
        match result {
            WaitTimeoutResult::Ready(outcome) => Ok(outcome),
            WaitTimeoutResult::TimedOut => Ok(TimerWaitOutcome::DeadlineReached),
        }
    }
}

#[cfg(feature = "tokio")]
impl AsyncSleeper for SystemTimer {
    /// Waits asynchronously until the deadline has been reached.
    ///
    /// This sleep uses Tokio's timer directly and does not observe timer
    /// notifications.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` once the system timer has reached or
    /// passed `deadline`.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// belongs to another timer domain.
    fn sleep_until_async<'a>(&'a self, deadline: TimerInstant) -> AsyncTimerResult<'a, ()> {
        Box::pin(async move {
            deadline.ensure_domain_id(self.domain_id)?;
            let deadline_elapsed = deadline.elapsed_since_timer_start();
            loop {
                let now_elapsed = self.origin.elapsed();
                if now_elapsed >= deadline_elapsed {
                    return Ok(());
                }
                tokio::time::sleep(deadline_elapsed - now_elapsed).await;
            }
        })
    }
}

#[cfg(feature = "tokio")]
impl AsyncWaiter for SystemTimer {
    /// Waits asynchronously until the deadline is reached or waiters are
    /// notified.
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
        Box::pin(async move {
            deadline.ensure_domain_id(self.domain_id)?;
            let deadline_elapsed = deadline.elapsed_since_timer_start();
            let now_elapsed = self.origin.elapsed();
            if now_elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }

            let remaining = deadline_elapsed - now_elapsed;
            let outcome = tokio::select! {
                _ = tokio::time::sleep(remaining) => {
                    TimerWaitOutcome::DeadlineReached
                }
                _ = self.async_notifier.notified() => {
                    TimerWaitOutcome::Notified
                }
            };
            Ok(outcome)
        })
    }
}

impl Default for SystemTimer {
    /// Creates a new system timer.
    ///
    /// # Returns
    ///
    /// A timer equivalent to [`SystemTimer::new`].
    fn default() -> Self {
        Self::new()
    }
}
