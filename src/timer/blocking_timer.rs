/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::Duration;

use crate::timer::{
    MonotonicTimer,
    TimerError,
    TimerInstant,
    TimerWaitOutcome,
};

/// Adds blocking wait operations to a monotonic timer.
///
/// The `wait_*` methods are notification-sensitive waits: they return
/// [`TimerWaitOutcome::Notified`] when [`notify_waiters`](Self::notify_waiters)
/// wakes them before the deadline. The `sleep_*` methods provide sleep
/// semantics: they use the same notification mechanism only to re-check the
/// deadline, and they keep blocking until the deadline is actually reached.
pub trait BlockingTimer: MonotonicTimer {
    /// Blocks until the deadline is reached or waiters are explicitly notified.
    ///
    /// Unlike [`sleep_until`](Self::sleep_until), this method returns as soon as
    /// waiters are notified, even if the deadline has not been reached yet.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer's domain.
    ///
    /// # Returns
    ///
    /// * `Ok(TimerWaitOutcome::DeadlineReached)` when the deadline has been reached.
    /// * `Ok(TimerWaitOutcome::Notified)` when [`notify_waiters`](Self::notify_waiters)
    ///   woke the wait before the deadline.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` was created by
    /// a different timer domain.
    fn wait_until(&self, deadline: TimerInstant) -> Result<TimerWaitOutcome, TimerError>;

    /// Wakes current waiters without advancing time.
    ///
    /// Blocking waits blocked on [`wait_until`](Self::wait_until) return
    /// [`TimerWaitOutcome::Notified`]. This does not change the monotonic instant
    /// reported by [`MonotonicTimer::now`].
    fn notify_waiters(&self);

    /// Blocks for a duration relative to this timer's current instant.
    ///
    /// This is equivalent to [`wait_until`](Self::wait_until) with a deadline
    /// created by [`MonotonicTimer::deadline_after`].
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative delay from the current instant.
    ///
    /// # Returns
    ///
    /// The same outcomes as [`wait_until`](Self::wait_until).
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] only if the self-created
    /// deadline is rejected by an invalid timer implementation.
    fn wait_for(&self, duration: Duration) -> Result<TimerWaitOutcome, TimerError> {
        self.wait_until(self.deadline_after(duration))
    }

    /// Blocks until the deadline has been reached.
    ///
    /// Explicit notifications wake the underlying [`wait_until`](Self::wait_until)
    /// call, but this method keeps waiting until the deadline is actually reached.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer's domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the timer's monotonic time has reached or passed `deadline`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` was created by
    /// a different timer domain.
    fn sleep_until(&self, deadline: TimerInstant) -> Result<(), TimerError> {
        loop {
            match self.wait_until(deadline)? {
                TimerWaitOutcome::DeadlineReached => return Ok(()),
                TimerWaitOutcome::Notified => {}
            }
        }
    }

    /// Blocks for a duration relative to this timer's current instant.
    ///
    /// This is equivalent to [`sleep_until`](Self::sleep_until) with a deadline
    /// created by [`MonotonicTimer::deadline_after`].
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative delay from the current instant.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the timer's monotonic time has advanced by at least `duration`
    /// from the instant observed at the start of the wait.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] only if the self-created
    /// deadline is rejected by an invalid timer implementation.
    fn sleep_for(&self, duration: Duration) -> Result<(), TimerError> {
        self.sleep_until(self.deadline_after(duration))
    }
}
