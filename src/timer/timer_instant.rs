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
    TimerError,
    TimerResult,
};

/// A monotonic instant inside one timer domain.
///
/// The stored duration is relative to the creation point of the timer domain
/// that produced the instant. Values from different domains are intentionally
/// incompatible, even if their elapsed durations happen to be equal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimerInstant {
    domain_id: u64,
    elapsed: Duration,
}

impl TimerInstant {
    /// Creates an instant in the given timer domain.
    ///
    /// # Arguments
    ///
    /// * `domain_id` - The timer domain ID that owns the instant.
    /// * `elapsed` - The elapsed time since that domain was created.
    ///
    /// # Returns
    ///
    /// A new [`TimerInstant`] on the specified domain axis.
    pub(crate) fn new(domain_id: u64, elapsed: Duration) -> Self {
        Self { domain_id, elapsed }
    }

    /// Returns the timer domain ID that owns this instant.
    ///
    /// # Returns
    ///
    /// The numeric timer domain ID associated with this instant.
    pub fn domain_id(self) -> u64 {
        self.domain_id
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
            .map(|elapsed| Self::new(self.domain_id, elapsed))
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
        Self::new(self.domain_id, self.elapsed.saturating_add(duration))
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
    pub fn checked_duration_since(self, earlier: Self) -> TimerResult<Option<Duration>> {
        self.ensure_domain_id(earlier.domain_id)?;
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
    pub fn saturating_duration_since(self, earlier: Self) -> TimerResult<Duration> {
        self.ensure_domain_id(earlier.domain_id)?;
        Ok(self.elapsed.saturating_sub(earlier.elapsed))
    }

    /// Verifies that this instant belongs to the expected timer domain.
    ///
    /// # Arguments
    ///
    /// * `expected_domain_id` - The timer domain ID required by the caller.
    ///
    /// # Returns
    ///
    /// `Ok(())` when this instant's domain ID matches `expected_domain_id`.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when the domains differ.
    pub(crate) fn ensure_domain_id(self, expected_domain_id: u64) -> TimerResult<()> {
        if self.domain_id == expected_domain_id {
            Ok(())
        } else {
            Err(TimerError::timer_domain_mismatch(
                expected_domain_id,
                self.domain_id,
            ))
        }
    }
}
