// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a timer driven by explicitly advanced monotonic time.

use std::sync::Arc;

use crate::ManualMonotonicClock;
use crate::MonotonicClock;
use crate::MonotonicInstant;
use crate::TimeError;
use crate::Timer;
use crate::TimerFuture;
use crate::timer::internal::manual_timer_future::ManualTimerFuture;

/// An asynchronous timer driven by one manual monotonic time domain.
///
/// Registrations are visible through the source clock's coordination APIs
/// before this timer returns their futures. The timer and its futures retain a
/// private same-domain clock handle, so they remain valid if the source clock
/// value is dropped.
#[derive(Debug)]
pub struct ManualTimer {
    /// Private handle retaining the manual clock domain and mutable timeline.
    clock: Arc<ManualMonotonicClock>,
}

impl ManualTimer {
    /// Creates a timer sharing the supplied manual clock's exact time domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Manual clock whose domain and timeline drive this timer.
    ///
    /// # Returns
    ///
    /// An independent timer handle retaining the same manual time domain.
    #[must_use]
    #[inline]
    pub fn from_clock(clock: &ManualMonotonicClock) -> Self {
        Self {
            clock: Arc::new(clock.same_domain_handle()),
        }
    }
}

impl Timer for ManualTimer {
    /// Returns the private same-domain manual clock handle.
    ///
    /// # Returns
    ///
    /// The monotonic clock driving this timer.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Eagerly registers an absolute deadline with the manual clock.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Deadline in this timer's manual clock domain.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future whose registration is already active.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline.
    ///
    /// # Panics
    ///
    /// Panics when waiter identifiers are exhausted or when a reached
    /// observer waker panics during registration notification.
    #[inline]
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let future = ManualTimerFuture::register(Arc::clone(&self.clock), deadline)?;
        Ok(Box::pin(future))
    }
}
