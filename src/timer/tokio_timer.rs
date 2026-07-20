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
use std::time::Duration;
use tokio::time::Instant;

/// An asynchronous timer backed by one Tokio runtime time driver.
///
/// The timer retains the source clock's exact domain, origin, and runtime
/// binding. Registration validates that runtime before reading Tokio time, so
/// both reached and future deadlines reject a missing or independent runtime.
/// The timer fixes each native Tokio deadline before [`Timer::at`] returns;
/// Tokio may enroll the resulting sleep with its time driver on first poll.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioTimer {
    /// Private handle retaining the source clock domain and Tokio origin.
    clock: TokioMonotonicClock,
}

impl TokioTimer {
    /// Creates a timer bound to the currently entered Tokio runtime.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain bound to the current runtime.
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

    /// Tries to create a timer bound to the currently entered Tokio runtime.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain bound to the current runtime.
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
    /// * `clock` - Tokio clock whose domain, origin, and runtime binding apply.
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
    /// for a reached deadline in the bound runtime. Tokio may register the
    /// sleep on first poll.
    ///
    /// # Errors
    ///
    /// Returns a domain mismatch or instant overflow before runtime access.
    /// Returns [`TimeError::TimerUnavailable`] with
    /// [`TimerUnavailableError::TokioRuntime`] when no runtime is entered or
    /// an independent runtime is entered, or
    /// [`TimerUnavailableError::TimeDriverDisabled`] when the bound runtime's
    /// time driver is disabled. Runtime validation applies to reached and
    /// future deadlines alike.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let deadline = self.native_deadline(deadline)?;
        self.clock.ensure_current_runtime().map_err(|source| {
            TimeError::TimerUnavailable {
                source: TimerUnavailableError::TokioRuntime { source },
            }
        })?;
        if deadline <= Instant::now() {
            return Ok(Box::pin(std::future::ready(())));
        }
        let sleep =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::time::sleep_until(deadline)
            }))
            .map_err(|_| TimeError::TimerUnavailable {
                source: TimerUnavailableError::TimeDriverDisabled,
            })?;
        Ok(Box::pin(async move {
            sleep.await;
        }))
    }

    /// Registers a notification after a duration in the bound Tokio runtime.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration from the current bound-runtime instant.
    ///
    /// # Returns
    ///
    /// A future that becomes ready when the fixed deadline is reached.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::TimerUnavailable`] with
    /// [`TimerUnavailableError::TokioRuntime`] when no runtime is entered or
    /// an independent runtime is entered. Returns
    /// [`TimeError::InstantOverflow`] when the relative deadline cannot be
    /// represented, or another timer-unavailability error when registration
    /// fails.
    #[inline]
    fn after(&self, duration: Duration) -> Result<TimerFuture, TimeError> {
        let now = self.clock.try_now().map_err(|source| {
            TimeError::TimerUnavailable {
                source: TimerUnavailableError::TokioRuntime { source },
            }
        })?;
        self.at(now.checked_add(duration)?)
    }
}
