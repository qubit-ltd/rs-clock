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
};

/// A monotonic instant inside one timer domain.
///
/// The stored duration is relative to the creation point of the timer domain
/// that produced the instant. Values from different domains are intentionally
/// incompatible, even if their elapsed durations happen to be equal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimerInstant {
    domain: TimerDomainId,
    elapsed: Duration,
}

impl TimerInstant {
    /// Creates an instant in the given timer domain.
    ///
    /// # Arguments
    ///
    /// * `domain` - The timer domain that owns the instant.
    /// * `elapsed` - The elapsed time since that domain was created.
    ///
    /// # Returns
    ///
    /// A new [`TimerInstant`] on the specified domain axis.
    pub(crate) fn new(domain: TimerDomainId, elapsed: Duration) -> Self {
        Self { domain, elapsed }
    }

    /// Returns the timer domain that owns this instant.
    ///
    /// # Returns
    ///
    /// The [`TimerDomainId`] associated with this instant.
    pub fn domain(self) -> TimerDomainId {
        self.domain
    }

    /// Returns the elapsed duration since the owning timer domain was created.
    ///
    /// # Returns
    ///
    /// The monotonic offset stored in this instant, measured from the domain's
    /// zero point.
    pub fn elapsed_since_timer_start(self) -> Duration {
        self.elapsed
    }

    /// Returns a new instant after adding the duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The offset to add to this instant's elapsed time.
    ///
    /// # Returns
    ///
    /// `Some(instant)` on success, or `None` if the addition overflows
    /// [`Duration`].
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        self.elapsed
            .checked_add(duration)
            .map(|elapsed| Self::new(self.domain, elapsed))
    }

    /// Returns a new instant after adding the duration, saturating on overflow.
    ///
    /// # Arguments
    ///
    /// * `duration` - The offset to add to this instant's elapsed time.
    ///
    /// # Returns
    ///
    /// A new instant whose elapsed time is the sum of this instant and `duration`,
    /// or [`Duration::MAX`] if the sum would overflow.
    pub fn saturating_add(self, duration: Duration) -> Self {
        Self::new(self.domain, self.elapsed.saturating_add(duration))
    }

    /// Returns the duration elapsed since an earlier instant in the same domain.
    ///
    /// # Arguments
    ///
    /// * `earlier` - The reference instant, which must belong to the same domain
    ///   as `self`.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(duration))` when `self` is not earlier than `earlier`.
    /// * `Ok(None)` when `earlier` is later than `self`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when the two instants belong to
    /// different timer domains.
    pub fn checked_duration_since(self, earlier: Self) -> Result<Option<Duration>, TimerError> {
        self.ensure_domain(earlier.domain)?;
        Ok(self.elapsed.checked_sub(earlier.elapsed))
    }

    /// Returns the duration elapsed since an earlier instant in the same domain.
    ///
    /// # Arguments
    ///
    /// * `earlier` - The reference instant, which must belong to the same domain
    ///   as `self`.
    ///
    /// # Returns
    ///
    /// The elapsed duration from `earlier` to `self`, saturating at
    /// [`Duration::ZERO`] when `earlier` is later than `self`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when the two instants belong to
    /// different timer domains.
    pub fn saturating_duration_since(self, earlier: Self) -> Result<Duration, TimerError> {
        self.ensure_domain(earlier.domain)?;
        Ok(self.elapsed.saturating_sub(earlier.elapsed))
    }

    /// Verifies that this instant belongs to the expected timer domain.
    ///
    /// # Arguments
    ///
    /// * `expected` - The timer domain required by the caller.
    ///
    /// # Returns
    ///
    /// `Ok(())` when this instant's domain matches `expected`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when the domains differ.
    pub(crate) fn ensure_domain(self, expected: TimerDomainId) -> Result<(), TimerError> {
        if self.domain == expected {
            Ok(())
        } else {
            Err(TimerError::timer_domain_mismatch(expected, self.domain))
        }
    }
}
