// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Exposes the production standard Timer waiter to external Loom models.

use crate::timer::internal::std_timer_waiter::StdTimerWaiter;
use std::task::{
    Context,
    Poll,
    Waker,
};

/// Loom-facing adapter around the production standard Timer waiter.
pub struct LoomStdTimerWaiter {
    /// Production waiter whose mutex operations are modeled by Loom.
    inner: StdTimerWaiter,
}

impl LoomStdTimerWaiter {
    /// Creates an incomplete waiter without a task Waker.
    ///
    /// # Returns
    ///
    /// A model adapter containing the production waiter.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: StdTimerWaiter::new(),
        }
    }

    /// Polls the production waiter using the supplied task context.
    ///
    /// # Parameters
    ///
    /// * `context` - Task context whose Waker observes a terminal transition.
    ///
    /// # Returns
    ///
    /// The production waiter's current pending, ready, or failed state.
    #[inline(always)]
    pub fn poll(&self, context: &Context<'_>) -> Poll<Result<(), ()>> {
        self.inner.poll(context)
    }

    /// Latches completion and detaches the currently registered Waker.
    ///
    /// # Returns
    ///
    /// The detached Waker, or `None` when no Waker was registered or the
    /// waiter was already terminal.
    #[must_use = "the detached Waker must be invoked or safely discarded"]
    #[inline(always)]
    pub fn complete(&self) -> Option<Waker> {
        self.inner.complete()
    }

    /// Latches worker failure and detaches the currently registered Waker.
    ///
    /// # Returns
    ///
    /// The detached Waker, or `None` when no Waker was registered or the
    /// waiter was already terminal.
    #[must_use = "the detached Waker must be invoked or safely discarded"]
    #[inline(always)]
    pub fn fail(&self) -> Option<Waker> {
        self.inner.fail()
    }
}
