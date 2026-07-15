// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines an async sleeper driven by manual monotonic time.

use crate::sleep::internal::manual_sleep_future::ManualSleepFuture;
use crate::{
    AsyncSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
    SleepFuture,
};
use std::sync::Arc;

/// An async sleeper paired with one explicit manual monotonic clock.
///
/// Each sleep waiter is registered before the sleep method returns, so an
/// unpolled future is visible through the clock's waiter coordination methods.
/// Advancing to its deadline before the first poll makes that first poll ready.
/// Dropping an incomplete future unregisters its waiter.
#[derive(Debug)]
pub struct ManualAsyncSleeper {
    /// Shared manual clock that owns this sleeper's deadline waiters.
    clock: Arc<ManualMonotonicClock>,
}

impl ManualAsyncSleeper {
    /// Creates an async sleeper in the supplied manual clock domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Shared manual clock that owns this sleeper's waiters.
    ///
    /// # Returns
    ///
    /// An async sleeper paired with the exact supplied clock.
    #[must_use]
    #[inline(always)]
    pub const fn from_clock(clock: Arc<ManualMonotonicClock>) -> Self {
        Self { clock }
    }
}

impl AsyncSleeper for ManualAsyncSleeper {
    /// Returns the manual clock driving this sleeper.
    ///
    /// # Returns
    ///
    /// The paired manual clock as a monotonic-clock trait object.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Creates a cancellation-safe future in the manual clock domain.
    ///
    /// The waiter is registered before this method returns, rather than when
    /// the future is first polled. A relative sleep has already fixed its
    /// deadline when the default [`AsyncSleeper::sleep_for_async`]
    /// implementation calls this method. Dropping an incomplete future
    /// unregisters its waiter; a foreign deadline is returned through an
    /// immediately ready error future.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Instant to await in the paired manual clock domain.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future registered before this method returns.
    ///
    /// # Errors
    ///
    /// The future resolves to [`crate::TimeError::ClockDomainMismatch`] for a
    /// deadline from another clock domain.
    ///
    /// # Panics
    ///
    /// Panics after attempting every reached waiter-observer waker if one of
    /// those custom wakers panics. The new waiter is unregistered while the
    /// panic unwinds.
    #[inline]
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        Box::pin(ManualSleepFuture::new(Arc::clone(&self.clock), deadline))
    }
}
