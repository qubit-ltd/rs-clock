// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
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
/// All active futures created by one timer share a single lazily started
/// scheduler worker. The worker exits when no active registrations remain.
pub struct StdTimer {
    /// Private clock handle retaining the source domain and native origin.
    clock: Arc<StdMonotonicClock>,
    /// Lazy scheduler shared by every registration from this timer.
    scheduler: Arc<StdTimerScheduler>,
}

impl StdTimer {
    /// Creates a timer sharing the supplied standard clock's exact domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Standard clock whose domain and origin drive this timer.
    ///
    /// # Returns
    ///
    /// A timer with an initially idle scheduler.
    #[must_use]
    #[inline]
    pub fn from_clock(clock: &StdMonotonicClock) -> Self {
        Self {
            clock: Arc::new(clock.same_domain_handle()),
            scheduler: Arc::new(StdTimerScheduler::new()),
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
        self.clock.as_ref()
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
    /// immediately ready future for a reached deadline.
    ///
    /// # Errors
    ///
    /// Returns a domain mismatch, native-instant overflow, or scheduler startup
    /// error before returning a future.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let deadline = self.native_deadline(deadline)?;
        if deadline <= Instant::now() {
            return Ok(Box::pin(std::future::ready(())));
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
