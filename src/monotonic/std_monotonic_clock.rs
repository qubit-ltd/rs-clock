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
    domain: ClockDomain,
    origin: Instant,
}

impl StdMonotonicClock {
    /// Creates a new independent clock domain at the current native instant.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domain: ClockDomain::new(),
            origin: Instant::now(),
        }
    }

    /// Returns the native origin used by the paired blocking sleeper.
    pub(crate) const fn origin(&self) -> Instant {
        self.origin
    }
}

impl Default for StdMonotonicClock {
    /// Creates a new independent standard monotonic clock domain.
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for StdMonotonicClock {
    /// Returns the current instant in this clock's domain.
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.origin.elapsed())
    }
}
