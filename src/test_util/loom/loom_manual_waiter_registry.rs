// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Exposes the production manual waiter registry to external Loom models.

use crate::monotonic::internal::manual_waiter_registry::ManualWaiterRegistry;
use std::{
    task::{
        Context,
        Poll,
        Waker,
    },
    time::Duration,
};

/// Loom-facing adapter around the production manual waiter registry.
pub struct LoomManualWaiterRegistry {
    /// Production registry serialized by the model's Loom mutex.
    inner: ManualWaiterRegistry,
}

impl LoomManualWaiterRegistry {
    /// Creates an empty manual waiter registry.
    ///
    /// # Returns
    ///
    /// A model adapter containing the production registry.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: ManualWaiterRegistry::new(),
        }
    }

    /// Returns the number of currently registered timer waiters.
    ///
    /// # Returns
    ///
    /// The production registry's current timer-waiter count.
    #[must_use]
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.inner.count()
    }

    /// Registers a timer deadline in the production registry.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Elapsed duration at which the waiter becomes ready.
    ///
    /// # Returns
    ///
    /// The allocated waiter identifier.
    #[must_use]
    #[inline(always)]
    pub fn register_timer(&mut self, deadline: Duration) -> u64 {
        self.inner.register_timer(deadline)
    }

    /// Removes one timer registration from the production registry.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Identifier returned by [`Self::register_timer`].
    ///
    /// # Returns
    ///
    /// `Some` containing the optional detached Waker when the waiter existed,
    /// or `None` when it had already been removed.
    #[inline(always)]
    pub fn unregister_timer(
        &mut self,
        waiter_id: u64,
    ) -> Option<Option<Waker>> {
        self.inner.unregister_timer(waiter_id)
    }

    /// Registers an observer for the next future deadline.
    ///
    /// # Returns
    ///
    /// The allocated observer identifier.
    #[must_use]
    #[inline(always)]
    pub fn register_deadline_observer(&mut self) -> u64 {
        self.inner.register_deadline_observer()
    }

    /// Removes one observer from the production registry.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Observer identifier to remove.
    ///
    /// # Returns
    ///
    /// The detached Waker when one was registered, or `None` otherwise.
    #[inline(always)]
    pub fn unregister_observer(&mut self, observer_id: u64) -> Option<Waker> {
        self.inner.unregister_observer(observer_id)
    }

    /// Polls one registered timer waiter.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Timer waiter identifier to poll.
    /// * `elapsed` - Current elapsed manual time.
    /// * `context` - Task context whose Waker is registered while pending.
    ///
    /// # Returns
    ///
    /// The waiter state and any replaced or removed Waker.
    ///
    /// # Panics
    ///
    /// Panics when `waiter_id` is not registered.
    #[must_use = "the poll state and detached waker must both be handled"]
    #[inline(always)]
    pub fn poll_timer(
        &mut self,
        waiter_id: u64,
        elapsed: Duration,
        context: &Context<'_>,
    ) -> (Poll<()>, Option<Waker>) {
        self.inner.poll_timer(waiter_id, elapsed, context)
    }

    /// Polls one observer for the next future deadline.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Deadline observer identifier to poll.
    /// * `elapsed` - Current elapsed manual time.
    /// * `context` - Task context whose Waker is registered while pending.
    ///
    /// # Returns
    ///
    /// The observer state and any replaced or removed Waker.
    ///
    /// # Panics
    ///
    /// Panics when `observer_id` is not registered.
    #[must_use = "the poll state and detached waker must both be handled"]
    #[inline(always)]
    pub fn poll_deadline_observer(
        &mut self,
        observer_id: u64,
        elapsed: Duration,
        context: &Context<'_>,
    ) -> (Poll<Duration>, Option<Waker>) {
        self.inner
            .poll_deadline_observer(observer_id, elapsed, context)
    }

    /// Takes Wakers belonging to timer waiters due at `elapsed`.
    ///
    /// # Parameters
    ///
    /// * `elapsed` - Current elapsed manual time.
    ///
    /// # Returns
    ///
    /// Wakers detached from all due timer waiters.
    #[inline(always)]
    pub fn take_due_timer_wakers(&mut self, elapsed: Duration) -> Vec<Waker> {
        self.inner.take_due_timer_wakers(elapsed)
    }

    /// Takes Wakers whose waiter-count or deadline condition is satisfied.
    ///
    /// # Parameters
    ///
    /// * `elapsed` - Current elapsed manual time.
    ///
    /// # Returns
    ///
    /// Wakers detached from all satisfied observers.
    #[inline(always)]
    pub fn reached_observer_wakers(&mut self, elapsed: Duration) -> Vec<Waker> {
        self.inner.reached_observer_wakers(elapsed)
    }
}
