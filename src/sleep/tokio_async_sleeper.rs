// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines an async sleeper backed by Tokio's time driver.

use crate::sleep::async_sleeper::ready_sleep_result;
use crate::{
    AsyncSleeper,
    MonotonicClock,
    MonotonicInstant,
    SleepFuture,
    TimeError,
    TokioMonotonicClock,
};
use std::sync::Arc;
use tokio::time::Instant;

/// An async sleeper paired with one explicit [`TokioMonotonicClock`].
#[derive(Debug)]
pub struct TokioAsyncSleeper {
    clock: Arc<TokioMonotonicClock>,
}

impl TokioAsyncSleeper {
    /// Creates a Tokio sleeper in the supplied clock domain.
    #[must_use]
    pub const fn from_clock(clock: Arc<TokioMonotonicClock>) -> Self {
        Self { clock }
    }

    /// Converts a domain-scoped deadline into a Tokio instant.
    ///
    /// Returns a domain mismatch or overflow error when conversion is not
    /// possible.
    fn native_deadline(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<Instant, TimeError> {
        deadline.ensure_domain(self.clock.now().domain())?;
        self.clock
            .origin()
            .checked_add(deadline.elapsed_since_origin())
            .ok_or(TimeError::InstantOverflow)
    }
}

impl AsyncSleeper for TokioAsyncSleeper {
    /// Returns the Tokio monotonic clock driving this sleeper.
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Returns a future driven by Tokio's time driver.
    ///
    /// The native Tokio timer is created when the returned future is first
    /// polled, so creating the future itself does not require a Tokio runtime.
    ///
    /// # Panics
    ///
    /// The returned future panics when first polled without a Tokio runtime
    /// whose time driver is enabled.
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        let deadline = match self.native_deadline(deadline) {
            Ok(deadline) => deadline,
            Err(error) => return ready_sleep_result(Err(error)),
        };
        Box::pin(async move {
            tokio::time::sleep_until(deadline).await;
            Ok(())
        })
    }
}
