// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a blocking sleeper driven by manual monotonic time.

use crate::{
    BlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
};
use std::sync::Arc;

/// A blocking sleeper paired with one explicit manual monotonic clock.
#[derive(Debug)]
pub struct ManualBlockingSleeper {
    /// Shared manual clock that owns this sleeper's deadline waiters.
    clock: Arc<ManualMonotonicClock>,
}

impl ManualBlockingSleeper {
    /// Creates a blocking sleeper in the supplied manual clock domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Shared manual clock that owns this sleeper's waiters.
    ///
    /// # Returns
    ///
    /// A blocking sleeper paired with the exact supplied clock.
    #[must_use]
    #[inline(always)]
    pub const fn from_clock(clock: Arc<ManualMonotonicClock>) -> Self {
        Self { clock }
    }
}

impl BlockingSleeper for ManualBlockingSleeper {
    /// Returns the manual clock driving this sleeper.
    ///
    /// # Returns
    ///
    /// The paired manual clock as a monotonic-clock trait object.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Blocks until explicit manual time reaches `deadline`.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Instant to wait for in the paired manual clock domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` after manual time reaches the deadline.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a deadline from another
    /// clock domain.
    ///
    /// # Panics
    ///
    /// Panics after attempting every reached waiter-observer waker if one of
    /// those custom wakers panics. The blocking waiter is unregistered while
    /// the panic unwinds.
    #[inline(always)]
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        self.clock.wait_until_blocking(deadline)
    }
}
