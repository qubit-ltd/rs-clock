// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the wall-clock capability.

use std::time::SystemTime;

/// Provides the current civil time as a [`SystemTime`].
///
/// Unlike monotonic time, wall time may move backward after a system clock
/// adjustment and must not be used to measure elapsed durations.
pub trait WallClock: Send + Sync {
    /// Returns the current wall-clock time.
    fn now(&self) -> SystemTime;
}

impl<T> WallClock for std::sync::Arc<T>
where
    T: WallClock + ?Sized,
{
    /// Delegates to the shared wall clock object.
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
    #[inline(always)]
    fn now(&self) -> SystemTime {
        self.as_ref().now()
    }
}
