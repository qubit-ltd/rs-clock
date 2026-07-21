// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a timer driven by Tokio's time driver.

use crate::{
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    Timer,
    TimerFuture,
    TimerUnavailableError,
    TokioMonotonicClock,
    TokioRuntimeError,
};
use std::{
    convert::Infallible,
    panic::{
        AssertUnwindSafe,
        catch_unwind,
        resume_unwind,
    },
    task::{
        Context,
        Poll,
    },
    time::Duration,
};
use tokio::runtime::Handle;
use tokio::task::JoinSet;
use tokio::time::Instant;

/// An asynchronous timer backed by one Tokio runtime time driver.
///
/// The timer retains the source clock's exact domain, origin, and runtime
/// capability. It enters that runtime briefly to sample time and create each
/// Tokio sleep, so registration does not depend on the caller's ambient
/// runtime. The returned future may be polled elsewhere, but the retained
/// runtime must remain alive and driven until the future completes or is
/// dropped. If it shuts down first, a pending future returns
/// [`TimerUnavailableError::RuntimeShuttingDown`].
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioTimer {
    /// Private handle retaining the source clock domain and Tokio origin.
    clock: TokioMonotonicClock,
}

impl TokioTimer {
    /// Creates a timer backed by an explicit runtime handle.
    ///
    /// This constructor does not depend on an ambient Tokio runtime.
    ///
    /// # Parameters
    ///
    /// * `runtime` - Runtime capability providing the clock and time driver.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain backed by `runtime`.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn from_handle(runtime: Handle) -> Self {
        Self {
            clock: TokioMonotonicClock::from_handle(runtime),
        }
    }

    /// Creates a timer by capturing the currently entered Tokio runtime.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain retaining the current runtime's handle.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime is entered or all process-wide clock-domain
    /// identifiers are exhausted.
    #[must_use]
    #[track_caller]
    #[inline]
    pub fn current() -> Self {
        Self::try_current().unwrap_or_else(|error| {
            panic!("cannot create Tokio timer: {error}")
        })
    }

    /// Tries to create a timer by capturing the current Tokio runtime.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain retaining the current runtime's handle.
    ///
    /// # Errors
    ///
    /// Returns [`TokioRuntimeError::NotEntered`] when no Tokio runtime is
    /// entered.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[inline]
    pub fn try_current() -> Result<Self, TokioRuntimeError> {
        TokioMonotonicClock::try_current().map(|clock| Self { clock })
    }

    /// Creates a timer sharing the supplied Tokio clock's exact domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Tokio clock whose domain, origin, and runtime capability
    ///   apply.
    ///
    /// # Returns
    ///
    /// A timer retaining an independent same-domain clock handle.
    #[must_use]
    #[inline]
    pub fn from_clock(clock: &TokioMonotonicClock) -> Self {
        Self {
            clock: clock.same_domain_handle(),
        }
    }

    /// Converts a domain-scoped deadline to its native Tokio instant.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline in the source clock domain.
    ///
    /// # Returns
    ///
    /// The corresponding Tokio instant.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline and
    /// [`TimeError::InstantOverflow`] when conversion overflows.
    fn native_deadline(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<Instant, TimeError> {
        deadline.ensure_domain(self.clock.domain())?;
        self.clock
            .origin()
            .checked_add(deadline.elapsed_since_origin())
            .ok_or(TimeError::InstantOverflow)
    }

    /// Polls the retained runtime's liveness sentinel.
    ///
    /// # Parameters
    ///
    /// * `tasks` - Single pending task owned by the retained runtime.
    /// * `context` - Task context polling the Timer future.
    ///
    /// # Returns
    ///
    /// [`Poll::Pending`] while the retained runtime remains alive, or a
    /// structured shutdown error after that runtime cancels the sentinel.
    ///
    /// # Panics
    ///
    /// Resumes an unexpected sentinel-task panic and panics if the one-task
    /// set becomes empty without yielding a task result.
    fn poll_runtime_shutdown(
        tasks: &mut JoinSet<Infallible>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), TimeError>> {
        match tasks.poll_join_next(context) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Err(error))) if error.is_cancelled() => {
                Poll::Ready(Err(TimeError::TimerUnavailable {
                    source: TimerUnavailableError::RuntimeShuttingDown,
                }))
            }
            Poll::Ready(Some(Err(error))) => resume_unwind(error.into_panic()),
            Poll::Ready(Some(Ok(never))) => match never {},
            Poll::Ready(None) => {
                panic!("Tokio Timer task set must retain one liveness sentinel")
            }
        }
    }

    /// Creates the future for one native deadline while the target runtime is
    /// entered.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Fixed native Tokio deadline.
    /// * `now` - Single current-time sample used to detect reached deadlines.
    ///
    /// # Returns
    ///
    /// An immediately ready future or a Tokio sleep paired with a retained-
    /// runtime liveness sentinel. Dropping the future aborts the sentinel.
    ///
    /// # Errors
    ///
    /// Returns [`TimerUnavailableError::TimeDriverDisabled`] when a future
    /// deadline cannot be registered because Tokio time is disabled.
    fn schedule(
        deadline: Instant,
        now: Instant,
    ) -> Result<TimerFuture, TimeError> {
        if deadline <= now {
            return Ok(Box::pin(std::future::ready(Ok(()))));
        }
        let sleep = catch_unwind(AssertUnwindSafe(|| {
            tokio::time::sleep_until(deadline)
        }))
        .map_err(|_| TimeError::TimerUnavailable {
            source: TimerUnavailableError::TimeDriverDisabled,
        })?;
        let mut sentinel = JoinSet::new();
        sentinel.spawn(std::future::pending::<Infallible>());
        let mut sleep = Box::pin(sleep);
        Ok(Box::pin(std::future::poll_fn(move |context| {
            if let Poll::Ready(result) =
                Self::poll_runtime_shutdown(&mut sentinel, context)
            {
                return Poll::Ready(result);
            }
            match catch_unwind(AssertUnwindSafe(|| {
                sleep.as_mut().poll(context)
            })) {
                Ok(Poll::Pending) => Poll::Pending,
                Ok(Poll::Ready(())) => Poll::Ready(Ok(())),
                Err(payload) => {
                    if let Poll::Ready(result) =
                        Self::poll_runtime_shutdown(&mut sentinel, context)
                    {
                        Poll::Ready(result)
                    } else {
                        resume_unwind(payload)
                    }
                }
            }
        })))
    }
}

