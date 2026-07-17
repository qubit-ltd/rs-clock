// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stores completion state for one standard timer registration.

use super::std_timer_waiter_state::StdTimerWaiterState;
use std::sync::Mutex;
use std::task::{
    Context,
    Poll,
    Waker,
};

/// Completion latch and task waker shared by a future and scheduler worker.
pub(crate) struct StdTimerWaiter {
    /// Completion state serialized across polling and scheduling threads.
    state: Mutex<StdTimerWaiterState>,
}

impl StdTimerWaiter {
    /// Creates an incomplete waiter without a task waker.
    ///
    /// # Returns
    ///
    /// A waiter ready to be registered with a scheduler.
    #[must_use]
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new(StdTimerWaiterState::new()),
        }
    }

    /// Polls the completion latch and stores a different task waker if needed.
    ///
    /// # Parameters
    ///
    /// * `context` - Task context whose waker observes completion.
    ///
    /// # Returns
    ///
    /// [`Poll::Ready`] after completion has latched, otherwise
    /// [`Poll::Pending`].
    pub(crate) fn poll(&self, context: &Context<'_>) -> Poll<()> {
        let replaced_waker = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.ready {
                return Poll::Ready(());
            }
            if state
                .waker
                .as_ref()
                .is_some_and(|value| value.will_wake(context.waker()))
            {
                None
            } else {
                state.waker.replace(context.waker().clone())
            }
        };
        drop(replaced_waker);
        Poll::Pending
    }

    /// Latches completion and detaches the currently registered task Waker.
    ///
    /// # Returns
    ///
    /// The detached Waker that must be invoked outside scheduler locks, or
    /// `None` when this future has not yet registered one.
    #[must_use = "the detached Waker must be invoked or safely discarded"]
    pub(crate) fn complete(&self) -> Option<Waker> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.ready = true;
        state.waker.take()
    }
}
