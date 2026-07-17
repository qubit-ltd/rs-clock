// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores the shared mutable state of one manual monotonic time domain.

use super::ManualMonotonicState;
use std::sync::{
    Condvar,
    Mutex,
};

/// Shared synchronization state retained by same-domain manual clock handles.
pub(crate) struct ManualTimeDomain {
    /// Mutable logical time, waiter registrations, and advance observers.
    pub(crate) state: Mutex<ManualMonotonicState>,
    /// Condition variable notifying coordination helpers of waiter changes.
    pub(crate) waiters_changed: Condvar,
}

impl ManualTimeDomain {
    /// Creates an empty time domain at elapsed duration zero.
    ///
    /// # Returns
    ///
    /// Shared-state storage for a newly allocated manual clock domain.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(ManualMonotonicState::new()),
            waiters_changed: Condvar::new(),
        }
    }
}
