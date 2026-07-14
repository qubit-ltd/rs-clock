// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the monotonic clock capability.

use crate::MonotonicInstant;

/// Provides the current instant in a stable, non-decreasing clock domain.
pub trait MonotonicClock: Send + Sync {
    /// Returns the current instant in this clock's domain.
    fn now(&self) -> MonotonicInstant;
}

impl<T> MonotonicClock for std::sync::Arc<T>
where
    T: MonotonicClock + ?Sized,
{
    /// Delegates the current instant to the shared clock object.
    fn now(&self) -> MonotonicInstant {
        self.as_ref().now()
    }
}

impl<T> MonotonicClock for Box<T>
where
    T: MonotonicClock + ?Sized,
{
    /// Delegates the current instant to the boxed clock object.
    fn now(&self) -> MonotonicInstant {
        self.as_ref().now()
    }
}
