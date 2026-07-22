// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a Timer that deterministically injects configured failures.

use super::TimerFailurePoint;
use crate::{
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    Timer,
    TimerFuture,
    TimerUnavailableError,
};
use std::{
    io,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

/// A deterministic Timer fixture that fails registration or completion.
///
/// The fixture owns a private manual clock domain. Foreign deadlines and
/// already reached deadlines retain the normal [`Timer`] contract; only valid
/// future deadlines reach the configured failure point.
///
/// # Examples
///
/// ```
/// use qubit_clock::{
///     Timer,
///     test_util::{FaultInjectingTimer, TimerFailurePoint},
/// };
/// use std::time::Duration;
///
/// let timer = FaultInjectingTimer::backend_unavailable(
///     TimerFailurePoint::Registration,
///     "example",
///     "backend offline",
/// );
/// assert!(timer.after(Duration::from_secs(1)).is_err());
/// assert_eq!(1, timer.registration_count());
/// ```
pub struct FaultInjectingTimer {
    /// Manual clock defining the fixture's private monotonic domain.
    clock: ManualMonotonicClock,
    /// Timer lifecycle point where the configured error is returned.
    failure_point: TimerFailurePoint,
    /// Thread-safe factory producing one fresh error per failed registration.
    error_factory: Box<dyn Fn() -> TimeError + Send + Sync + 'static>,
    /// Number of valid future-deadline registrations attempted by the fixture.
    registration_count: AtomicUsize,
}

impl FaultInjectingTimer {
    /// Creates a Timer that invokes `error_factory` at `failure_point`.
    ///
    /// # Parameters
    ///
    /// * `failure_point` - Registration or completion stage to fail.
    /// * `error_factory` - Thread-safe factory returning one fresh error for
    ///   every failed future-deadline registration.
    ///
    /// # Returns
    ///
    /// A fault-injecting Timer with a new private monotonic clock domain.
    ///
    /// # Panics
    ///
    /// Panics if process-wide clock-domain identifiers are exhausted.
    #[must_use]
    pub fn new<F>(failure_point: TimerFailurePoint, error_factory: F) -> Self
    where
        F: Fn() -> TimeError + Send + Sync + 'static,
    {
        Self {
            clock: ManualMonotonicClock::new(),
            failure_point,
            error_factory: Box::new(error_factory),
            registration_count: AtomicUsize::new(0),
        }
    }

    /// Creates a Timer reporting a custom backend-unavailable error.
    ///
    /// # Parameters
    ///
    /// * `failure_point` - Registration or completion stage to fail.
    /// * `backend` - Stable static name identifying the unavailable backend.
    /// * `message` - Error message copied into each fresh source error.
    ///
    /// # Returns
    ///
    /// A fault-injecting Timer producing
    /// [`TimerUnavailableError::BackendUnavailable`].
    ///
    /// # Panics
    ///
    /// Panics if process-wide clock-domain identifiers are exhausted.
    #[must_use]
    pub fn backend_unavailable(
        failure_point: TimerFailurePoint,
        backend: &'static str,
        message: &str,
    ) -> Self {
        let message = message.to_owned();
        Self::new(failure_point, move || TimeError::TimerUnavailable {
            source: TimerUnavailableError::BackendUnavailable {
                backend,
                source: Box::new(io::Error::other(message.clone())),
            },
        })
    }

    /// Returns the configured Timer lifecycle failure point.
    ///
    /// # Returns
    ///
    /// Registration or completion according to fixture construction.
    #[must_use]
    #[inline(always)]
    pub fn failure_point(&self) -> TimerFailurePoint {
        self.failure_point
    }

    /// Returns the number of valid future-deadline registrations attempted.
    ///
    /// Foreign and already reached deadlines do not increment this count.
    ///
    /// # Returns
    ///
    /// The current registration count. Concurrent observations are intended
    /// for test diagnostics and use relaxed ordering.
    #[must_use]
    #[inline(always)]
    pub fn registration_count(&self) -> usize {
        self.registration_count.load(Ordering::Relaxed)
    }
}

impl Timer for FaultInjectingTimer {
    /// Returns the fixture's private manual monotonic clock.
    ///
    /// # Returns
    ///
    /// The clock defining valid deadlines for this Timer.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Registers a future deadline and injects the configured failure.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline expected in this Timer's clock domain.
    ///
    /// # Returns
    ///
    /// An immediately ready successful future for a reached deadline, or a
    /// failing future when completion failure is configured.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline.
    /// Returns the error factory's value directly when registration failure is
    /// configured.
    ///
    /// # Panics
    ///
    /// Propagates a panic raised by the configured error factory.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let now = self.clock.now();
        deadline.ensure_domain(now.domain())?;
        if deadline <= now {
            return Ok(Box::pin(std::future::ready(Ok(()))));
        }
        self.registration_count.fetch_add(1, Ordering::Relaxed);
        let error = (self.error_factory)();
        match self.failure_point {
            TimerFailurePoint::Registration => Err(error),
            TimerFailurePoint::Completion => {
                Ok(Box::pin(std::future::ready(Err(error))))
            }
        }
    }
}
