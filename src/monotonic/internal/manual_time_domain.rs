// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stores the shared mutable state of one manual monotonic time domain.

use super::{
    AdvanceEffects,
    ManualMonotonicState,
};
use crate::TimeError;
use std::sync::{
    Condvar,
    Mutex,
    MutexGuard,
};
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::{
    Duration,
    Instant,
};

/// Shared synchronization state retained by same-domain manual clock handles.
pub(crate) struct ManualTimeDomain {
    /// Mutable logical time, waiter registrations, and advance observers.
    state: Mutex<ManualMonotonicState>,
    /// Condition variable notifying coordination helpers of waiter changes.
    waiters_changed: Condvar,
}

impl ManualTimeDomain {
    /// Creates an empty time domain at elapsed duration zero.
    ///
    /// # Returns
    ///
    /// Shared-state storage for a newly allocated manual clock domain.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(ManualMonotonicState::new()),
            waiters_changed: Condvar::new(),
        }
    }

    /// Returns the current logical duration from this domain's origin.
    ///
    /// # Returns
    ///
    /// The elapsed manual duration.
    #[must_use]
    #[inline]
    pub(crate) fn elapsed(&self) -> Duration {
        self.lock_state().elapsed
    }

    /// Advances logical time by `duration` and collects due effects.
    ///
    /// # Parameters
    ///
    /// * `duration` - Logical duration to add.
    ///
    /// # Returns
    ///
    /// Due effects for a nonzero advance, or `None` for a zero-duration no-op.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when elapsed time overflows.
    pub(crate) fn advance(
        &self,
        duration: Duration,
    ) -> Result<Option<AdvanceEffects>, TimeError> {
        if duration.is_zero() {
            return Ok(None);
        }
        let mut state = self.lock_state();
        state.elapsed = state
            .elapsed
            .checked_add(duration)
            .ok_or(TimeError::InstantOverflow)?;
        Ok(Some(Self::collect_advance_effects(&mut state)))
    }

    /// Advances logical time to `target_elapsed` and collects due effects.
    ///
    /// # Parameters
    ///
    /// * `target_elapsed` - Target duration from this domain's origin.
    ///
    /// # Returns
    ///
    /// Due effects after moving forward, or `None` at the current instant.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::CannotMoveBackward`] for an earlier target.
    pub(crate) fn advance_to(
        &self,
        target_elapsed: Duration,
    ) -> Result<Option<AdvanceEffects>, TimeError> {
        let mut state = self.lock_state();
        if target_elapsed < state.elapsed {
            return Err(TimeError::CannotMoveBackward);
        }
        if target_elapsed == state.elapsed {
            return Ok(None);
        }
        state.elapsed = target_elapsed;
        Ok(Some(Self::collect_advance_effects(&mut state)))
    }

    /// Returns the number of registered timer waiters.
    ///
    /// # Returns
    ///
    /// The active waiter count, including reached waiters awaiting cleanup.
    #[must_use]
    #[inline]
    pub(crate) fn waiter_count(&self) -> usize {
        self.lock_state().waiter_count()
    }

    /// Returns the earliest registered deadline after current logical time.
    ///
    /// # Returns
    ///
    /// The future deadline's elapsed duration, or `None`.
    #[must_use]
    #[inline]
    pub(crate) fn next_future_deadline(&self) -> Option<Duration> {
        let state = self.lock_state();
        state.waiters.next_future_deadline(state.elapsed)
    }

    /// Waits in real time for a future deadline registration.
    ///
    /// # Parameters
    ///
    /// * `real_timeout` - Maximum native time spent waiting.
    ///
    /// # Returns
    ///
    /// The future deadline's elapsed duration, or `None` after timeout or
    /// timeout-representation overflow.
    #[must_use]
    pub(crate) fn wait_for_next_deadline(
        &self,
        real_timeout: Duration,
    ) -> Option<Duration> {
        let mut state = self.lock_state();
        if let Some(deadline) =
            state.waiters.next_future_deadline(state.elapsed)
        {
            return Some(deadline);
        }
        let real_deadline = Instant::now().checked_add(real_timeout)?;
        loop {
            let remaining =
                real_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (next_state, wait_result) = self
                .waiters_changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next_state;
            if let Some(deadline) =
                state.waiters.next_future_deadline(state.elapsed)
            {
                return Some(deadline);
            }
            if wait_result.timed_out() {
                return None;
            }
        }
    }

    /// Advances to the earliest future deadline and collects due effects.
    ///
    /// # Returns
    ///
    /// The reached elapsed duration and due effects, or `None` when no future
    /// deadline is registered.
    pub(crate) fn advance_to_next_deadline(
        &self,
    ) -> Option<(Duration, AdvanceEffects)> {
        let mut state = self.lock_state();
        let target_elapsed =
            state.waiters.next_future_deadline(state.elapsed)?;
        state.elapsed = target_elapsed;
        let effects = Self::collect_advance_effects(&mut state);
        Some((target_elapsed, effects))
    }

    /// Waits for enough timer waiters and advances to the earliest deadline.
    ///
    /// The waiter-count check, future-deadline selection, and logical-time
    /// update occur while one state lock is held. Registrations that are
    /// already due may contribute to `expected_count`, but a future deadline
    /// must still exist before the clock advances.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Minimum active timer waiter count.
    /// * `real_timeout` - Maximum native time spent waiting for both
    ///   conditions.
    ///
    /// # Returns
    ///
    /// The reached elapsed duration and due effects. Returns `None` when the
    /// conditions remain unsatisfied until the real-time guard expires or the
    /// guard cannot be represented.
    pub(crate) fn advance_to_next_deadline_after_waiters(
        &self,
        expected_count: usize,
        real_timeout: Duration,
    ) -> Option<(Duration, AdvanceEffects)> {
        let mut state = self.lock_state();
        if let Some(advance) =
            Self::advance_if_waiters_ready(&mut state, expected_count)
        {
            return Some(advance);
        }
        let real_deadline = Instant::now().checked_add(real_timeout)?;
        loop {
            let remaining =
                real_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (next_state, wait_result) = self
                .waiters_changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next_state;
            if let Some(advance) =
                Self::advance_if_waiters_ready(&mut state, expected_count)
            {
                return Some(advance);
            }
            if wait_result.timed_out() {
                return None;
            }
        }
    }

    /// Waits in real time for the timer waiter count to reach `expected_count`.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Waiter count that satisfies the wait.
    /// * `real_timeout` - Maximum native time spent waiting.
    ///
    /// # Returns
    ///
    /// `true` when the count is reached, otherwise `false` after timeout or
    /// timeout-representation overflow.
    ///
    /// # Panics
    ///
    /// Panics when observer identifiers are exhausted.
    pub(crate) fn wait_for_waiters(
        &self,
        expected_count: usize,
        real_timeout: Duration,
    ) -> bool {
        let mut state = self.lock_state();
        let count = state.waiter_count();
        if count >= expected_count {
            return true;
        }
        let Some(real_deadline) = Instant::now().checked_add(real_timeout)
        else {
            return false;
        };
        let Some(observer_id) =
            state.waiters.register_observer(expected_count, count)
        else {
            return true;
        };
        loop {
            let remaining =
                real_deadline.saturating_duration_since(Instant::now());
            let (next_state, wait_result) = self
                .waiters_changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next_state;
            if !state.waiters.contains_observer(observer_id) {
                return true;
            }
            if wait_result.timed_out() {
                let removed_waker =
                    state.waiters.unregister_observer(observer_id);
                drop(state);
                drop(removed_waker);
                return false;
            }
        }
    }

    /// Registers a future timer deadline and detaches reached observer wakers.
    ///
    /// # Parameters
    ///
    /// * `deadline_elapsed` - Deadline duration from the domain origin.
    ///
    /// # Returns
    ///
    /// The new waiter identifier and observer wakers, or `None` when the
    /// deadline has already been reached.
    ///
    /// # Panics
    ///
    /// Panics when timer waiter identifiers are exhausted.
    pub(crate) fn register_timer_waiter(
        &self,
        deadline_elapsed: Duration,
    ) -> Option<(u64, Vec<Waker>)> {
        let mut state = self.lock_state();
        if state.elapsed >= deadline_elapsed {
            return None;
        }
        let waiter_id = state.waiters.register_timer(deadline_elapsed);
        let elapsed = state.elapsed;
        let observer_wakers = state.waiters.reached_observer_wakers(elapsed);
        Some((waiter_id, observer_wakers))
    }

    /// Registers an observer of the total timer waiter count.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Count that satisfies the observer.
    ///
    /// # Returns
    ///
    /// A new observer identifier, or `None` when already satisfied.
    ///
    /// # Panics
    ///
    /// Panics when observer identifiers are exhausted.
    #[inline]
    pub(crate) fn register_waiter_observer(
        &self,
        expected_count: usize,
    ) -> Option<u64> {
        let mut state = self.lock_state();
        let count = state.waiter_count();
        state.waiters.register_observer(expected_count, count)
    }

    /// Registers an observer of the next future deadline.
    ///
    /// # Returns
    ///
    /// The new observer identifier.
    ///
    /// # Panics
    ///
    /// Panics when observer identifiers are exhausted.
    #[must_use]
    #[inline]
    pub(crate) fn register_deadline_observer(&self) -> u64 {
        self.lock_state().waiters.register_deadline_observer()
    }

    /// Polls a next-deadline observer and detaches its replaced waker.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Deadline observer identifier.
    /// * `context` - Task context used to update the observer waker.
    ///
    /// # Returns
    ///
    /// The raw elapsed deadline poll state and detached waker.
    #[must_use = "the poll state and detached waker must both be handled"]
    pub(crate) fn poll_deadline_observer(
        &self,
        observer_id: u64,
        context: &Context<'_>,
    ) -> (Poll<Duration>, Option<Waker>) {
        let mut state = self.lock_state();
        let elapsed = state.elapsed;
        state
            .waiters
            .poll_deadline_observer(observer_id, elapsed, context)
    }

    /// Polls a waiter-count observer and detaches its replaced waker.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Count observer identifier.
    /// * `context` - Task context used to update the observer waker.
    ///
    /// # Returns
    ///
    /// The observer poll state and detached waker.
    #[must_use = "the poll state and detached waker must both be handled"]
    #[inline]
    pub(crate) fn poll_waiter_observer(
        &self,
        observer_id: u64,
        context: &Context<'_>,
    ) -> (Poll<()>, Option<Waker>) {
        self.lock_state()
            .waiters
            .poll_observer(observer_id, context)
    }

    /// Unregisters an incomplete observer and returns its detached waker.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Observer identifier to remove.
    ///
    /// # Returns
    ///
    /// The observer's registered waker, or `None`.
    #[inline]
    pub(crate) fn unregister_waiter_observer(
        &self,
        observer_id: u64,
    ) -> Option<Waker> {
        self.lock_state().waiters.unregister_observer(observer_id)
    }

    /// Polls a timer waiter and notifies native coordination after completion.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Timer waiter identifier.
    /// * `context` - Task context used to update the waiter waker.
    ///
    /// # Returns
    ///
    /// The waiter poll state and detached waker.
    #[must_use = "the poll state and detached waker must both be handled"]
    pub(crate) fn poll_timer_waiter(
        &self,
        waiter_id: u64,
        context: &Context<'_>,
    ) -> (Poll<()>, Option<Waker>) {
        let result = {
            let mut state = self.lock_state();
            let elapsed = state.elapsed;
            state.waiters.poll_timer(waiter_id, elapsed, context)
        };
        let (poll, _) = result;
        if poll.is_ready() {
            self.notify_waiters_changed();
        }
        result
    }

    /// Unregisters a timer waiter and notifies native coordination.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Timer waiter identifier to remove.
    ///
    /// # Returns
    ///
    /// The removed waiter's optional waker, or `None` when it was absent.
    pub(crate) fn unregister_timer_waiter(
        &self,
        waiter_id: u64,
    ) -> Option<Option<Waker>> {
        let removed_waiter =
            self.lock_state().waiters.unregister_timer(waiter_id);
        if removed_waiter.is_some() {
            self.notify_waiters_changed();
        }
        removed_waiter
    }

    /// Notifies all native coordination waiters after registration changes.
    #[inline(always)]
    pub(crate) fn notify_waiters_changed(&self) {
        self.waiters_changed.notify_all();
    }

    /// Locks mutable domain state, recovering after poisoning.
    ///
    /// # Returns
    ///
    /// A guard granting mutable state access.
    #[inline]
    fn lock_state(&self) -> MutexGuard<'_, ManualMonotonicState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Advances locked state when its waiter and deadline conditions are met.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked mutable state to inspect and possibly advance.
    /// * `expected_count` - Minimum active timer waiter count.
    ///
    /// # Returns
    ///
    /// The reached elapsed duration and due effects, or `None` while either
    /// condition remains unsatisfied.
    #[inline]
    fn advance_if_waiters_ready(
        state: &mut ManualMonotonicState,
        expected_count: usize,
    ) -> Option<(Duration, AdvanceEffects)> {
        if state.waiter_count() < expected_count {
            return None;
        }
        let target_elapsed =
            state.waiters.next_future_deadline(state.elapsed)?;
        state.elapsed = target_elapsed;
        let effects = Self::collect_advance_effects(state);
        Some((target_elapsed, effects))
    }

    /// Collects due task wakers while state remains locked.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked mutable state after a time advance.
    ///
    /// # Returns
    ///
    /// Owned effects ready for delivery after unlocking.
    #[inline]
    fn collect_advance_effects(
        state: &mut ManualMonotonicState,
    ) -> AdvanceEffects {
        let elapsed = state.elapsed;
        let due_wakers = state.waiters.take_due_timer_wakers(elapsed);
        AdvanceEffects { due_wakers }
    }
}
