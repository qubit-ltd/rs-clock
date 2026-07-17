// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the asynchronous timer capability.

use crate::{
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    TimerFuture,
};
use std::time::Duration;

/// Registers asynchronous notifications in one monotonic clock domain.
///
/// Calling [`at()`](Self::at) or [`after()`](Self::after) performs timer
/// registration before returning. The returned future therefore only
/// represents completion, never deferred registration. It remains ready once
/// the deadline has been reached, including when completion happens before the
/// future is first polled. Dropping an incomplete future cancels its
/// registration.
///
/// Implementations must reject deadlines from a different clock domain with
/// [`TimeError::ClockDomainMismatch`].
pub trait Timer: Send + Sync {
    /// Returns the monotonic clock whose domain this timer uses.
    ///
    /// # Returns
    ///
    /// The clock retained by this timer.
    ///
    /// # Examples
    ///
    /// Discarding the retained clock is diagnosed when unused results are
    /// denied:
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_clock::{MonotonicClock, StdMonotonicClock, Timer};
    ///
    /// let timer = StdMonotonicClock::new().new_timer();
    /// timer.clock();
    /// ```
    #[must_use = "the Timer clock should be used to sample or validate deadlines"]
    fn clock(&self) -> &dyn MonotonicClock;

    /// Registers a notification for an absolute monotonic deadline.
    ///
    /// Registration is complete before this method returns. A deadline at or
    /// before the current time produces a future that is already ready.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline in this timer's clock domain.
    ///
    /// # Returns
    ///
    /// A future that becomes ready when `deadline` is reached.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] when `deadline` belongs to a
    /// different clock domain. Returns another [`TimeError`] when registration
    /// cannot be completed.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError>;

    /// Registers a notification after a relative duration.
    ///
    /// The deadline is fixed by sampling [`clock()`](Self::clock) during this
    /// call, not when the returned future is first polled.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration from the current monotonic instant.
    ///
    /// # Returns
    ///
    /// A future that becomes ready when the fixed deadline is reached.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when the deadline cannot be
    /// represented. Returns any error produced while registering that
    /// deadline.
    #[inline]
    fn after(&self, duration: Duration) -> Result<TimerFuture, TimeError> {
        let deadline = self.clock().now().checked_add(duration)?;
        self.at(deadline)
    }
}

impl<T> Timer for std::sync::Arc<T>
where
    T: Timer + ?Sized,
{
    /// Delegates access to the shared timer's clock.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates absolute deadline registration to the shared timer.
    #[inline(always)]
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        self.as_ref().at(deadline)
    }
}

impl<T> Timer for Box<T>
where
    T: Timer + ?Sized,
{
    /// Delegates access to the boxed timer's clock.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates absolute deadline registration to the boxed timer.
    #[inline(always)]
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        self.as_ref().at(deadline)
    }
}
