// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the inline Tokio timer future representation.

// qubit-style: allow coverage-cfg

use crate::timer::internal::tokio_runtime_liveness::TokioRuntimeLiveness;
#[cfg(coverage)]
use crate::timer::tokio_timer::take_tokio_timer_sleep_poll_panic;
use crate::{TimeError, TimerUnavailableError};
use pin_project_lite::pin_project;
use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{sync::futures::OwnedNotified, time::Sleep};

pin_project! {
    /// Tokio sleep paired with shared retained-runtime liveness.
    #[derive(Debug)]
    pub(crate) struct TokioTimerFuture {
        // Native Tokio deadline future.
        #[pin]
        sleep: Sleep,
        // Shutdown notification owned by this deadline.
        #[pin]
        shutdown: OwnedNotified,
        // Shared state used for race-free shutdown checks.
        liveness: Arc<TokioRuntimeLiveness>,
    }
}

impl TokioTimerFuture {
    /// Creates a future for one native sleep.
    ///
    /// # Parameters
    ///
    /// * `sleep` - Native Tokio deadline future.
    /// * `liveness` - Shared state for the sleep's retained runtime.
    ///
    /// # Returns
    ///
    /// A single allocation containing both wait conditions.
    #[must_use]
    pub(crate) fn new(sleep: Sleep, liveness: Arc<TokioRuntimeLiveness>) -> Self {
        let shutdown = liveness.shutdown_notification();
        Self {
            sleep,
            shutdown,
            liveness,
        }
    }
}

impl Future for TokioTimerFuture {
    type Output = Result<(), TimeError>;

    /// Polls shutdown before the native sleep and preserves unexpected panics.
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.liveness.is_shutdown() || this.shutdown.as_mut().poll(context).is_ready() {
            return runtime_shutdown();
        }
        match catch_unwind(AssertUnwindSafe(|| {
            #[cfg(coverage)]
            if take_tokio_timer_sleep_poll_panic() {
                panic!("injected Tokio sleep poll panic");
            }
            this.sleep.as_mut().poll(context)
        })) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(())) => Poll::Ready(Ok(())),
            Err(payload) => {
                if this.liveness.is_shutdown() {
                    runtime_shutdown()
                } else {
                    resume_unwind(payload)
                }
            }
        }
    }
}

/// Creates the structured runtime-shutdown result.
///
/// # Returns
///
/// A ready timer-unavailable result reporting runtime shutdown.
fn runtime_shutdown() -> Poll<Result<(), TimeError>> {
    Poll::Ready(Err(TimeError::TimerUnavailable {
        source: TimerUnavailableError::RuntimeShuttingDown,
    }))
}
