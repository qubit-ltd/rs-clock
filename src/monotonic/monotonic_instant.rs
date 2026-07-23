// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
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
///
/// Discarding a sampled instant is rejected when `unused_must_use` is denied:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use qubit_clock::{ManualMonotonicClock, MonotonicClock};
///
/// ManualMonotonicClock::new().now();
/// ```
#[must_use = "monotonic instants should be used to measure or compare time"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MonotonicInstant {
    /// The identifier of the originating monotonic clock domain.
    domain: ClockDomain,
    /// The elapsed duration from the originating clock domain's origin.
    elapsed: Duration,
}

impl MonotonicInstant {
    /// Creates an instant for a clock implementation.
    ///
    /// # Parameters
    ///
    /// * `domain` - Identifier of the originating clock.
    /// * `elapsed` - Duration measured from that clock's private origin.
    ///
    /// # Returns
    ///
    /// An instant scoped to `domain` at `elapsed`.
    #[inline(always)]
    pub const fn new(domain: ClockDomain, elapsed: Duration) -> Self {
        Self { domain, elapsed }
    }

    /// Returns the identifier of the originating monotonic clock domain.
    ///
    /// # Returns
    ///
    /// The domain carried by this instant.
    #[inline(always)]
    pub const fn domain(self) -> ClockDomain {
        self.domain
    }

    /// Returns the elapsed duration from this clock domain's origin.
    ///
    /// The value is meaningful only inside the domain identified by
    /// [`domain()`](Self::domain).
    ///
    /// # Returns
    ///
    /// The duration from the originating clock's private origin.
    #[must_use]
    #[inline(always)]
    pub const fn elapsed_since_origin(self) -> Duration {
        self.elapsed
    }

    /// Adds a duration while preserving the originating clock domain.
    ///
    /// Returns [`TimeError::InstantOverflow`] when the result cannot be
    /// represented by [`Duration`].
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration to add to this instant.
    ///
    /// # Returns
    ///
    /// A same-domain instant advanced by `duration`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when the elapsed duration
    /// cannot represent the result.
    #[inline]
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
    ///
    /// # Parameters
    ///
    /// * `earlier` - Earlier instant expected to belong to the same domain.
    ///
    /// # Returns
    ///
    /// The elapsed duration from `earlier` to this instant.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign instant.
    /// Returns [`TimeError::InvalidInstantOrder`] when `earlier` is later,
    /// retaining both elapsed durations.
    #[inline]
    pub fn duration_since(self, earlier: Self) -> Result<Duration, TimeError> {
        earlier.ensure_domain(self.domain)?;
        self.elapsed.checked_sub(earlier.elapsed).ok_or(
            TimeError::InvalidInstantOrder {
                current_elapsed: self.elapsed,
                earlier_elapsed: earlier.elapsed,
            },
        )
    }

    /// Verifies that an external instant belongs to expected_domain.
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign instant.
    ///
    /// # Parameters
    ///
    /// * `expected_domain` - Domain the instant must belong to.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the domains match.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] when the domains differ.
    #[inline]
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
    ///
    /// # Parameters
    ///
    /// * `other` - Instant to compare with this one.
    ///
    /// # Returns
    ///
    /// Their elapsed-time ordering for a shared domain, or `None` for distinct
    /// domains.
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.domain == other.domain).then(|| self.elapsed.cmp(&other.elapsed))
    }
}
