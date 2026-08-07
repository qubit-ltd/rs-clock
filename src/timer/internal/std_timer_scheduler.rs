// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Schedules all standard Timer registrations on one process-wide worker.

// qubit-style: allow coverage-cfg

use std::io;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
#[cfg(coverage)]
use std::sync::atomic::AtomicBool;
#[cfg(coverage)]
use std::sync::atomic::AtomicUsize;
#[cfg(coverage)]
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::std_timer_scheduler_state::StdTimerSchedulerState;
use super::std_timer_waiter::StdTimerWaiter;
use super::std_timer_worker_guard::StdTimerWorkerGuard;
use crate::TimeError;
use crate::TimerUnavailableError;
use crate::internal::PanicFanout;

#[cfg(coverage)]
static FAIL_NEXT_WORKER_SPAWN: AtomicBool = AtomicBool::new(false);

#[cfg(coverage)]
static PANIC_NEXT_WORKER_RUN: AtomicBool = AtomicBool::new(false);

#[cfg(coverage)]
static WORKER_NOTIFICATION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Resets the coverage-only worker-notification counter.
#[cfg(coverage)]
pub fn reset_std_timer_worker_notification_count() {
    WORKER_NOTIFICATION_COUNT.store(0, Ordering::Release);
}

/// Returns the coverage-only worker-notification count.
///
/// # Returns
///
/// The number of scheduler notifications since the most recent reset.
#[cfg(coverage)]
#[must_use]
pub fn std_timer_worker_notification_count() -> usize {
    WORKER_NOTIFICATION_COUNT.load(Ordering::Acquire)
}

/// Makes the next standard Timer worker startup fail deterministically.
///
/// This coverage-only hook is public solely so the external integration suite
/// can exercise native spawn-failure recovery in an isolated test process.
#[cfg(coverage)]
pub fn fail_next_std_timer_worker_spawn() {
    FAIL_NEXT_WORKER_SPAWN.store(true, Ordering::Release);
}

/// Makes the next standard Timer worker panic after locking scheduler state.
///
/// This coverage-only hook is public solely so the external integration suite
/// can reproduce an unexpected worker exit in an isolated test process.
#[cfg(coverage)]
pub fn panic_next_std_timer_worker() {
    let scheduler = StdTimerScheduler::shared();
    let _state = scheduler.lock_state();
    PANIC_NEXT_WORKER_RUN.store(true, Ordering::Release);
    scheduler.notify_worker();
}

