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
    clock: Arc<ManualMonotonicClock>,
}

impl ManualBlockingSleeper {
    /// Creates a blocking sleeper in the supplied manual clock domain.
    #[must_use]
    #[inline(always)]
    pub const fn from_clock(clock: Arc<ManualMonotonicClock>) -> Self {
        Self { clock }
    }
}

impl BlockingSleeper for ManualBlockingSleeper {
    /// Returns the manual clock driving this sleeper.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Blocks until explicit manual time reaches `deadline`.
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
