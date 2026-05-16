/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::Duration;

use crate::timer::{
    MonotonicTimer,
    TimerError,
    TimerInstant,
    TimerWaitOutcome,
};

/// Adds blocking wait operations to a monotonic timer.
pub trait BlockingTimer: MonotonicTimer {
    /// Blocks until the deadline is reached or waiters are explicitly notified.
    ///
    /// Returns `Err` when the deadline belongs to another timer domain.
    fn wait_until(&self, deadline: TimerInstant) -> Result<TimerWaitOutcome, TimerError>;

    /// Wakes current waiters without advancing time.
    fn notify_waiters(&self);

    /// Blocks for a duration relative to this timer's current instant.
    ///
    /// Returns `Err` only if the self-created deadline is rejected by an invalid
    /// timer implementation.
    fn wait_for(&self, duration: Duration) -> Result<TimerWaitOutcome, TimerError> {
        self.wait_until(self.deadline_after(duration))
    }

    /// Blocks until the deadline has been reached.
    ///
    /// Explicit notifications wake the underlying wait, but this method keeps
    /// waiting until the deadline is reached. Returns `Err` when the deadline
    /// belongs to another timer domain.
    fn sleep_until(&self, deadline: TimerInstant) -> Result<(), TimerError> {
        loop {
            match self.wait_until(deadline)? {
                TimerWaitOutcome::DeadlineReached => return Ok(()),
                TimerWaitOutcome::Notified => {}
            }
        }
    }

    /// Blocks for a duration relative to this timer's current instant.
    ///
    /// Returns `Err` only if the self-created deadline is rejected by an invalid
    /// timer implementation.
    fn sleep_for(&self, duration: Duration) -> Result<(), TimerError> {
        self.sleep_until(self.deadline_after(duration))
    }
}
