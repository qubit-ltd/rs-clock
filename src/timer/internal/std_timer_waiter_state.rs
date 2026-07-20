// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stores the terminal state protected by one standard Timer waiter lock.

use std::task::Waker;

/// Mutable terminal state for one standard Timer waiter.
pub(super) enum StdTimerWaiterState {
    /// Deadline is pending, optionally with the most recently registered Waker.
    Pending(Option<Waker>),
    /// Deadline completion has latched.
    Ready,
    /// The owning scheduler worker exited before deadline completion.
    WorkerFailed,
}

impl StdTimerWaiterState {
    /// Creates pending state without a registered Waker.
    ///
    /// # Returns
    ///
    /// Initial state for a newly allocated waiter.
    #[must_use]
    #[inline(always)]
    pub(super) const fn new() -> Self {
        Self::Pending(None)
    }
}
