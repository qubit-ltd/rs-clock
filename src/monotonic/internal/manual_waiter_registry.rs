// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores deadline waiters and waiter-count observers for a manual clock.

use crate::monotonic::clock_domain::next_identifier_state;
use std::collections::HashMap;
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;

/// Allocates the current nonzero registry identifier and advances its state.
///
/// The maximum identifier is returned while changing `next_identifier` to the
/// terminal zero state. Later calls panic with `exhausted_message`.
///
/// # Parameters
///
/// * `next_identifier` - Mutable allocator state to read and advance.
/// * `exhausted_message` - Panic message used after identifier exhaustion.
///
/// # Returns
///
/// The current nonzero identifier before advancing the allocator.
///
/// # Panics
///
/// Panics with `exhausted_message` when the allocator is already exhausted.
#[must_use = "the allocated identifier must be retained by its registration"]
#[inline(always)]
pub(crate) fn allocate_identifier(
    next_identifier: &mut u64,
    exhausted_message: &str,
) -> u64 {
    let identifier = *next_identifier;
    *next_identifier =
        next_identifier_state(identifier).expect(exhausted_message);
    identifier
}

/// Waiters registered against one manual monotonic timeline.
pub(crate) struct ManualWaiterRegistry {
    /// Next identifier assigned to a timer waiter.
    next_timer_waiter_id: u64,
    /// Timer deadlines and optional task wakers keyed by registration ID.
    timer_waiters: HashMap<u64, (Duration, Option<Waker>)>,
    /// Next identifier assigned to a waiter-registration observer.
    next_observer_id: u64,
    /// Waiter-count observers keyed by registration identifier.
    count_observers: HashMap<u64, (usize, Option<Waker>)>,
    /// Future-deadline observer wakers keyed by registration identifier.
    deadline_observers: HashMap<u64, Option<Waker>>,
}

