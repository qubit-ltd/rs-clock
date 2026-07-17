// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stores the completion state protected by one standard Timer waiter lock.

use std::task::Waker;

/// Mutable completion state for one standard Timer waiter.
pub(super) struct StdTimerWaiterState {
    /// Whether the scheduler has reached this waiter's deadline.
    pub(super) ready: bool,
    /// Most recently registered task Waker.
    pub(super) waker: Option<Waker>,
}

impl StdTimerWaiterState {
    /// Creates incomplete state without a registered Waker.
    ///
    /// # Returns
    ///
    /// Initial state for a newly allocated waiter.
    #[must_use]
    #[inline(always)]
    pub(super) const fn new() -> Self {
        Self {
            ready: false,
            waker: None,
        }
    }
}
