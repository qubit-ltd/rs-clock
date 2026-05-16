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
    TimerDomain,
    TimerInstant,
    TimerResult,
};

/// Adds blocking sleep operations to a timer domain.
///
/// Sleep operations block the current thread until the deadline is reached.
/// Notifications are not completion signals for this trait.
pub trait BlockingSleeper: TimerDomain {
    /// Blocks the current thread until the deadline has been reached.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` once this timer domain has reached or passed `deadline`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` was created by
    /// a different timer domain.
    fn sleep_until(&self, deadline: TimerInstant) -> TimerResult<()>;

    /// Blocks the current thread for a duration relative to this timer's current
    /// instant.
    ///
    /// This is equivalent to [`sleep_until`](Self::sleep_until) with a deadline
    /// created by [`TimerDomain::deadline_after`].
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative delay from the current instant.
    ///
    /// # Returns
    ///
    /// `Ok(())` once this timer domain has advanced by at least `duration` from
    /// the instant observed at the start of the sleep.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] only if the self-created
    /// deadline is rejected by an invalid timer implementation.
    fn sleep_for(&self, duration: Duration) -> TimerResult<()> {
        self.sleep_until(self.deadline_after(duration))
    }
}
