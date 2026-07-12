// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the monotonic clock capability.

use crate::MonotonicInstant;
use std::time::Duration;

/// Provides the current instant in a stable, non-decreasing clock domain.
pub trait MonotonicClock: Send + Sync {
    /// Returns the stable identifier of this clock's monotonic domain.
    fn domain_id(&self) -> u64;

    /// Returns elapsed time from this clock's private origin.
    fn elapsed_since_origin(&self) -> Duration;

    /// Returns the current instant in this clock's domain.
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain_id(), self.elapsed_since_origin())
    }
}

impl<T> MonotonicClock for std::sync::Arc<T>
where
    T: MonotonicClock + ?Sized,
{
    /// Delegates domain identity to the shared clock object.
    fn domain_id(&self) -> u64 {
        self.as_ref().domain_id()
    }

    /// Delegates elapsed time to the shared clock object.
    fn elapsed_since_origin(&self) -> Duration {
        self.as_ref().elapsed_since_origin()
    }
}

impl<T> MonotonicClock for Box<T>
where
    T: MonotonicClock + ?Sized,
{
    /// Delegates domain identity to the boxed clock object.
    fn domain_id(&self) -> u64 {
        self.as_ref().domain_id()
    }

    /// Delegates elapsed time to the boxed clock object.
    fn elapsed_since_origin(&self) -> Duration {
        self.as_ref().elapsed_since_origin()
    }
}
