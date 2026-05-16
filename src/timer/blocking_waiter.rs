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
    TimerInstant,
    TimerResult,
    TimerWaitOutcome,
    WaitNotifier,
};

/// Adds notification-sensitive blocking wait operations to a timer domain.
///
/// Wait operations block the current thread until either the deadline is reached
/// or [`WaitNotifier::notify_all_waiters`] broadcasts a notification.
pub trait BlockingWaiter: WaitNotifier {
    /// Blocks the current thread until the deadline is reached or waiters are
    /// explicitly notified.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer domain.
    ///
    /// # Returns
    ///
    /// * `Ok(TimerWaitOutcome::DeadlineReached)` when the deadline has been reached.
    /// * `Ok(TimerWaitOutcome::Notified)` when notification woke the wait before
    ///   the deadline.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` was created by
    /// a different timer domain.
    fn wait_until(&self, deadline: TimerInstant) -> TimerResult<TimerWaitOutcome>;

    /// Blocks the current thread for a duration relative to this timer's current
    /// instant, or until waiters are notified.
    ///
    /// This is equivalent to [`wait_until`](Self::wait_until) with a deadline
    /// created by [`TimerDomain::deadline_after`](crate::timer::TimerDomain::deadline_after).
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
    fn wait_for(&self, duration: Duration) -> TimerResult<TimerWaitOutcome> {
        self.wait_until(self.deadline_after(duration))
    }
}
