// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the standard-library monotonic clock implementation.

use crate::{
    MonotonicClock,
    allocate_clock_domain_id,
};
use std::time::Duration;
use std::time::Instant;

/// A real monotonic clock backed by [`std::time::Instant`].
///
/// The type intentionally does not implement [`Clone`]. Shared identity is
/// expressed explicitly with `Arc<StdMonotonicClock>`.
#[derive(Debug)]
pub struct StdMonotonicClock {
    domain_id: u64,
    origin: Instant,
}

impl StdMonotonicClock {
    /// Creates a new independent clock domain at the current native instant.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domain_id: allocate_clock_domain_id(),
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
    /// Returns this clock's stable domain identifier.
    fn domain_id(&self) -> u64 {
        self.domain_id
    }

    /// Returns elapsed real time from this clock's native origin.
    fn elapsed_since_origin(&self) -> Duration {
        self.origin.elapsed()
    }
}
