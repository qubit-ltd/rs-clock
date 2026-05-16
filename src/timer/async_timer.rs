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
    /// Unlike [`sleep_until_async`](Self::sleep_until_async), this method completes
    /// as soon as waiters are notified, even if the deadline has not been reached
    /// yet.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer's domain.
    ///
    /// # Returns
    ///
    /// A future that resolves to:
    ///
    /// * `Ok(TimerWaitOutcome::DeadlineReached)` when the deadline has been reached.
    /// * `Ok(TimerWaitOutcome::Notified)` when blocking or async notification woke
    ///   the wait before the deadline.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// was created by a different timer domain.
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> Pin<Box<dyn Future<Output = Result<TimerWaitOutcome, TimerError>> + Send + 'a>>;

    /// Waits asynchronously until the deadline has been reached.
    ///
    /// Explicit notifications wake the underlying [`wait_until_async`](Self::wait_until_async)
    /// call, but this method keeps waiting until the deadline is actually reached.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer's domain.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` once the timer's monotonic time has
    /// reached or passed `deadline`.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// was created by a different timer domain.
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
    /// This is equivalent to [`sleep_until_async`](Self::sleep_until_async) with a
    /// deadline created by [`MonotonicTimer::deadline_after`].
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative delay from the current instant.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` once the timer's monotonic time has
    /// advanced by at least `duration` from the instant observed when the future
    /// is polled for the first time.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] only if the
    /// self-created deadline is rejected by an invalid timer implementation.
    fn sleep_for_async<'a>(
        &'a self,
        duration: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<(), TimerError>> + Send + 'a>> {
        self.sleep_until_async(self.deadline_after(duration))
    }
}
