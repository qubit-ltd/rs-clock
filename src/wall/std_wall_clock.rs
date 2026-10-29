// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the standard system wall clock.

use std::time::SystemTime;

use crate::WallClock;

/// A zero-sized wall clock backed by [`SystemTime::now`].
///
/// # Examples
///
/// ```
/// use qubit_clock::{StdWallClock, WallClock};
///
/// let clock = StdWallClock::new();
/// let now = clock.now();
/// assert!(now.duration_since(std::time::SystemTime::UNIX_EPOCH).is_ok());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct StdWallClock;

impl StdWallClock {
    /// Creates a standard system wall clock.
    ///
    /// # Returns
    ///
    /// A zero-sized wall clock backed by [`SystemTime::now`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl WallClock for StdWallClock {
    /// Returns the current system wall time.
    ///
    /// # Returns
    ///
    /// The value produced by [`SystemTime::now`].
    #[inline]
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}
