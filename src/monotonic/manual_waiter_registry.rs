// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores deadline waiters and waiter-count observers for a manual clock.

use std::collections::HashMap;
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;

/// Waiters registered against one manual monotonic timeline.
pub(crate) struct ManualWaiterRegistry {
    /// Next identifier assigned to a blocking waiter.
    next_blocking_waiter_id: u64,
    /// Blocking waiter deadlines keyed by registration identifier.
    blocking_waiters: HashMap<u64, Duration>,
    /// Next identifier assigned to an async waiter.
    next_async_waiter_id: u64,
    /// Async deadlines and optional task wakers keyed by registration ID.
    async_waiters: HashMap<u64, (Duration, Option<Waker>)>,
    /// Next identifier assigned to a waiter-count observer.
    next_observer_id: u64,
    /// Expected waiter counts and optional task wakers keyed by registration
    /// ID.
    observers: HashMap<u64, (usize, Option<Waker>)>,
}

impl ManualWaiterRegistry {
    /// Creates an empty registry.
    pub(crate) fn new() -> Self {
        Self {
            next_blocking_waiter_id: 1,
            blocking_waiters: HashMap::new(),
            next_async_waiter_id: 1,
            async_waiters: HashMap::new(),
            next_observer_id: 1,
            observers: HashMap::new(),
        }
    }

    /// Registers a blocking deadline and returns its registration identifier.
    ///
    /// Panics when the registry cannot allocate another identifier.
    pub(crate) fn register_blocking(&mut self, deadline: Duration) -> u64 {
        let waiter_id = Self::allocate_identifier(
            &mut self.next_blocking_waiter_id,
            "manual blocking waiter identifiers exhausted",
        );
        self.blocking_waiters.insert(waiter_id, deadline);
        waiter_id
    }

    /// Removes the blocking waiter identified by waiter_id.
    pub(crate) fn unregister_blocking(&mut self, waiter_id: u64) {
        self.blocking_waiters.remove(&waiter_id);
    }

    /// Registers an async deadline and returns its registration identifier.
    ///
    /// Panics when the registry cannot allocate another identifier.
    pub(crate) fn register_async(&mut self, deadline: Duration) -> u64 {
        let waiter_id = Self::allocate_identifier(
            &mut self.next_async_waiter_id,
            "manual async waiter identifiers exhausted",
        );
        self.async_waiters.insert(waiter_id, (deadline, None));
        waiter_id
    }

    /// Removes an async waiter and returns whether a registration existed.
    pub(crate) fn unregister_async(&mut self, waiter_id: u64) -> bool {
        self.async_waiters.remove(&waiter_id).is_some()
    }

    /// Returns the earliest deadline strictly after elapsed.
    pub(crate) fn next_future_deadline(
        &self,
        elapsed: Duration,
    ) -> Option<Duration> {
        self.blocking_waiters
            .values()
            .chain(self.async_waiters.values().map(|(deadline, _)| deadline))
            .filter(|deadline| **deadline > elapsed)
            .min()
            .copied()
    }

    /// Collects cloned wakers for async deadlines reached by elapsed.
    pub(crate) fn due_async_wakers(&self, elapsed: Duration) -> Vec<Waker> {
        self.async_waiters
            .values()
            .filter(|(deadline, _)| *deadline <= elapsed)
            .filter_map(|(_, waker)| waker.clone())
            .collect()
    }

    /// Registers a count observer when count has not already been reached.
    ///
    /// Returns None when the requested count is already satisfied.
    ///
    /// Panics when the registry cannot allocate another identifier.
    pub(crate) fn register_observer(
        &mut self,
        expected_count: usize,
        count: usize,
    ) -> Option<u64> {
        if count >= expected_count {
            return None;
        }
        let observer_id = Self::allocate_identifier(
            &mut self.next_observer_id,
            "manual waiter observer identifiers exhausted",
        );
        self.observers.insert(observer_id, (expected_count, None));
        Some(observer_id)
    }

    /// Polls an observer and records the task waker while it remains pending.
    ///
    /// A missing observer is ready because reaching the count removes and
    /// latches the observer before waking its task.
    pub(crate) fn poll_observer(
        &mut self,
        observer_id: u64,
        count: usize,
        context: &Context<'_>,
    ) -> Poll<()> {
        let Some((expected_count, _)) = self.observers.get(&observer_id) else {
            return Poll::Ready(());
        };
        if count >= *expected_count {
            self.observers.remove(&observer_id);
            return Poll::Ready(());
        }
        if let Some((_, registered_waker)) =
            self.observers.get_mut(&observer_id)
            && registered_waker
                .as_ref()
                .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            *registered_waker = Some(context.waker().clone());
        }
        Poll::Pending
    }

    /// Removes an incomplete waiter-count observer.
    pub(crate) fn unregister_observer(&mut self, observer_id: u64) {
        self.observers.remove(&observer_id);
    }

    /// Updates the async waiter waker or reports that its deadline is due.
    ///
    /// The returned ready state removes the waiter registration.
    pub(crate) fn poll_async(
        &mut self,
        waiter_id: u64,
        deadline: Duration,
        elapsed: Duration,
        context: &Context<'_>,
    ) -> Poll<()> {
        if elapsed >= deadline {
            self.async_waiters.remove(&waiter_id);
            return Poll::Ready(());
        }
        if let Some((_, registered_waker)) =
            self.async_waiters.get_mut(&waiter_id)
            && registered_waker
                .as_ref()
                .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            *registered_waker = Some(context.waker().clone());
        }
        Poll::Pending
    }

    /// Removes reached observers and returns their registered task wakers.
    pub(crate) fn reached_observer_wakers(&mut self) -> Vec<Waker> {
        let count = self.count();
        let mut wakers = Vec::new();
        self.observers.retain(|_, (expected_count, waker)| {
            if *expected_count <= count {
                if let Some(waker) = waker.take() {
                    wakers.push(waker);
                }
                false
            } else {
                true
            }
        });
        wakers
    }

    /// Returns the number of registered deadline waiters.
    pub(crate) fn count(&self) -> usize {
        self.blocking_waiters.len() + self.async_waiters.len()
    }

    /// Allocates the next monotonic registry identifier.
    ///
    /// Panics with exhausted_message when the identifier space is exhausted.
    fn allocate_identifier(
        next_identifier: &mut u64,
        exhausted_message: &str,
    ) -> u64 {
        let identifier = *next_identifier;
        *next_identifier =
            next_identifier.checked_add(1).expect(exhausted_message);
        identifier
    }
}
