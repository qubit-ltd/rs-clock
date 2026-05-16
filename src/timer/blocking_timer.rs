/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::timer::{
    BlockingSleeper,
    BlockingWaiter,
    TimerInstant,
    TimerResult,
    TimerWaitOutcome,
    WaitNotifier,
};

/// Combines blocking sleep and wait operations for a timer domain.
///
/// This facade keeps the common blocking timer API available from one trait
/// while the underlying semantics remain split:
///
/// * `sleep_*` blocks until the deadline is reached.
/// * `wait_*` blocks until the deadline is reached or waiters are notified.
pub trait BlockingTimer: BlockingSleeper + BlockingWaiter {
    /// Wakes all current waiters without advancing time.
    fn notify_all_waiters(&self) {
        WaitNotifier::notify_all_waiters(self);
    }

    /// Blocks until the deadline has been reached.
    fn sleep_until(&self, deadline: TimerInstant) -> TimerResult<()> {
        BlockingSleeper::sleep_until(self, deadline)
    }

    /// Blocks for a duration relative to this timer's current instant.
    fn sleep_for(&self, duration: std::time::Duration) -> TimerResult<()> {
        BlockingSleeper::sleep_for(self, duration)
    }

    /// Blocks until the deadline is reached or waiters are explicitly notified.
    fn wait_until(&self, deadline: TimerInstant) -> TimerResult<TimerWaitOutcome> {
        BlockingWaiter::wait_until(self, deadline)
    }

    /// Blocks for a duration relative to this timer's current instant, or until
    /// waiters are notified.
    fn wait_for(&self, duration: std::time::Duration) -> TimerResult<TimerWaitOutcome> {
        BlockingWaiter::wait_for(self, duration)
    }
}

impl<T> BlockingTimer for T where T: BlockingSleeper + BlockingWaiter {}
