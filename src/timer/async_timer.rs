/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::timer::{
    MonotonicTimer,
    TimerError,
    TimerInstant,
    TimerWaitOutcome,
};

/// Adds Tokio-compatible asynchronous wait operations to a monotonic timer.
pub trait AsyncTimer: MonotonicTimer {
    /// Waits asynchronously until the deadline is reached or waiters are
    /// explicitly notified.
    ///
    /// Returns `Err` when the deadline belongs to another timer domain.
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> Pin<Box<dyn Future<Output = Result<TimerWaitOutcome, TimerError>> + Send + 'a>>;

    /// Waits asynchronously until the deadline has been reached.
    ///
    /// Explicit notifications wake the underlying wait, but this method keeps
    /// waiting until the deadline is reached. Returns `Err` when the deadline
    /// belongs to another timer domain.
    fn sleep_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> Pin<Box<dyn Future<Output = Result<(), TimerError>> + Send + 'a>> {
        Box::pin(async move {
            loop {
                match self.wait_until_async(deadline).await? {
                    TimerWaitOutcome::DeadlineReached => return Ok(()),
                    TimerWaitOutcome::Notified => {}
                }
            }
        })
    }

    /// Waits asynchronously for a duration relative to this timer's current
    /// instant.
    ///
    /// Returns `Err` only if the self-created deadline is rejected by an invalid
    /// timer implementation.
    fn sleep_for_async<'a>(
        &'a self,
        duration: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<(), TimerError>> + Send + 'a>> {
        self.sleep_until_async(self.deadline_after(duration))
    }
}
