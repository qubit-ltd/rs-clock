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
    /// the future is first polled. Dropping an incomplete future unregisters
    /// it.
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        Box::pin(ManualSleepFuture::new(Arc::clone(&self.clock), deadline))
    }
}
