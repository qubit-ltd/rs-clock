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
    TokioMonotonicClock,
};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::time::Instant;

/// An asynchronous timer backed by one Tokio runtime time driver.
///
/// The timer fixes each native Tokio deadline before [`Timer::at`] returns;
/// Tokio may enroll the resulting sleep with its time driver on first poll.
/// The timer retains the source clock's exact domain and origin. Future
/// deadlines must be created and polled under the same Tokio time driver;
/// reached deadlines return an immediately ready future without accessing a
/// runtime. Tokio does not expose a driver identity that this crate can
/// validate.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioTimer {
    /// Private handle retaining the source clock domain and Tokio origin.
    clock: Arc<TokioMonotonicClock>,
}

impl TokioTimer {
    /// Creates a timer sharing the supplied Tokio clock's exact domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Tokio clock whose domain, origin, and driver affinity apply.
    ///
    /// # Returns
    ///
    /// A timer retaining an independent same-domain clock handle.
    #[must_use]
    #[inline]
    pub fn from_clock(clock: &TokioMonotonicClock) -> Self {
        Self {
            clock: Arc::new(clock.same_domain_handle()),
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
        self.clock.as_ref()
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
    /// for a reached deadline. Tokio may register the sleep on first poll.
    ///
    /// # Errors
    ///
    /// Returns a domain mismatch or instant overflow before runtime access.
    /// For future deadlines, returns [`TimeError::TimerUnavailable`] when no
    /// runtime is entered or its time driver is disabled. Reached deadlines do
    /// not require runtime access.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let deadline = self.native_deadline(deadline)?;
        if deadline <= Instant::now() {
            return Ok(Box::pin(std::future::ready(())));
        }
        Handle::try_current().map_err(|_| TimeError::TimerUnavailable)?;
        let sleep =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::time::sleep_until(deadline)
            }))
            .map_err(|_| TimeError::TimerUnavailable)?;
        Ok(Box::pin(async move {
            sleep.await;
        }))
    }
}
