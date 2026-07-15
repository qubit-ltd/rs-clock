// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines an immutable fixed wall clock.

use crate::WallClock;
use std::time::SystemTime;

/// A wall clock that always returns one fixed time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedWallClock {
    /// Immutable wall-clock value returned by every sample.
    fixed_time: SystemTime,
}

impl FixedWallClock {
    /// Creates a clock that always returns `fixed_time`.
    #[must_use]
    #[inline(always)]
    pub const fn new(fixed_time: SystemTime) -> Self {
        Self { fixed_time }
    }

    /// Returns the immutable time held by this clock.
    #[must_use]
    #[inline(always)]
    pub const fn fixed_time(&self) -> SystemTime {
        self.fixed_time
    }
}

impl WallClock for FixedWallClock {
    /// Returns the configured fixed wall time.
    #[inline(always)]
    fn now(&self) -> SystemTime {
        self.fixed_time
    }
}
