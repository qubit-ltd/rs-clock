// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a future registered with the standard timer scheduler.

use super::std_timer_scheduler::StdTimerScheduler;
use super::std_timer_waiter::StdTimerWaiter;
use crate::{TimeError, TimerUnavailableError};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// A cancellation-safe future backed by one shared standard scheduler.
pub(crate) struct StdTimerFuture {
    /// Scheduler that owns the active registration.
    scheduler: Arc<StdTimerScheduler>,
    /// Registration identifier, cleared after observed completion.
    waiter_id: Option<u64>,
    /// Completion latch and task waker shared with the worker.
    waiter: Arc<StdTimerWaiter>,
}

impl StdTimerFuture {
    /// Creates a future for an already registered waiter.
    ///
    /// # Parameters
    ///
    /// * `scheduler` - Scheduler owning the registration.
    /// * `waiter_id` - Active registration identifier.
    /// * `waiter` - Completion state shared with the worker.
    ///
    /// # Returns
    ///
    /// A cancellation-safe pending future.
    #[must_use]
    #[inline]
    pub(crate) const fn new(
        scheduler: Arc<StdTimerScheduler>,
        waiter_id: u64,
        waiter: Arc<StdTimerWaiter>,
    ) -> Self {
        Self {
            scheduler,
            waiter_id: Some(waiter_id),
            waiter,
        }
    }
}

impl Future for StdTimerFuture {
    type Output = Result<(), TimeError>;

    /// Polls the worker-owned completion latch.
    ///
    /// # Parameters
    ///
    /// * `context` - Task context whose waker observes completion.
    ///
    /// # Returns
    ///
    /// [`Poll::Ready(Ok)`] after the native deadline,
    /// [`Poll::Ready(Err)`] if the scheduler worker exits, otherwise
    /// [`Poll::Pending`].
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this.waiter.poll(context) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                this.waiter_id = None;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(())) => {
                this.waiter_id = None;
                Poll::Ready(Err(TimeError::TimerUnavailable {
                    source: TimerUnavailableError::SchedulerWorkerTerminated,
                }))
            }
        }
    }
}

impl Drop for StdTimerFuture {
    /// Cancels the registration when the future has not observed completion.
    #[inline]
    fn drop(&mut self) {
        if let Some(waiter_id) = self.waiter_id.take() {
            self.scheduler.cancel(waiter_id);
        }
    }
}
