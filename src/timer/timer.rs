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

/// Creates asynchronous notifications in one monotonic clock domain.
///
/// Calling [`at()`](Self::at) or [`after()`](Self::after) fixes the logical
/// deadline and cancellation ownership before returning. The returned future
/// waits for that fixed deadline. If the deadline is reached before the first
/// poll, that first poll returns ready. As with every [`Future`], callers must
/// not poll it again after it first returns ready. A backend may defer
/// enrollment with its native scheduler until the future is polled. Dropping an
/// incomplete future cancels the outstanding notification.
///
/// Every call to [`clock()`](Self::clock) on one Timer must report the same
/// clock domain for the Timer's lifetime. Implementations must reject deadlines
/// from a different domain with [`TimeError::ClockDomainMismatch`].
///
/// Timer failures have two stages: the outer [`Result`] reports registration
/// failures, while the returned [`TimerFuture`] reports failures observed after
/// registration, such as an unavailable scheduler worker or a Tokio runtime
/// that shut down. Custom implementations may document additional lifecycle
/// preconditions and panic conditions.
pub trait Timer: Send + Sync {
    /// Returns the monotonic clock whose domain this timer uses.
    ///
    /// Successive calls may return different handles, but every returned clock
    /// must report the same domain for this Timer's lifetime.
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

    /// Returns the current monotonic instant in this timer's clock domain.
    ///
    /// # Returns
    ///
    /// The current instant sampled from this timer's clock.
    #[must_use = "the current timer instant should be used to measure or validate deadlines"]
    #[inline(always)]
    fn now(&self) -> MonotonicInstant {
        self.clock().now()
    }

    /// Creates a notification for an absolute monotonic deadline.
    ///
    /// The deadline is fixed before this method returns. A deadline at or
    /// before the current time produces a future that is already ready.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline in this timer's clock domain.
    ///
    /// # Returns
    ///
    /// A future that returns `Ok(())` when `deadline` is reached. The future
    /// returns a [`TimeError`] if the backend fails after registration.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] when `deadline` belongs to a
    /// different clock domain. Returns another [`TimeError`] when the
    /// notification cannot be created.
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
    /// A future that returns `Ok(())` when the fixed deadline is reached. The
    /// future returns a [`TimeError`] if the backend later fails.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when the deadline cannot be
    /// represented. Returns any error produced while creating the notification
    /// for that deadline.
    #[inline]
    fn after(&self, duration: Duration) -> Result<TimerFuture, TimeError> {
        let deadline = self.now().checked_add(duration)?;
        self.at(deadline)
    }
}

impl<T> Timer for std::sync::Arc<T>
where
    T: Timer + ?Sized,
{
    /// Delegates access to the shared timer's clock.
    ///
    /// # Returns
    ///
    /// The clock exposed by the wrapped timer.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates absolute deadline registration to the shared timer.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline in the wrapped timer's clock domain.
    ///
    /// # Returns
    ///
    /// The wrapped timer's cancellation-safe completion future.
    ///
    /// # Errors
    ///
    /// Returns any registration error reported by the wrapped timer.
    #[inline(always)]
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        self.as_ref().at(deadline)
    }

    /// Delegates relative deadline registration to the shared timer.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration from the wrapped timer's current instant.
    ///
    /// # Returns
    ///
    /// The wrapped timer's cancellation-safe completion future.
    ///
    /// # Errors
    ///
    /// Returns any registration error reported by the wrapped timer.
    #[inline(always)]
    fn after(&self, duration: Duration) -> Result<TimerFuture, TimeError> {
        self.as_ref().after(duration)
    }
}

impl<T> Timer for Box<T>
where
    T: Timer + ?Sized,
{
    /// Delegates access to the boxed timer's clock.
    ///
    /// # Returns
    ///
    /// The clock exposed by the wrapped timer.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates absolute deadline registration to the boxed timer.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline in the wrapped timer's clock domain.
    ///
    /// # Returns
    ///
    /// The wrapped timer's cancellation-safe completion future.
    ///
    /// # Errors
    ///
    /// Returns any registration error reported by the wrapped timer.
    #[inline(always)]
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        self.as_ref().at(deadline)
    }

    /// Delegates relative deadline registration to the boxed timer.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration from the wrapped timer's current instant.
    ///
    /// # Returns
    ///
    /// The wrapped timer's cancellation-safe completion future.
    ///
    /// # Errors
    ///
    /// Returns any registration error reported by the wrapped timer.
    #[inline(always)]
    fn after(&self, duration: Duration) -> Result<TimerFuture, TimeError> {
        self.as_ref().after(duration)
    }
}
