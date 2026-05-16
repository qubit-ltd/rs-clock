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
    TimerDomain,
    TimerInstant,
};

/// Adds Tokio-compatible asynchronous sleep operations to a timer domain.
///
/// Sleep futures resolve only after the deadline is reached. Notifications are
/// not completion signals for this trait.
pub trait AsyncSleeper: TimerDomain {
    /// Waits asynchronously until the deadline has been reached.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer domain.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` once this timer domain has reached or
    /// passed `deadline`.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// was created by a different timer domain.
    fn sleep_until_async<'a>(&'a self, deadline: TimerInstant) -> AsyncTimerResult<'a, ()>;

    /// Waits asynchronously for a duration relative to this timer's current
    /// instant.
    ///
    /// This is equivalent to [`sleep_until_async`](Self::sleep_until_async) with a
    /// deadline created by [`TimerDomain::deadline_after`].
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative delay from the current instant.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` once this timer domain has advanced by
    /// at least `duration` from the instant observed when the future is created.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] only if the
    /// self-created deadline is rejected by an invalid timer implementation.
    fn sleep_for_async<'a>(&'a self, duration: Duration) -> AsyncTimerResult<'a, ()> {
        self.sleep_until_async(self.deadline_after(duration))
    }
}