impl Timer for TokioTimer {
    /// Returns the private same-domain Tokio clock handle.
    ///
    /// # Returns
    ///
    /// The monotonic clock driving this timer.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Creates a Tokio sleep with a fixed absolute deadline.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Deadline in this timer's clock domain.
    ///
    /// # Returns
    ///
    /// A future waiting for the fixed deadline, or an immediately ready future
    /// for a reached deadline in the retained runtime's time domain. If the
    /// retained runtime shuts down before a pending future completes, that
    /// future returns [`TimerUnavailableError::RuntimeShuttingDown`].
    ///
    /// # Errors
    ///
    /// Returns a domain mismatch or instant overflow before runtime access.
    /// Returns [`TimerUnavailableError::TimeDriverDisabled`] when a future
    /// deadline requires a time driver that the retained runtime did not
    /// enable. Reached deadlines do not require a time driver.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let deadline = self.native_deadline(deadline)?;
        self.clock
            .with_runtime(|| Self::schedule(deadline, Instant::now()))
    }

    /// Registers a notification after a duration in the retained Tokio
    /// runtime.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration from the retained runtime's current instant.
    ///
    /// # Returns
    ///
    /// A future that becomes ready when the fixed deadline is reached.
    /// If the retained runtime shuts down before that happens, the future
    /// returns [`TimerUnavailableError::RuntimeShuttingDown`].
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when the relative deadline cannot
    /// be represented, or [`TimerUnavailableError::TimeDriverDisabled`] when a
    /// nonzero future deadline requires a disabled time driver.
    #[inline]
    fn after(&self, duration: Duration) -> Result<TimerFuture, TimeError> {
        self.clock.with_runtime(|| {
            let now = Instant::now();
            let deadline = now
                .checked_add(duration)
                .ok_or(TimeError::InstantOverflow)?;
            Self::schedule(deadline, now)
        })
    }
}