impl ManualWaiterRegistry {
    /// Creates an empty registry.
    ///
    /// # Returns
    ///
    /// A registry with no waiters or observers.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            next_timer_waiter_id: 1,
            timer_waiters: HashMap::new(),
            next_observer_id: 1,
            count_observers: HashMap::new(),
            deadline_observers: HashMap::new(),
        }
    }

    /// Registers a timer deadline and returns its registration identifier.
    ///
    /// Panics when the registry cannot allocate another identifier.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Elapsed clock duration at which the waiter becomes ready.
    ///
    /// # Returns
    ///
    /// The nonzero identifier assigned to the timer waiter.
    ///
    /// # Panics
    ///
    /// Panics when the timer-waiter identifier space is exhausted.
    #[must_use = "the waiter identifier is required to poll or cancel the wait"]
    #[inline]
    pub(crate) fn register_timer(&mut self, deadline: Duration) -> u64 {
        let waiter_id = allocate_identifier(
            &mut self.next_timer_waiter_id,
            "manual timer waiter identifiers exhausted",
        );
        self.timer_waiters.insert(waiter_id, (deadline, None));
        waiter_id
    }

    /// Removes a timer waiter and returns its optional registered waker.
    ///
    /// The outer option reports whether the waiter existed. The inner option
    /// contains the waker most recently registered by polling its future.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Identifier of the timer waiter to remove.
    ///
    /// # Returns
    ///
    /// `None` when the waiter was absent, `Some(None)` for an unpolled waiter,
    /// or `Some(Some(waker))` for a waiter with a registered task waker.
    #[inline(always)]
    pub(crate) fn unregister_timer(
        &mut self,
        waiter_id: u64,
    ) -> Option<Option<Waker>> {
        self.timer_waiters
            .remove(&waiter_id)
            .map(|(_, waker)| waker)
    }

    /// Returns the earliest deadline strictly after elapsed.
    ///
    /// # Parameters
    ///
    /// * `elapsed` - Current elapsed duration used to exclude reached waiters.
    ///
    /// # Returns
    ///
    /// The earliest future deadline, or `None` when none is registered.
    pub(crate) fn next_future_deadline(
        &self,
        elapsed: Duration,
    ) -> Option<Duration> {
        self.timer_waiters
            .values()
            .map(|(deadline, _)| deadline)
            .filter(|deadline| **deadline > elapsed)
            .min()
            .copied()
    }

    /// Takes task wakers for timer deadlines reached by elapsed.
    ///
    /// Waiter registrations remain present until their futures are polled or
    /// dropped, but subsequent advances cannot wake the same stored waker
    /// again.
    ///
    /// # Parameters
    ///
    /// * `elapsed` - Current elapsed duration defining which waiters are due.
    ///
    /// # Returns
    ///
    /// Every stored waker whose deadline is at or before `elapsed`.
    #[must_use = "due wakers should be invoked after unlocking"]
    pub(crate) fn take_due_timer_wakers(
        &mut self,
        elapsed: Duration,
    ) -> Vec<Waker> {
        self.timer_waiters
            .values_mut()
            .filter(|(deadline, _)| *deadline <= elapsed)
            .filter_map(|(_, waker)| waker.take())
            .collect()
    }

    /// Registers a count observer when count has not already been reached.
    ///
    /// Returns None when the requested count is already satisfied.
    ///
    /// Panics when the registry cannot allocate another identifier.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Waiter count at which the observer becomes ready.
    /// * `count` - Current registered waiter count.
    ///
    /// # Returns
    ///
    /// A new observer identifier, or `None` when `count` already satisfies the
    /// requested threshold.
    ///
    /// # Panics
    ///
    /// Panics when the observer identifier space is exhausted.
    #[inline]
    pub(crate) fn register_observer(
        &mut self,
        expected_count: usize,
        count: usize,
    ) -> Option<u64> {
        if count >= expected_count {
            return None;
        }
        let observer_id = allocate_identifier(
            &mut self.next_observer_id,
            "manual waiter observer identifiers exhausted",
        );
        self.count_observers
            .insert(observer_id, (expected_count, None));
        Some(observer_id)
    }

    /// Registers an observer for the earliest strictly future deadline.
    ///
    /// # Returns
    ///
    /// The nonzero identifier assigned to the deadline observer.
    ///
    /// # Panics
    ///
    /// Panics when the observer identifier space is exhausted.
    #[must_use = "the observer identifier is required to poll or cancel the wait"]
    pub(crate) fn register_deadline_observer(&mut self) -> u64 {
        let observer_id = allocate_identifier(
            &mut self.next_observer_id,
            "manual waiter observer identifiers exhausted",
        );
        self.deadline_observers.insert(observer_id, None);
        observer_id
    }

    /// Polls an observer and records the task waker while it remains pending.
    ///
    /// A missing observer is ready because reaching the count removes and
    /// latches the observer before waking its task.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Identifier of the observer to poll.
    /// * `context` - Task context whose waker is retained while pending.
    ///
    /// # Returns
    ///
    /// The observer poll state and any replaced or removed waker that the
    /// caller must destroy after releasing the clock state lock.
    #[must_use = "the poll state and detached waker must both be handled"]
    pub(crate) fn poll_observer(
        &mut self,
        observer_id: u64,
        context: &Context<'_>,
    ) -> (Poll<()>, Option<Waker>) {
        let Some((_, registered_waker)) =
            self.count_observers.get_mut(&observer_id)
        else {
            return (Poll::Ready(()), None);
        };
        let replaced_waker = if registered_waker
            .as_ref()
            .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            registered_waker.replace(context.waker().clone())
        } else {
            None
        };
        (Poll::Pending, replaced_waker)
    }

    /// Polls an observer for the earliest future deadline.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Identifier of the deadline observer to poll.
    /// * `elapsed` - Current elapsed duration used to exclude due waiters.
    /// * `context` - Task context whose waker is retained while pending.
    ///
    /// # Returns
    ///
    /// The deadline poll state and any replaced or removed waker that the
    /// caller must destroy after releasing the clock state lock.
    ///
    /// # Panics
    ///
    /// Panics if the identifier is missing or does not identify a deadline
    /// observer.
    #[must_use = "the poll state and detached waker must both be handled"]
    pub(crate) fn poll_deadline_observer(
        &mut self,
        observer_id: u64,
        elapsed: Duration,
        context: &Context<'_>,
    ) -> (Poll<Duration>, Option<Waker>) {
        if !self.deadline_observers.contains_key(&observer_id) {
            panic!("manual deadline observer {observer_id} is not registered");
        }
        if let Some(deadline) = self.next_future_deadline(elapsed) {
            let removed_waker =
                self.deadline_observers.remove(&observer_id).flatten();
            return (Poll::Ready(deadline), removed_waker);
        }
        let Some(registered_waker) =
            self.deadline_observers.get_mut(&observer_id)
        else {
            unreachable!("deadline observer existence was checked above");
        };
        let replaced_waker = if registered_waker
            .as_ref()
            .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            registered_waker.replace(context.waker().clone())
        } else {
            None
        };
        (Poll::Pending, replaced_waker)
    }

    /// Removes an incomplete waiter-registration observer and returns its task
    /// waker.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Identifier of the observer to remove.
    ///
    /// # Returns
    ///
    /// Its stored task waker, or `None` when absent or not yet polled.
    #[inline(always)]
    pub(crate) fn unregister_observer(
        &mut self,
        observer_id: u64,
    ) -> Option<Waker> {
        if let Some((_, waker)) = self.count_observers.remove(&observer_id) {
            return waker;
        }
        self.deadline_observers.remove(&observer_id).flatten()
    }

    /// Returns whether an observer is still waiting for its target count.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Identifier of the observer to inspect.
    ///
    /// # Returns
    ///
    /// `true` when the observer remains registered.
    #[must_use]
    #[inline(always)]
    pub(crate) fn contains_observer(&self, observer_id: u64) -> bool {
        self.count_observers.contains_key(&observer_id)
    }

    /// Updates the timer waiter waker or reports that its deadline is due.
    ///
    /// The returned ready state removes the waiter registration.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Identifier of the registered timer waiter.
    /// * `elapsed` - Current elapsed duration of the manual clock.
    /// * `context` - Task context whose waker is stored while pending.
    ///
    /// # Returns
    ///
    /// The waiter poll state and any replaced or removed waker that the caller
    /// must destroy after releasing the clock state lock.
    ///
    /// # Panics
    ///
    /// Panics if `waiter_id` no longer identifies a registered timer waiter.
    #[must_use = "the poll state and detached waker must both be handled"]
    pub(crate) fn poll_timer(
        &mut self,
        waiter_id: u64,
        elapsed: Duration,
        context: &Context<'_>,
    ) -> (Poll<()>, Option<Waker>) {
        let Some((deadline, registered_waker)) =
            self.timer_waiters.get_mut(&waiter_id)
        else {
            panic!("manual timer waiter {waiter_id} is not registered");
        };
        if elapsed < *deadline {
            let replaced_waker = if registered_waker
                .as_ref()
                .is_none_or(|waker| !waker.will_wake(context.waker()))
            {
                registered_waker.replace(context.waker().clone())
            } else {
                None
            };
            return (Poll::Pending, replaced_waker);
        }
        let removed_waker = self
            .timer_waiters
            .remove(&waiter_id)
            .and_then(|(_, waker)| waker);
        (Poll::Ready(()), removed_waker)
    }

    /// Removes reached observers and returns their registered task wakers.
    ///
    /// # Returns
    ///
    /// Stored task wakers for every observer whose threshold has been reached.
    #[must_use = "reached observer wakers should be invoked after unlocking"]
    pub(crate) fn reached_observer_wakers(
        &mut self,
        elapsed: Duration,
    ) -> Vec<Waker> {
        let count = self.count();
        let next_deadline = self.next_future_deadline(elapsed);
        let mut wakers = Vec::new();
        self.count_observers.retain(|_, (expected_count, waker)| {
            if *expected_count <= count {
                if let Some(waker) = waker.take() {
                    wakers.push(waker);
                }
                false
            } else {
                true
            }
        });
        if next_deadline.is_some() {
            self.deadline_observers.values_mut().for_each(|waker| {
                if let Some(waker) = waker.take() {
                    wakers.push(waker);
                }
            });
        }
        wakers
    }

    /// Returns the number of registered deadline waiters.
    ///
    /// # Returns
    ///
    /// The number of timer waiter registrations.
    #[must_use]
    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        self.timer_waiters.len()
    }
}
