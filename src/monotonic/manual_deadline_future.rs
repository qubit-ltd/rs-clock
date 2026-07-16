// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a future that observes the next manual deadline registration.

use crate::{
    ManualMonotonicClock,
    MonotonicInstant,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
};

/// A future that completes with a manual clock's next future deadline.
///
/// The observer is registered when the future is created, before its first
/// poll. It latches the earliest deadline that is strictly later than the
/// clock reading at observation time. Dropping an incomplete future
/// unregisters the observer from the clock.
#[derive(Debug)]
pub struct ManualDeadlineFuture {
    /// Manual clock whose deadline registrations are observed.
    clock: Arc<ManualMonotonicClock>,
    /// Identifier of the registered deadline observer.
    observer_id: Option<u64>,
}

impl ManualDeadlineFuture {
    /// Creates a deadline observer for `clock`.
    ///
    /// # Parameters
    ///
    /// * `clock` - Manual clock whose waiter deadlines are observed.
    ///
    /// # Returns
    ///
    /// A future whose observer is registered before this method returns.
    ///
    /// # Panics
    ///
    /// Panics when the observer identifier space is exhausted.
    #[must_use]
    #[inline]
    pub(crate) fn new(clock: Arc<ManualMonotonicClock>) -> Self {
        let observer_id = clock.register_deadline_observer();
        Self {
            clock,
            observer_id: Some(observer_id),
        }
    }
}

impl Future for ManualDeadlineFuture {
    type Output = MonotonicInstant;

    /// Polls for the first future deadline observed after registration.
    ///
    /// # Parameters
    ///
    /// * `context` - Task context whose waker replaces any prior registration.
    ///
    /// # Returns
    ///
    /// [`Poll::Ready`] with the latched deadline, otherwise [`Poll::Pending`].
    ///
    /// # Panics
    ///
    /// Panics if the observer registration is unexpectedly missing or if
    /// destroying a replaced custom task waker panics.
    fn poll(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let Some(observer_id) = self.observer_id else {
            panic!("manual deadline future polled after completion");
        };
        let result = self.clock.poll_deadline_observer(observer_id, context);
        if result.is_ready() {
            self.observer_id = None;
        }
        result
    }
}

impl Drop for ManualDeadlineFuture {
    /// Unregisters an incomplete deadline observer.
    ///
    /// # Panics
    ///
    /// Panics if destroying the observer's custom task waker panics.
    #[inline]
    fn drop(&mut self) {
        if let Some(observer_id) = self.observer_id.take() {
            self.clock.unregister_waiter_observer(observer_id);
        }
    }
}
