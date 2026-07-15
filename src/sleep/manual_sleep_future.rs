// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the cancellation-safe future used by manual async sleepers.

use crate::{
    ManualMonotonicClock,
    MonotonicInstant,
    TimeError,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
};

/// A future registered with one manual monotonic clock.
///
/// Registration happens when the future is created. Dropping an incomplete
/// future removes its waiter registration.
#[derive(Debug)]
pub(crate) struct ManualSleepFuture {
    clock: Arc<ManualMonotonicClock>,
    deadline: MonotonicInstant,
    waiter_id: Option<u64>,
    error: Option<TimeError>,
}

impl ManualSleepFuture {
    /// Creates and immediately registers a manual async wait.
    #[inline]
    pub(crate) fn new(
        clock: Arc<ManualMonotonicClock>,
        deadline: MonotonicInstant,
    ) -> Self {
        match clock.register_async_waiter(deadline) {
            Ok(waiter_id) => Self {
                clock,
                deadline,
                waiter_id,
                error: None,
            },
            Err(error) => Self {
                clock,
                deadline,
                waiter_id: None,
                error: Some(error),
            },
        }
    }
}

impl Future for ManualSleepFuture {
    type Output = Result<(), TimeError>;

    /// Checks manual time and registers the current task waker when pending.
    fn poll(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(error) = this.error.take() {
            return Poll::Ready(Err(error));
        }
        let Some(waiter_id) = this.waiter_id else {
            return Poll::Ready(Ok(()));
        };
        match this
            .clock
            .poll_async_waiter(waiter_id, this.deadline, context)
        {
            Poll::Ready(result) => {
                this.waiter_id = None;
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for ManualSleepFuture {
    /// Removes an incomplete waiter registration during cancellation.
    #[inline]
    fn drop(&mut self) {
        if let Some(waiter_id) = self.waiter_id.take() {
            self.clock.unregister_async_waiter(waiter_id);
        }
    }
}
