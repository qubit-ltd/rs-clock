// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Schedules standard timer registrations on one shared worker thread.

use super::std_timer_waiter::StdTimerWaiter;
use super::std_timer_worker_guard::StdTimerWorkerGuard;
use crate::TimeError;
use crate::internal::PanicFanout;
use std::cmp::Reverse;
use std::collections::{
    BinaryHeap,
    HashMap,
};
use std::sync::{
    Arc,
    Condvar,
    Mutex,
    MutexGuard,
};
use std::time::Instant;

/// Mutable registrations protected by one scheduler lock.
struct StdTimerSchedulerState {
    /// Next nonzero registration identifier.
    next_waiter_id: u64,
    /// Deadline keys ordered from earliest to latest, including stale keys.
    deadlines: BinaryHeap<Reverse<(Instant, u64)>>,
    /// Active waiters keyed by registration identifier.
    waiters: HashMap<u64, Arc<StdTimerWaiter>>,
    /// Whether a scheduler worker is currently running.
    worker_running: bool,
}

impl StdTimerSchedulerState {
    /// Creates empty scheduler state.
    ///
    /// # Returns
    ///
    /// State without registrations or a worker thread.
    #[must_use]
    fn new() -> Self {
        Self {
            next_waiter_id: 1,
            deadlines: BinaryHeap::new(),
            waiters: HashMap::new(),
            worker_running: false,
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
    fn allocate_waiter_id(&mut self) -> u64 {
        let waiter_id = self.next_waiter_id;
        assert_ne!(waiter_id, 0, "standard timer waiter identifiers exhausted");
        self.next_waiter_id = waiter_id.wrapping_add(1);
        waiter_id
    }

    /// Removes stale heap keys whose registrations have been cancelled.
    fn remove_stale_deadlines(&mut self) {
        while self
            .deadlines
            .peek()
            .is_some_and(|Reverse((_, waiter_id))| {
                !self.waiters.contains_key(waiter_id)
            })
        {
            self.deadlines.pop();
        }
    }
}

/// One lazily started worker shared by every future from a standard timer.
pub(crate) struct StdTimerScheduler {
    /// Mutable scheduler state.
    state: Mutex<StdTimerSchedulerState>,
    /// Wakes the worker for new earlier deadlines and cancellations.
    changed: Condvar,
}

impl StdTimerScheduler {
    /// Creates a scheduler without starting a worker thread.
    ///
    /// # Returns
    ///
    /// An empty lazy scheduler.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(StdTimerSchedulerState::new()),
            changed: Condvar::new(),
        }
    }

    /// Eagerly registers a waiter and starts the shared worker if necessary.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Native standard-library deadline.
    /// * `waiter` - Completion latch owned by the returned future.
    ///
    /// # Returns
    ///
    /// The active registration identifier.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::TimerUnavailable`] when the worker thread cannot be
    /// started. The attempted registration is removed before returning.
    pub(crate) fn register(
        self: &Arc<Self>,
        deadline: Instant,
        waiter: Arc<StdTimerWaiter>,
    ) -> Result<u64, TimeError> {
        let mut state = self.lock_state();
        let waiter_id = state.allocate_waiter_id();
        state.waiters.insert(waiter_id, waiter);
        state.deadlines.push(Reverse((deadline, waiter_id)));
        if state.worker_running {
            drop(state);
            self.changed.notify_one();
            return Ok(waiter_id);
        }
        state.worker_running = true;
        let scheduler = Arc::clone(self);
        let spawn_result = std::thread::Builder::new()
            .name("qubit-clock-timer".to_owned())
            .spawn(move || {
                let mut worker_guard =
                    StdTimerWorkerGuard::new(scheduler.as_ref());
                scheduler.run();
                worker_guard.disarm();
            });
        if spawn_result.is_err() {
            state.waiters.remove(&waiter_id);
            state.worker_running = false;
            return Err(TimeError::TimerUnavailable);
        }
        drop(state);
        Ok(waiter_id)
    }

    /// Cancels an active waiter and wakes the worker to recalculate its wait.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Registration identifier to cancel.
    pub(crate) fn cancel(&self, waiter_id: u64) {
        let waiter = self.lock_state().waiters.remove(&waiter_id);
        let was_registered = waiter.is_some();
        drop(waiter);
        if was_registered {
            self.changed.notify_one();
        }
    }

    /// Runs deadline selection and completion until no active waiters remain.
    fn run(&self) {
        let mut state = self.lock_state();
        loop {
            state.remove_stale_deadlines();
            if state.waiters.is_empty() {
                state.worker_running = false;
                return;
            }
            let Some(Reverse((deadline, _))) = state.deadlines.peek().copied()
            else {
                unreachable!("active standard timer waiter has no deadline");
            };
            let now = Instant::now();
            if deadline > now {
                let duration = deadline.duration_since(now);
                let (next_state, _) = self
                    .changed
                    .wait_timeout(state, duration)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                state = next_state;
                continue;
            }
            let mut due_waiters = Vec::new();
            while let Some(Reverse((deadline, waiter_id))) =
                state.deadlines.peek().copied()
            {
                if deadline > now {
                    break;
                }
                state.deadlines.pop();
                if let Some(waiter) = state.waiters.remove(&waiter_id) {
                    due_waiters.push(waiter);
                }
            }
            drop(state);
            let wakers = due_waiters
                .iter()
                .filter_map(|waiter| waiter.complete())
                .collect();
            drop(due_waiters);
            let mut fanout = PanicFanout::new();
            fanout.wake_all(wakers);
            fanout.discard_panics();
            state = self.lock_state();
        }
    }

    /// Clears the worker-running flag after an unexpected worker exit.
    pub(super) fn mark_worker_stopped(&self) {
        self.lock_state().worker_running = false;
    }

    /// Locks scheduler state, recovering the inner value after poisoning.
    ///
    /// # Returns
    ///
    /// A guard granting mutable access to scheduler state.
    fn lock_state(&self) -> MutexGuard<'_, StdTimerSchedulerState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
