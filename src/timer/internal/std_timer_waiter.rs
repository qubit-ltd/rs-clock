// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stores completion state for one standard timer registration.

use super::std_timer_waiter_state::StdTimerWaiterState;
#[cfg(all(test, loom))]
use loom::sync::Mutex;
#[cfg(not(all(test, loom)))]
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
    pub(crate) fn new() -> Self {
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
    /// [`Poll::Ready(Ok)`] after deadline completion,
    /// [`Poll::Ready(Err)`] after worker failure, or [`Poll::Pending`] while
    /// the deadline remains active. The unit error is private and only
    /// distinguishes the worker-failure terminal state from deadline
    /// completion.
    pub(crate) fn poll(&self, context: &Context<'_>) -> Poll<Result<(), ()>> {
        let replaced_waker = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match &mut *state {
                StdTimerWaiterState::Pending(waker)
                    if waker.as_ref().is_some_and(|value| {
                        value.will_wake(context.waker())
                    }) =>
                {
                    None
                }
                StdTimerWaiterState::Pending(waker) => {
                    waker.replace(context.waker().clone())
                }
                StdTimerWaiterState::Ready => return Poll::Ready(Ok(())),
                StdTimerWaiterState::WorkerFailed => {
                    return Poll::Ready(Err(()));
                }
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
    /// `None` when no Waker was registered or the waiter was already terminal.
    #[must_use = "the detached Waker must be invoked or safely discarded"]
    pub(crate) fn complete(&self) -> Option<Waker> {
        self.transition_to(StdTimerWaiterState::Ready)
    }

    /// Latches worker failure and detaches the registered task Waker.
    ///
    /// # Returns
    ///
    /// The detached Waker that must be invoked outside scheduler locks, or
    /// `None` when no Waker was registered or the waiter was already terminal.
    #[must_use = "the detached Waker must be invoked or safely discarded"]
    pub(crate) fn fail(&self) -> Option<Waker> {
        self.transition_to(StdTimerWaiterState::WorkerFailed)
    }

    /// Moves a pending waiter to one terminal state and detaches its Waker.
    ///
    /// # Parameters
    ///
    /// * `terminal` - Ready or worker-failed state to latch.
    ///
    /// # Returns
    ///
    /// The previously registered Waker, or `None` when no Waker was registered
    /// or another terminal state had already latched.
    fn transition_to(&self, terminal: StdTimerWaiterState) -> Option<Waker> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match &mut *state {
            StdTimerWaiterState::Pending(waker) => {
                let detached = waker.take();
                *state = terminal;
                detached
            }
            StdTimerWaiterState::Ready | StdTimerWaiterState::WorkerFailed => {
                None
            }
        }
    }
}
