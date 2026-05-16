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
    AsyncSleeper,
    AsyncTimerResult,
    AsyncWaiter,
    TimerInstant,
    TimerWaitOutcome,
    WaitNotifier,
};

/// Combines asynchronous sleep and wait operations for a timer domain.
///
/// This facade keeps the common async timer API available from one trait while
/// the underlying semantics remain split:
///
/// * `sleep_*_async` resolves only after the deadline is reached.
/// * `wait_*_async` resolves after the deadline or waiter notification.
pub trait AsyncTimer: AsyncSleeper + AsyncWaiter {
    /// Wakes all current waiters without advancing time.
    fn notify_all_waiters(&self) {
        WaitNotifier::notify_all_waiters(self);
    }

    /// Waits asynchronously until the deadline is reached or waiters are
    /// explicitly notified.
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> AsyncTimerResult<'a, TimerWaitOutcome> {
        AsyncWaiter::wait_until_async(self, deadline)
    }

    /// Waits asynchronously for a relative duration, or until waiters are
    /// explicitly notified.
    fn wait_for_async<'a>(&'a self, duration: Duration) -> AsyncTimerResult<'a, TimerWaitOutcome> {
        AsyncWaiter::wait_for_async(self, duration)
    }

    /// Waits asynchronously until the deadline has been reached.
    fn sleep_until_async<'a>(&'a self, deadline: TimerInstant) -> AsyncTimerResult<'a, ()> {
        AsyncSleeper::sleep_until_async(self, deadline)
    }

    /// Waits asynchronously for a duration relative to this timer's current
    /// instant.
    fn sleep_for_async<'a>(&'a self, duration: Duration) -> AsyncTimerResult<'a, ()> {
        AsyncSleeper::sleep_for_async(self, duration)
    }
}

impl<T> AsyncTimer for T where T: AsyncSleeper + AsyncWaiter {}
