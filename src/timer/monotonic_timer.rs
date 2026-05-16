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
    TimerDomainId,
    TimerError,
    TimerInstant,
};

/// Provides a monotonic timer-domain clock.
///
/// Implementations expose instants on a per-timer-domain monotonic axis. A
/// [`TimerInstant`] produced by one timer must not be used with another timer.
pub trait MonotonicTimer: Send + Sync {
    /// Returns the timer domain owned by this timer.
    fn timer_domain(&self) -> TimerDomainId;

    /// Returns the current instant in this timer's domain.
    fn now(&self) -> TimerInstant;

    /// Creates a deadline after the specified relative duration.
    ///
    /// The duration is relative to the current instant returned by this timer.
    fn deadline_after(&self, duration: Duration) -> TimerInstant {
        self.now().saturating_add(duration)
    }

    /// Returns the remaining duration until a deadline in this timer's domain.
    ///
    /// Returns `Ok(Some(duration))` when the deadline is still in the future,
    /// `Ok(None)` when the deadline has already been reached or passed, and
    /// `Err` when the deadline belongs to another timer domain.
    fn duration_until(&self, deadline: TimerInstant) -> Result<Option<Duration>, TimerError> {
        deadline.ensure_domain(self.timer_domain())?;
        deadline.checked_duration_since(self.now())
    }
}
