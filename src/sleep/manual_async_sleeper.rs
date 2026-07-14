// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines an async sleeper driven by manual monotonic time.

use crate::sleep::manual_sleep_future::ManualSleepFuture;
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
    clock: Arc<ManualMonotonicClock>,
}

impl ManualAsyncSleeper {
    /// Creates an async sleeper in the supplied manual clock domain.
    #[must_use]
    pub const fn from_clock(clock: Arc<ManualMonotonicClock>) -> Self {
        Self { clock }
    }
}

impl AsyncSleeper for ManualAsyncSleeper {
    /// Returns the manual clock driving this sleeper.
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
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        Box::pin(ManualSleepFuture::new(Arc::clone(&self.clock), deadline))
    }
}
