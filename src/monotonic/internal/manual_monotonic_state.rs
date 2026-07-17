// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stores mutable state for a manual monotonic clock.

use super::manual_waiter_registry::ManualWaiterRegistry;
use std::time::Duration;

/// Mutable time and waiter registrations protected by the owning clock.
pub(crate) struct ManualMonotonicState {
    /// Current logical duration from the manual clock origin.
    pub(crate) elapsed: Duration,
    /// Deadline waiters and waiter-count observers.
    pub(crate) waiters: ManualWaiterRegistry,
}

impl ManualMonotonicState {
    /// Creates state at the clock domain origin.
    ///
    /// # Returns
    ///
    /// Empty manual-clock state at elapsed duration zero.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            waiters: ManualWaiterRegistry::new(),
        }
    }

    /// Returns the number of timer deadline waiters.
    ///
    /// # Returns
    ///
    /// The total number of registered deadline waiters.
    #[must_use]
    #[inline(always)]
    pub(crate) fn waiter_count(&self) -> usize {
        self.waiters.count()
    }
}
