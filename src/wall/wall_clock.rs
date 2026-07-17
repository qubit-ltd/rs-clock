// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the wall-clock capability.

use std::time::SystemTime;

/// Provides the current civil time as a [`SystemTime`].
///
/// Unlike monotonic time, wall time may move backward after a system clock
/// adjustment and must not be used to measure elapsed durations.
///
/// Discarding a sampled wall time is rejected when `unused_must_use` is denied:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use qubit_clock::{StdWallClock, WallClock};
///
/// StdWallClock::new().now();
/// ```
pub trait WallClock: Send + Sync {
    /// Returns the current wall-clock time.
    ///
    /// # Returns
    ///
    /// The implementor's current civil-time reading.
    #[must_use = "the sampled wall-clock time should be used"]
    fn now(&self) -> SystemTime;
}

impl<T> WallClock for std::sync::Arc<T>
where
    T: WallClock + ?Sized,
{
    /// Delegates to the shared wall clock object.
    ///
    /// # Returns
    ///
    /// The current wall time returned by the wrapped clock.
    #[inline(always)]
    fn now(&self) -> SystemTime {
        self.as_ref().now()
    }
}

impl<T> WallClock for Box<T>
where
    T: WallClock + ?Sized,
{
    /// Delegates to the boxed wall clock object.
    ///
    /// # Returns
    ///
    /// The current wall time returned by the wrapped clock.
    #[inline(always)]
    fn now(&self) -> SystemTime {
        self.as_ref().now()
    }
}
