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
    AsyncTimerResult,
    TimerInstant,
    TimerWaitOutcome,
    WaitNotifier,
};

/// Adds notification-sensitive asynchronous wait operations to a timer domain.
///
/// Wait futures resolve when either the deadline is reached or
/// [`WaitNotifier::notify_all_waiters`] broadcasts a notification.
pub trait AsyncWaiter: WaitNotifier {
    /// Waits asynchronously until the deadline is reached or waiters are
    /// explicitly notified.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer domain.
    ///
    /// # Returns
    ///
    /// A future that resolves to:
    ///
    /// * `Ok(TimerWaitOutcome::DeadlineReached)` when the deadline has been reached.
    /// * `Ok(TimerWaitOutcome::Notified)` when notification woke the wait before
    ///   the deadline.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// was created by a different timer domain.
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> AsyncTimerResult<'a, TimerWaitOutcome>;

    /// Waits asynchronously for a duration relative to this timer's current
    /// instant, or until waiters are notified.
    ///
    /// This is equivalent to [`wait_until_async`](Self::wait_until_async) with a
    /// deadline created by [`TimerDomain::deadline_after`](crate::timer::TimerDomain::deadline_after).
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative delay from the current instant.
    ///
    /// # Returns
    ///
    /// A future with the same outcomes as [`wait_until_async`](Self::wait_until_async).
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] only if the
    /// self-created deadline is rejected by an invalid timer implementation.
    fn wait_for_async<'a>(&'a self, duration: Duration) -> AsyncTimerResult<'a, TimerWaitOutcome> {
        self.wait_until_async(self.deadline_after(duration))
    }
}
