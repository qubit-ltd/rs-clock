// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the standard system wall clock.

use crate::WallClock;
use std::time::SystemTime;

/// A zero-sized wall clock backed by [`SystemTime::now`].
#[derive(Debug, Clone, Copy, Default)]
pub struct StdWallClock;

impl StdWallClock {
    /// Creates a standard system wall clock.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl WallClock for StdWallClock {
    /// Returns the current system wall time.
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}
