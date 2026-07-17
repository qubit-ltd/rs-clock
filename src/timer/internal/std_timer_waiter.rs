// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores completion state for one standard timer registration.

use std::sync::Mutex;
use std::task::{
    Context,
    Poll,
    Waker,
};

/// Completion state protected by one waiter lock.
struct StdTimerWaiterState {
    /// Whether the scheduler has reached this waiter's deadline.
    ready: bool,
    /// Most recently registered task waker.
    waker: Option<Waker>,
}

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
            state: Mutex::new(StdTimerWaiterState {
                ready: false,
                waker: None,
            }),
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

    /// Latches completion and wakes the currently registered task.
    pub(crate) fn complete(&self) {
        let waker = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.ready = true;
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
