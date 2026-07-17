// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Restores scheduler state when a standard Timer worker unwinds.

use super::std_timer_scheduler::StdTimerScheduler;

/// Clears the worker-running flag after an unexpected worker exit.
pub(super) struct StdTimerWorkerGuard<'a> {
    /// Scheduler whose worker state must be restored during unwinding.
    scheduler: &'a StdTimerScheduler,
    /// Whether dropping this guard must restore scheduler state.
    armed: bool,
}

impl<'a> StdTimerWorkerGuard<'a> {
    /// Creates an armed guard for `scheduler`.
    ///
    /// # Parameters
    ///
    /// * `scheduler` - Scheduler currently running on the guarded worker.
    ///
    /// # Returns
    ///
    /// A guard that restores worker state unless disarmed after normal exit.
    #[must_use]
    #[inline(always)]
    pub(super) const fn new(scheduler: &'a StdTimerScheduler) -> Self {
        Self {
            scheduler,
            armed: true,
        }
    }

    /// Marks the worker as having completed its normal locked state transition.
    #[inline(always)]
    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StdTimerWorkerGuard<'_> {
    /// Restores scheduler state only when the worker exits unexpectedly.
    #[inline]
    fn drop(&mut self) {
        if self.armed {
            self.scheduler.mark_worker_stopped();
        }
    }
}
