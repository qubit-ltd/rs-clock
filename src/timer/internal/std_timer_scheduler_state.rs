// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Maintains exact deadline and registration indexes for a standard Timer.

use super::std_timer_registration::StdTimerRegistration;
use super::std_timer_waiter::StdTimerWaiter;
use qubit_collections::OrderedIndexMap;
use std::sync::Arc;
use std::time::Instant;

/// Mutable registrations protected by one scheduler lock.
pub(super) struct StdTimerSchedulerState {
    /// Next nonzero registration identifier.
    next_waiter_id: u64,
    /// Registrations keyed by ID and ordered by active deadline.
    registrations: OrderedIndexMap<u64, Instant, StdTimerRegistration>,
    /// Whether a scheduler worker is currently running.
    worker_running: bool,
    /// Generation identifying the most recently started worker.
    worker_generation: u64,
}

impl StdTimerSchedulerState {
    /// Creates empty scheduler state.
    ///
    /// # Returns
    ///
    /// State without registrations or a worker thread.
    #[must_use]
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            next_waiter_id: 1,
            registrations: OrderedIndexMap::new(),
            worker_running: false,
            worker_generation: 0,
        }
    }

    /// Registers `waiter` at `deadline` in the indexed collection.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Native standard-library deadline.
    /// * `waiter` - Completion latch shared with the returned future.
    ///
    /// # Returns
    ///
    /// The active nonzero registration identifier.
    ///
    /// # Panics
    ///
    /// Panics after all nonzero identifiers have been allocated or when an
    /// internal index invariant is violated.
    #[must_use = "the registration identifier is required for cancellation"]
    pub(super) fn register(
        &mut self,
        deadline: Instant,
        waiter: Arc<StdTimerWaiter>,
    ) -> u64 {
        let waiter_id = self.allocate_waiter_id();
        let inserted = self.registrations.try_insert(
            waiter_id,
            deadline,
            StdTimerRegistration::new(deadline, waiter),
        );
        assert!(
            inserted.is_ok(),
            "standard Timer waiter identifier must be unique",
        );
        waiter_id
    }

    /// Cancels one active registration by identifier.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Registration identifier to remove.
    ///
    /// # Returns
    ///
    /// The removed waiter, or `None` when completion or cancellation already
    /// removed the registration.
    ///
    /// # Panics
    ///
    /// Panics when the indexed collection invariants are violated.
    pub(super) fn cancel(
        &mut self,
        waiter_id: u64,
    ) -> Option<Arc<StdTimerWaiter>> {
        let entry = self.registrations.remove(&waiter_id)?;
        debug_assert_eq!(*entry.order(), entry.value().deadline());
        let registration = entry.into_value();
        Some(registration.into_waiter())
    }

    /// Removes all registrations whose deadline is at or before `now`.
    ///
    /// # Parameters
    ///
    /// * `now` - Native time used to select the completed prefix.
    ///
    /// # Returns
    ///
    /// Completion waiters in deadline and registration order.
    ///
    /// # Panics
    ///
    /// Panics when the indexed collection invariants are violated.
    pub(super) fn take_due(
        &mut self,
        now: Instant,
    ) -> Vec<Arc<StdTimerWaiter>> {
        self.registrations
            .extract_range(..=now)
            .map(|entry| {
                debug_assert_eq!(*entry.order(), entry.value().deadline());
                entry.into_value().into_waiter()
            })
            .collect()
    }

    /// Returns the earliest active deadline.
    ///
    /// # Returns
    ///
    /// The earliest deadline, or `None` when there are no active registrations.
    #[must_use]
    #[inline(always)]
    pub(super) fn next_deadline(&self) -> Option<Instant> {
        self.registrations.first().map(|entry| *entry.order())
    }

    /// Reports whether no active registrations remain.
    ///
    /// # Returns
    ///
    /// `true` when no active registration remains.
    ///
    /// # Panics
    ///
    /// Panics in debug builds when an active registration has been unindexed.
    #[must_use]
    #[inline(always)]
    pub(super) fn is_empty(&self) -> bool {
        debug_assert_eq!(
            self.registrations.len(),
            self.registrations.attached_len()
        );
        self.registrations.is_empty()
    }

    /// Reports whether a scheduler worker is running.
    ///
    /// # Returns
    ///
    /// The current worker-running state.
    #[must_use]
    #[inline(always)]
    pub(super) const fn worker_running(&self) -> bool {
        self.worker_running
    }

    /// Marks a new scheduler worker as running.
    ///
    /// # Returns
    ///
    /// The nonzero generation assigned to the new worker.
    ///
    /// # Panics
    ///
    /// Panics after every nonzero worker generation has been allocated.
    #[must_use]
    #[inline]
    pub(super) fn mark_worker_started(&mut self) -> u64 {
        self.worker_generation = self.worker_generation.wrapping_add(1);
        assert_ne!(
            self.worker_generation, 0,
            "standard Timer worker generations exhausted",
        );
        self.worker_running = true;
        self.worker_generation
    }

    /// Marks the matching scheduler worker as stopped.
    ///
    /// # Parameters
    ///
    /// * `generation` - Generation of the worker that has exited, or zero for a
    ///   disarmed guard.
    #[inline]
    pub(super) fn mark_worker_stopped(&mut self, generation: u64) {
        if self.worker_generation == generation {
            self.worker_running = false;
        }
    }

    /// Stops a matching worker and removes every registration it owned.
    ///
    /// The running flag and indexed registrations change under the same
    /// scheduler lock. A zero or stale generation leaves the active generation
    /// untouched.
    ///
    /// # Parameters
    ///
    /// * `generation` - Generation of the worker that exited.
    ///
    /// # Returns
    ///
    /// Waiters removed from the exited generation, to be failed outside the
    /// scheduler lock. Returns an empty collection for zero or stale
    /// generations.
    #[must_use]
    pub(super) fn stop_worker_and_take_waiters(
        &mut self,
        generation: u64,
    ) -> Vec<Arc<StdTimerWaiter>> {
        if generation == 0 || self.worker_generation != generation {
            return Vec::new();
        }
        self.worker_running = false;
        let mut waiters = Vec::with_capacity(self.registrations.len());
        while let Some(entry) = self.registrations.pop_first() {
            waiters.push(entry.into_value().into_waiter());
        }
        waiters
    }

    /// Allocates a nonzero waiter identifier without reuse after exhaustion.
    ///
    /// # Returns
    ///
    /// The next registration identifier.
    ///
    /// # Panics
    ///
    /// Panics after all nonzero identifiers have been allocated.
    #[must_use]
    #[inline]
    fn allocate_waiter_id(&mut self) -> u64 {
        let waiter_id = self.next_waiter_id;
        assert_ne!(waiter_id, 0, "standard Timer waiter identifiers exhausted");
        self.next_waiter_id = waiter_id.wrapping_add(1);
        waiter_id
    }
}
