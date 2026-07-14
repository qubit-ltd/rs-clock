// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores mutable state for a manual monotonic clock.

use crate::monotonic::manual_advance_registry::ManualAdvanceRegistry;
use crate::monotonic::manual_waiter_registry::ManualWaiterRegistry;
use std::time::Duration;

/// Mutable time and waiter registrations protected by the owning clock.
pub(crate) struct ManualMonotonicState {
    /// Current logical duration from the manual clock origin.
    pub(crate) elapsed: Duration,
    /// Deadline waiters and waiter-count observers.
    pub(crate) waiters: ManualWaiterRegistry,
    /// Callbacks observing successful advances.
    pub(crate) advances: ManualAdvanceRegistry,
}

impl ManualMonotonicState {
    /// Creates state at the clock domain origin.
    pub(crate) fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            waiters: ManualWaiterRegistry::new(),
            advances: ManualAdvanceRegistry::new(),
        }
    }

    /// Returns the number of blocking and asynchronous deadline waiters.
    pub(crate) fn waiter_count(&self) -> usize {
        self.waiters.count()
    }
}
