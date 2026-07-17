// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Schedules standard timer registrations on one shared worker thread.

use super::std_timer_scheduler_state::StdTimerSchedulerState;
use super::std_timer_waiter::StdTimerWaiter;
use crate::TimeError;
use crate::internal::PanicFanout;
use std::sync::{
    Arc,
    Condvar,
    Mutex,
    MutexGuard,
};
use std::time::{
    Duration,
    Instant,
};

/// Grace period during which an idle worker can serve a new registration.
const WORKER_IDLE_GRACE: Duration = Duration::from_millis(1);

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
        let waiter_id = state.register(deadline, waiter);
        if state.worker_running() {
            drop(state);
            self.changed.notify_one();
            return Ok(waiter_id);
        }
        let worker_generation = state.mark_worker_started();
        self.spawn_worker(state, waiter_id, worker_generation)
    }

    /// Starts the native worker after its running state is committed.
    ///
    /// Native thread-spawn failure depends on operating-system failure
    /// injection that cannot be made deterministic in the integration suite.
    /// Observable registration behavior is exercised through the public Timer
    /// tests.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked scheduler state with the worker marked as running.
    /// * `waiter_id` - Registration to roll back if spawning fails.
    /// * `worker_generation` - Generation assigned to the new worker.
    ///
    /// # Returns
    ///
    /// The registration identifier after a successful spawn.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::TimerUnavailable`] when the worker cannot spawn.
    fn spawn_worker(
        self: &Arc<Self>,
        mut state: MutexGuard<'_, StdTimerSchedulerState>,
        waiter_id: u64,
        worker_generation: u64,
    ) -> Result<u64, TimeError> {
        let scheduler = Arc::clone(self);
        let spawn_result = std::thread::Builder::new()
            .name("qubit-clock-timer".to_owned())
            .spawn(move || {
                let _worker_guard = StdTimerWorkerGuard::new(
                    scheduler.as_ref(),
                    worker_generation,
                );
                scheduler.run(worker_generation);
            });
        if spawn_result.is_err() {
            drop(state.cancel(waiter_id));
            state.mark_worker_stopped(worker_generation);
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
        let waiter = self.lock_state().cancel(waiter_id);
        let was_registered = waiter.is_some();
        drop(waiter);
        if was_registered {
            self.changed.notify_one();
        }
    }

    /// Runs deadline selection and completion until no active waiters remain.
    fn run(&self, worker_generation: u64) {
        let mut state = self.lock_state();
        loop {
            if state.is_empty() {
                let (next_state, _) = self
                    .changed
                    .wait_timeout(state, WORKER_IDLE_GRACE)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                state = next_state;
                if state.is_empty() {
                    state.mark_worker_stopped(worker_generation);
                    return;
                }
                continue;
            }
            let deadline = state.next_deadline().expect(
                "active standard Timer registration must have a deadline",
            );
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
            let due_waiters = state.take_due(now);
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

    /// Clears the worker-running flag after a worker exits.
    fn mark_worker_stopped(&self, worker_generation: u64) {
        self.lock_state().mark_worker_stopped(worker_generation);
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

/// Restores scheduler state whenever a standard Timer worker exits.
struct StdTimerWorkerGuard<'a> {
    /// Scheduler whose worker state must be restored during exit.
    scheduler: &'a StdTimerScheduler,
    /// Generation of the guarded worker.
    worker_generation: u64,
}

impl<'a> StdTimerWorkerGuard<'a> {
    /// Creates an exit guard for `scheduler` and its worker generation.
    #[must_use]
    #[inline(always)]
    const fn new(
        scheduler: &'a StdTimerScheduler,
        worker_generation: u64,
    ) -> Self {
        Self {
            scheduler,
            worker_generation,
        }
    }
}

impl Drop for StdTimerWorkerGuard<'_> {
    /// Restores scheduler state after normal exit or unwinding.
    #[inline]
    fn drop(&mut self) {
        self.scheduler.mark_worker_stopped(self.worker_generation);
    }
}
