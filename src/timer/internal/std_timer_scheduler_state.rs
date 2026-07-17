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
use std::collections::{
    BTreeSet,
    HashMap,
};
use std::sync::Arc;
use std::time::Instant;

/// Mutable registrations protected by one scheduler lock.
pub(super) struct StdTimerSchedulerState {
    /// Next nonzero registration identifier.
    next_waiter_id: u64,
    /// Active deadline keys ordered from earliest to latest.
    deadlines: BTreeSet<(Instant, u64)>,
    /// Active registrations keyed by registration identifier.
    registrations: HashMap<u64, StdTimerRegistration>,
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
            deadlines: BTreeSet::new(),
            registrations: HashMap::new(),
            worker_running: false,
            worker_generation: 0,
        }
    }

    /// Registers `waiter` at `deadline` in both exact indexes.
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
        let deadline_inserted = self.deadlines.insert((deadline, waiter_id));
        let previous = self
            .registrations
            .insert(waiter_id, StdTimerRegistration::new(deadline, waiter));
        assert!(
            deadline_inserted && previous.is_none(),
            "standard Timer registration indexes must remain one-to-one",
        );
        waiter_id
    }

    /// Cancels one active registration in both exact indexes.
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
    /// Panics when the registration index contains an entry without its exact
    /// deadline key.
    pub(super) fn cancel(
        &mut self,
        waiter_id: u64,
    ) -> Option<Arc<StdTimerWaiter>> {
        let registration = self.registrations.remove(&waiter_id)?;
        let removed =
            self.deadlines.remove(&(registration.deadline(), waiter_id));
        assert!(
            removed,
            "active standard Timer registration must have a deadline key",
        );
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
    /// Panics when a deadline key does not have its matching registration.
    pub(super) fn take_due(
        &mut self,
        now: Instant,
    ) -> Vec<Arc<StdTimerWaiter>> {
        let mut due_waiters = Vec::new();
        while let Some((deadline, waiter_id)) = self.deadlines.first().copied()
        {
            if deadline > now {
                break;
            }
            self.deadlines.pop_first();
            let registration = self
                .registrations
                .remove(&waiter_id)
                .expect("standard Timer deadline must have a registration");
            debug_assert_eq!(deadline, registration.deadline());
            due_waiters.push(registration.into_waiter());
        }
        due_waiters
    }

    /// Returns the earliest active deadline.
    ///
    /// # Returns
    ///
    /// The earliest deadline, or `None` when there are no active registrations.
    #[must_use]
    #[inline(always)]
    pub(super) fn next_deadline(&self) -> Option<Instant> {
        self.deadlines.first().map(|(deadline, _)| *deadline)
    }

    /// Reports whether no active registrations remain.
    ///
    /// # Returns
    ///
    /// `true` when both exact indexes are empty.
    ///
    /// # Panics
    ///
    /// Panics in debug builds when the two indexes disagree about emptiness.
    #[must_use]
    #[inline(always)]
    pub(super) fn is_empty(&self) -> bool {
        debug_assert_eq!(
            self.deadlines.is_empty(),
            self.registrations.is_empty()
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
    #[must_use]
    #[inline(always)]
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
    #[inline(always)]
    pub(super) fn mark_worker_stopped(&mut self, generation: u64) {
        if self.worker_generation == generation {
            self.worker_running = false;
        }
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
    #[inline(always)]
    fn allocate_waiter_id(&mut self) -> u64 {
        let waiter_id = self.next_waiter_id;
        assert_ne!(waiter_id, 0, "standard Timer waiter identifiers exhausted");
        self.next_waiter_id = waiter_id.wrapping_add(1);
        waiter_id
    }
}
