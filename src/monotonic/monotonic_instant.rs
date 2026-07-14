// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a monotonic instant scoped to one clock domain.

use crate::{
    ClockDomain,
    TimeError,
};
use std::cmp::Ordering;
use std::time::Duration;

/// A fixed point in one monotonic clock domain.
///
/// Instants from different domains cannot be ordered or used in arithmetic
/// together. The value carries the full precision available through
/// [`Duration`] without claiming any particular hardware timer resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MonotonicInstant {
    domain: ClockDomain,
    elapsed: Duration,
}

impl MonotonicInstant {
    /// Creates an instant for a clock implementation.
    ///
    /// domain identifies the originating clock and elapsed is measured
    /// from that clock's private origin.
    pub const fn new(domain: ClockDomain, elapsed: Duration) -> Self {
        Self { domain, elapsed }
    }

    /// Returns the identifier of the originating monotonic clock domain.
    #[must_use]
    pub const fn domain(self) -> ClockDomain {
        self.domain
    }

    /// Returns the elapsed duration from this clock domain's origin.
    ///
    /// The value is meaningful only inside the domain identified by
    /// [`domain()`](Self::domain).
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
        Ok(Self::new(self.domain, elapsed))
    }

    /// Calculates the duration elapsed since an earlier instant.
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] when `earlier` belongs to a
    /// different clock, and [`TimeError::InvalidInstantOrder`] when `earlier`
    /// is later than this instant.
    pub fn duration_since(self, earlier: Self) -> Result<Duration, TimeError> {
        earlier.ensure_domain(self.domain)?;
        self.elapsed
            .checked_sub(earlier.elapsed)
            .ok_or(TimeError::InvalidInstantOrder)
    }

    /// Verifies that an external instant belongs to expected_domain.
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign instant.
    pub(crate) fn ensure_domain(
        self,
        expected_domain: ClockDomain,
    ) -> Result<(), TimeError> {
        if self.domain == expected_domain {
            Ok(())
        } else {
            Err(TimeError::ClockDomainMismatch {
                expected: expected_domain,
                actual: self.domain,
            })
        }
    }
}

impl PartialOrd for MonotonicInstant {
    /// Orders two instants only when they belong to the same clock domain.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.domain == other.domain).then(|| self.elapsed.cmp(&other.elapsed))
    }
}
