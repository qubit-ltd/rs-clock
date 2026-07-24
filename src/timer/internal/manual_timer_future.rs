// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the cancellation-safe future used by manual timers.

use crate::{ManualMonotonicClock, MonotonicInstant, TimeError};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// A deadline future eagerly registered with one manual time domain.
pub(crate) struct ManualTimerFuture {
    /// Private clock handle retaining the registration's time domain.
    clock: Arc<ManualMonotonicClock>,
    /// Active waiter identifier, or `None` after immediate or polled
    /// readiness.
    waiter_id: Option<u64>,
}

impl ManualTimerFuture {
    /// Registers a manual timer deadline before returning its future.
    ///
    /// # Parameters
    ///
    /// * `clock` - Same-domain clock handle retained by the future.
    /// * `deadline` - Absolute deadline to register.
    ///
    /// # Returns
    ///
    /// A future containing an active registration, or an immediately ready
    /// future when the deadline has already been reached.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline.
    ///
    /// # Panics
    ///
    /// Panics when waiter identifiers are exhausted or a custom coordination
    /// waker panics during registration.
    pub(crate) fn register(
        clock: Arc<ManualMonotonicClock>,
        deadline: MonotonicInstant,
    ) -> Result<Self, TimeError> {
        let waiter_id = clock.register_timer_waiter(deadline)?;
        Ok(Self { clock, waiter_id })
    }
}

impl Future for ManualTimerFuture {
    type Output = Result<(), TimeError>;

    /// Checks manual time and records the current task waker while pending.
    ///
    /// # Parameters
    ///
    /// * `context` - Task context whose waker replaces a different prior waker.
    ///
    /// # Returns
    ///
    /// [`Poll::Ready`] once the registered deadline is reached, otherwise
    /// [`Poll::Pending`].
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let Some(waiter_id) = this.waiter_id else {
            return Poll::Ready(Ok(()));
        };
        let result = this.clock.poll_timer_waiter(waiter_id, context);
        if result.is_ready() {
            this.waiter_id = None;
        }
        result.map(Ok)
    }
}

impl Drop for ManualTimerFuture {
    /// Cancels an incomplete manual timer registration.
    #[inline]
    fn drop(&mut self) {
        if let Some(waiter_id) = self.waiter_id.take() {
            self.clock.unregister_timer_waiter(waiter_id);
        }
    }
}
