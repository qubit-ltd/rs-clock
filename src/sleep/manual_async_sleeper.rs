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
use std::time::Duration;

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

impl MonotonicClock for ManualAsyncSleeper {
    /// Delegates domain identity to the explicitly supplied manual clock.
    fn domain_id(&self) -> u64 {
        MonotonicClock::domain_id(self.clock.as_ref())
    }

    /// Delegates elapsed time to the explicitly supplied manual clock.
    fn elapsed_since_origin(&self) -> Duration {
        self.clock.elapsed_since_origin()
    }
}

impl AsyncSleeper for ManualAsyncSleeper {
    /// Creates a cancellation-safe future in the manual clock domain.
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        Box::pin(ManualSleepFuture::new(Arc::clone(&self.clock), deadline))
    }
}