/// One lazily started worker shared by every standard Timer in the process.
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

    /// Returns the process-wide standard Timer scheduler.
    ///
    /// # Returns
    ///
    /// A shared scheduler whose worker starts lazily and remains parked while
    /// idle.
    #[must_use]
    #[inline]
    pub(crate) fn shared() -> Arc<Self> {
        static SCHEDULER: OnceLock<Arc<StdTimerScheduler>> = OnceLock::new();
        Arc::clone(SCHEDULER.get_or_init(|| Arc::new(Self::new())))
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
    /// Returns [`TimeError::TimerUnavailable`] with
    /// [`TimerUnavailableError::WorkerThreadSpawnFailed`] when the worker
    /// thread cannot be started. The attempted registration is removed before
    /// returning.
    ///
    /// # Panics
    ///
    /// Panics when registration identifiers or worker generations are
    /// exhausted, or an internal scheduler index invariant is violated.
    pub(crate) fn register(
        self: &Arc<Self>,
        deadline: Instant,
        waiter: Arc<StdTimerWaiter>,
    ) -> Result<u64, TimeError> {
        let mut state = self.lock_state();
        let previous_deadline = state.next_deadline();
        let waiter_id = state.register(deadline, waiter);
        let next_deadline_changed = state.next_deadline() != previous_deadline;
        if state.worker_running() {
            drop(state);
            if next_deadline_changed {
                self.notify_worker();
            }
            return Ok(waiter_id);
        }
        let worker_generation = state.mark_worker_started();
        self.spawn_worker(state, waiter_id, worker_generation)
    }

    /// Cancels an active waiter and wakes the worker when its earliest deadline
    /// changes.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Registration identifier to cancel.
    ///
    /// # Panics
    ///
    /// Panics when the registration index contains an entry without its exact
    /// deadline key.
    #[inline]
    pub(crate) fn cancel(&self, waiter_id: u64) {
        let mut state = self.lock_state();
        let previous_deadline = state.next_deadline();
        let waiter = state.cancel(waiter_id);
        let next_deadline_changed =
            waiter.is_some() && state.next_deadline() != previous_deadline;
        drop(state);
        drop(waiter);
        if next_deadline_changed {
            self.notify_worker();
        }
    }

    /// Fails registrations owned by a worker that has exited.
    ///
    /// Scheduler state is restored atomically under its lock. Waiter
    /// transitions, Waker destruction, and Waker invocation occur only
    /// after that lock is released. A disarmed startup guard passes
    /// generation zero and therefore leaves the active worker generation
    /// unchanged.
    ///
    /// # Parameters
    ///
    /// * `worker_generation` - Exited worker generation, or zero for a disarmed
    ///   startup guard.
    pub(super) fn handle_worker_exit(&self, worker_generation: u64) {
        let waiters = self
            .lock_state()
            .stop_worker_and_take_waiters(worker_generation);
        let wakers =
            waiters.iter().filter_map(|waiter| waiter.fail()).collect();
        drop(waiters);
        let mut fanout = PanicFanout::new();
        fanout.wake_all(wakers);
        fanout.discard_panics();
    }

    /// Notifies the worker that its scheduler wait may need recalculation.
    #[inline]
    fn notify_worker(&self) {
        #[cfg(coverage)]
        WORKER_NOTIFICATION_COUNT.fetch_add(1, Ordering::AcqRel);
        self.changed.notify_one();
    }

    /// Starts the native worker after its running state is committed.
    ///
    /// Production failures originate from the operating system. Instrumented
    /// coverage builds can inject the same failure before the native call so
    /// rollback and source preservation remain deterministic to test.
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
    /// Returns [`TimeError::TimerUnavailable`] with
    /// [`TimerUnavailableError::WorkerThreadSpawnFailed`] when the worker
    /// cannot spawn.
    ///
    /// # Panics
    ///
    /// Panics when spawn rollback encounters inconsistent scheduler indexes.
    fn spawn_worker(
        self: &Arc<Self>,
        mut state: MutexGuard<'_, StdTimerSchedulerState>,
        waiter_id: u64,
        worker_generation: u64,
    ) -> Result<u64, TimeError> {
        let scheduler = Arc::clone(self);
        let spawn_result =
            Self::spawn_native_worker(scheduler, worker_generation);
        if let Err(source) = spawn_result {
            return Err(Self::rollback_failed_worker_start(
                &mut state,
                waiter_id,
                worker_generation,
                source,
            ));
        }
        drop(state);
        Ok(waiter_id)
    }

    /// Starts one native scheduler worker.
    ///
    /// # Parameters
    ///
    /// * `scheduler` - Shared scheduler owned by the worker.
    /// * `worker_generation` - Generation assigned to the worker.
    ///
    /// # Returns
    ///
    /// The native worker handle.
    ///
    /// # Errors
    ///
    /// Returns the operating-system thread-spawn error. Coverage builds can
    /// also inject the same failure before invoking the operating system.
    fn spawn_native_worker(
        scheduler: Arc<Self>,
        worker_generation: u64,
    ) -> io::Result<std::thread::JoinHandle<()>> {
        #[cfg(coverage)]
        if FAIL_NEXT_WORKER_SPAWN.swap(false, Ordering::AcqRel) {
            return Err(io::Error::other(
                "injected standard Timer worker spawn failure",
            ));
        }
        std::thread::Builder::new()
            .name("qubit-clock-timer".to_owned())
            .spawn(move || {
                let startup_guard = StdTimerWorkerGuard::new(
                    scheduler.as_ref(),
                    worker_generation,
                );
                let _worker_guard = startup_guard.handoff();
                scheduler.run();
            })
    }

    /// Rolls back state after the operating system rejects a worker spawn.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked scheduler state to restore.
    /// * `waiter_id` - Registration created for the failed worker.
    /// * `worker_generation` - Worker generation to mark as stopped.
    /// * `source` - Native thread-spawn error.
    ///
    /// # Returns
    ///
    /// A Timer-unavailable error retaining `source`.
    fn rollback_failed_worker_start(
        state: &mut StdTimerSchedulerState,
        waiter_id: u64,
        worker_generation: u64,
        source: io::Error,
    ) -> TimeError {
        drop(state.cancel(waiter_id));
        state.mark_worker_stopped(worker_generation);
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::WorkerThreadSpawnFailed { source },
        }
    }

    /// Runs deadline selection and completion for the process lifetime.
    ///
    /// The worker waits on the scheduler condition variable while no
    /// registrations are active, so future standard Timers can reuse it without
    /// spawning another native thread.
    ///
    /// # Panics
    ///
    /// Panics when active scheduler indexes disagree about their earliest
    /// deadline or registration.
    fn run(&self) {
        let mut state = self.lock_state();
        loop {
            #[cfg(coverage)]
            if PANIC_NEXT_WORKER_RUN.swap(false, Ordering::AcqRel) {
                panic!("injected standard Timer worker failure");
            }
            if state.is_empty() {
                state = self
                    .changed
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
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

    /// Locks scheduler state, recovering the inner value after poisoning.
    ///
    /// # Returns
    ///
    /// A guard granting mutable access to scheduler state.
    #[inline(always)]
    fn lock_state(&self) -> MutexGuard<'_, StdTimerSchedulerState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
