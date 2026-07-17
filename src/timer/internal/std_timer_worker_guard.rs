// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Restores scheduler state when a standard Timer worker exits.

use super::std_timer_scheduler::StdTimerScheduler;

/// Restores the running state whenever a standard Timer worker exits.
#[must_use = "dropping an active worker guard clears its scheduler generation"]
pub(super) struct StdTimerWorkerGuard<'a> {
    /// Scheduler whose worker state must be restored during exit.
    scheduler: &'a StdTimerScheduler,
    /// Generation of the guarded worker, or zero after responsibility moves to
    /// a successor guard.
    worker_generation: u64,
}

impl<'a> StdTimerWorkerGuard<'a> {
    /// Creates an exit guard for `scheduler` and its worker generation.
    ///
    /// # Parameters
    ///
    /// * `scheduler` - Scheduler whose worker is starting.
    /// * `worker_generation` - Generation assigned to the worker.
    ///
    /// # Returns
    ///
    /// A guard that restores scheduler state when the worker exits.
    #[inline(always)]
    pub(super) const fn new(
        scheduler: &'a StdTimerScheduler,
        worker_generation: u64,
    ) -> Self {
        Self {
            scheduler,
            worker_generation,
        }
    }

    /// Transfers cleanup responsibility to a steady-state worker guard.
    ///
    /// The consumed startup guard is disarmed before it is dropped, while the
    /// returned guard retains the active worker generation. Dropping the
    /// disarmed guard briefly locks scheduler state but cannot clear the active
    /// generation.
    ///
    /// # Returns
    ///
    /// A guard responsible for restoring scheduler state when the worker exits.
    #[inline(always)]
    pub(super) fn handoff(mut self) -> Self {
        let worker_guard = Self::new(self.scheduler, self.worker_generation);
        self.worker_generation = 0;
        worker_guard
    }
}

impl Drop for StdTimerWorkerGuard<'_> {
    /// Restores scheduler state after normal exit or unwinding.
    ///
    /// A handed-off guard carries generation zero, so its drop only performs a
    /// no-op generation check. The check may briefly block while acquiring the
    /// scheduler state mutex.
    #[inline(always)]
    fn drop(&mut self) {
        self.scheduler.mark_worker_stopped(self.worker_generation);
    }
}
