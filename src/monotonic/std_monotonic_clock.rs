// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the standard-library monotonic clock implementation.

use crate::{
    ClockDomain,
    MonotonicClock,
    MonotonicInstant,
};
use std::time::Instant;

/// A real monotonic clock backed by [`std::time::Instant`].
///
/// The type intentionally does not implement [`Clone`]. Shared identity is
/// expressed explicitly with `Arc<StdMonotonicClock>`.
#[derive(Debug)]
pub struct StdMonotonicClock {
    /// Domain carried by instants sampled from this clock.
    domain: ClockDomain,
    /// Native standard-library instant mapped to elapsed duration zero.
    origin: Instant,
}

impl StdMonotonicClock {
    /// Creates a new independent clock domain at the current native instant.
    ///
    /// # Returns
    ///
    /// A standard monotonic clock with a newly allocated domain.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            domain: ClockDomain::new(),
            origin: Instant::now(),
        }
    }

    /// Returns the native origin used by the paired blocking sleeper.
    ///
    /// # Returns
    ///
    /// The standard-library instant mapped to elapsed duration zero.
    #[must_use]
    #[inline(always)]
    pub(crate) const fn origin(&self) -> Instant {
        self.origin
    }

    /// Returns this concrete clock's domain without sampling native time.
    ///
    /// # Returns
    ///
    /// This clock's process-unique domain.
    #[inline(always)]
    pub(crate) const fn domain(&self) -> ClockDomain {
        self.domain
    }
}

impl Default for StdMonotonicClock {
    /// Creates a new independent standard monotonic clock domain.
    ///
    /// # Returns
    ///
    /// A standard monotonic clock with a newly allocated domain.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for StdMonotonicClock {
    /// Returns the current instant in this clock's domain.
    ///
    /// # Returns
    ///
    /// The current elapsed duration represented in this clock's domain.
    #[inline]
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.origin.elapsed())
    }
}
