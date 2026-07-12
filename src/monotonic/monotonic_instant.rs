// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a monotonic instant scoped to one clock domain.

use crate::TimeError;
use std::cmp::Ordering;
use std::time::Duration;

/// A fixed point in one monotonic clock domain.
///
/// Instants from different domains cannot be ordered or used in arithmetic
/// together. The value carries the full precision available through
/// [`Duration`] without claiming any particular hardware timer resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MonotonicInstant {
    domain_id: u64,
    elapsed: Duration,
}

impl MonotonicInstant {
    /// Creates an instant for a clock implementation.
    ///
    /// `domain_id` identifies the originating clock and `elapsed` is measured
    /// from that clock's private origin.
    pub(crate) const fn new(domain_id: u64, elapsed: Duration) -> Self {
        Self { domain_id, elapsed }
    }

    /// Returns the identifier of the originating monotonic clock domain.
    #[must_use]
    pub const fn domain_id(self) -> u64 {
        self.domain_id
    }

    /// Returns the elapsed duration from this clock domain's origin.
    ///
    /// The value is meaningful only inside the domain identified by
    /// [`domain_id()`](Self::domain_id).
    #[must_use]
    pub const fn elapsed_since_origin(self) -> Duration {
        self.elapsed
    }

    /// Adds a duration while preserving the originating clock domain.
    ///
    /// Returns [`TimeError::InstantOverflow`] when the result cannot be
    /// represented by [`Duration`].
    pub fn checked_add(self, duration: Duration) -> Result<Self, TimeError> {
        let elapsed = self
            .elapsed
            .checked_add(duration)
            .ok_or(TimeError::InstantOverflow)?;
        Ok(Self::new(self.domain_id, elapsed))
    }

    /// Calculates the duration elapsed since an earlier instant.
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] when `earlier` belongs to a
    /// different clock, and [`TimeError::InvalidInstantOrder`] when `earlier`
    /// is later than this instant.
    pub fn duration_since(self, earlier: Self) -> Result<Duration, TimeError> {
        earlier.ensure_domain(self.domain_id)?;
        self.elapsed
            .checked_sub(earlier.elapsed)
            .ok_or(TimeError::InvalidInstantOrder)
    }

    /// Verifies that an external instant belongs to `expected_domain_id`.
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign instant.
    pub(crate) fn ensure_domain(
        self,
        expected_domain_id: u64,
    ) -> Result<(), TimeError> {
        if self.domain_id == expected_domain_id {
            Ok(())
        } else {
            Err(TimeError::ClockDomainMismatch {
                expected: expected_domain_id,
                actual: self.domain_id,
            })
        }
    }
}

impl PartialOrd for MonotonicInstant {
    /// Orders two instants only when they belong to the same clock domain.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.domain_id == other.domain_id)
            .then(|| self.elapsed.cmp(&other.elapsed))
    }
}
