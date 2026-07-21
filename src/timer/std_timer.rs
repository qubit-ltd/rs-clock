// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a standard-library timer with one shared scheduler worker.

use crate::timer::internal::std_timer_future::StdTimerFuture;
use crate::timer::internal::std_timer_scheduler::StdTimerScheduler;
use crate::timer::internal::std_timer_waiter::StdTimerWaiter;
use crate::{
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
    TimeError,
    Timer,
    TimerFuture,
};
use std::sync::Arc;
use std::time::Instant;

/// A real-time asynchronous timer backed by [`std::time::Instant`].
///
/// Every standard Timer in the process shares one scheduler worker. The worker
/// starts lazily with the first future registration and remains parked while
/// idle so later registrations do not need to create another native thread.
/// If that worker exits unexpectedly, its active futures are awakened and
/// return [`TimeError::TimerUnavailable`] with
/// [`TimerUnavailableError::SchedulerWorkerTerminated`](crate::TimerUnavailableError::SchedulerWorkerTerminated)
/// instead of remaining pending or reporting false deadline completion. A
/// later registration starts a replacement worker generation.
pub struct StdTimer {
    /// Private clock handle retaining the source domain and native origin.
    clock: StdMonotonicClock,
    /// Process-wide scheduler shared by every standard Timer registration.
    scheduler: Arc<StdTimerScheduler>,
}

impl StdTimer {
    /// Creates a timer backed by a new standard monotonic clock.
    ///
    /// # Returns
    ///
    /// A timer with a fresh clock domain and the process-wide scheduler.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        let clock = StdMonotonicClock::new();
        Self::from_clock(&clock)
    }

    /// Creates a timer sharing the supplied standard clock's exact domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Standard clock whose domain and origin drive this timer.
    ///
    /// # Returns
    ///
    /// A timer retaining the process-wide lazy scheduler.
    #[must_use]
    #[inline]
    pub fn from_clock(clock: &StdMonotonicClock) -> Self {
        Self {
            clock: clock.same_domain_handle(),
            scheduler: StdTimerScheduler::shared(),
        }
    }

    /// Converts a domain-scoped deadline to its native standard instant.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline in the source clock domain.
    ///
    /// # Returns
    ///
    /// The corresponding native instant.
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

impl Default for StdTimer {
    /// Creates a standard timer with a fresh clock domain.
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for StdTimer {
    /// Formats the retained clock without exposing scheduler internals.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination formatter.
    ///
    /// # Returns
    ///
    /// `Ok(())` when formatting succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`std::fmt::Error`] when the destination rejects output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StdTimer")
            .field("clock", &self.clock)
            .finish_non_exhaustive()
    }
}

impl Timer for StdTimer {
    /// Returns the private same-domain standard clock handle.
    ///
    /// # Returns
    ///
    /// The monotonic clock driving this timer.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Eagerly registers an absolute deadline with the shared scheduler.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Deadline in this timer's clock domain.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future whose registration is already active, or an
    /// immediately ready future for a reached deadline. The future returns
    /// [`TimeError::TimerUnavailable`] with
    /// [`TimerUnavailableError::SchedulerWorkerTerminated`](crate::TimerUnavailableError::SchedulerWorkerTerminated)
    /// when its scheduler worker exits unexpectedly.
    ///
    /// # Errors
    ///
    /// Returns a domain mismatch, native-instant overflow, or scheduler startup
    /// error before returning a future.
    ///
    /// # Panics
    ///
    /// Panics when scheduler registration identifiers or worker generations
    /// are exhausted, or an internal scheduler index invariant is violated.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let deadline = self.native_deadline(deadline)?;
        if deadline <= Instant::now() {
            return Ok(Box::pin(std::future::ready(Ok(()))));
        }
        let waiter = Arc::new(StdTimerWaiter::new());
        let waiter_id =
            self.scheduler.register(deadline, Arc::clone(&waiter))?;
        Ok(Box::pin(StdTimerFuture::new(
            Arc::clone(&self.scheduler),
            waiter_id,
            waiter,
        )))
    }
}
