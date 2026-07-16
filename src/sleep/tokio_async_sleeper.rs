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
///
/// When Tokio time is paused or explicitly advanced, the paired clock must be
/// created and read, and this sleeper's futures must be polled, under the same
/// Tokio runtime time driver. Moving a task between threads of one runtime is
/// supported, but moving the pair between independent runtimes is not. This
/// contract is not checked at runtime.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioAsyncSleeper {
    /// Shared Tokio clock used for deadline conversion and elapsed time.
    clock: Arc<TokioMonotonicClock>,
}

impl TokioAsyncSleeper {
    /// Creates a sleeper with a newly allocated Tokio clock domain.
    ///
    /// Calling this method does not itself require a Tokio runtime. When using
    /// paused or explicitly advanced Tokio time, call it after entering the
    /// runtime that will poll the sleeper's futures. Use [`Self::from_clock`]
    /// when another component must share the exact monotonic clock identity.
    ///
    /// # Returns
    ///
    /// An async sleeper paired with its own Tokio monotonic clock.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::from_clock(Arc::new(TokioMonotonicClock::new()))
    }

    /// Creates a Tokio sleeper in the supplied clock domain.
    ///
    /// The caller must preserve the clock's Tokio time-driver affinity while
    /// using the returned sleeper.
    ///
    /// # Parameters
    ///
    /// * `clock` - Shared Tokio monotonic clock paired with the sleeper.
    ///
    /// # Returns
    ///
    /// An async sleeper using the exact supplied clock.
    #[must_use]
    #[inline(always)]
    pub const fn from_clock(clock: Arc<TokioMonotonicClock>) -> Self {
        Self { clock }
    }

    /// Converts a domain-scoped deadline into a Tokio instant.
    ///
    /// Returns a domain mismatch or overflow error when conversion is not
    /// possible.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Domain-scoped deadline to convert.
    ///
    /// # Returns
    ///
    /// The corresponding Tokio instant.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline and
    /// [`TimeError::InstantOverflow`] when the Tokio instant cannot represent
    /// it.
    #[inline]
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

impl Default for TokioAsyncSleeper {
    /// Creates a sleeper with a newly allocated Tokio clock domain.
    ///
    /// When using paused or explicitly advanced Tokio time, construct the
    /// default after entering the runtime that will poll its futures.
    ///
    /// # Returns
    ///
    /// An async sleeper with the same behavior as [`Self::new`].
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncSleeper for TokioAsyncSleeper {
    /// Returns the Tokio monotonic clock driving this sleeper.
    ///
    /// # Returns
    ///
    /// The paired Tokio clock as a monotonic-clock trait object.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Returns a future driven by Tokio's time driver.
    ///
    /// The native Tokio timer is created when the returned future is first
    /// polled, so creating the future itself does not require a Tokio runtime.
    /// The first poll must occur under a Tokio runtime with time enabled. When
    /// using paused or explicitly advanced time, that runtime must be the same
    /// one used to create and read the paired clock.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Instant to await in the paired Tokio clock domain.
    ///
    /// # Returns
    ///
    /// An owned future driven by Tokio's time driver.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimeError::ClockDomainMismatch`] for a foreign
    /// deadline or [`TimeError::InstantOverflow`] when conversion fails.
    ///
    /// # Panics
    ///
    /// The returned future panics when first polled without a Tokio runtime
    /// whose time driver is enabled.
    #[inline]
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
