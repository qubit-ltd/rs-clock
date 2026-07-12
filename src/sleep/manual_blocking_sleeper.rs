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
use std::time::Duration;

/// A blocking sleeper paired with one explicit manual monotonic clock.
#[derive(Debug)]
pub struct ManualBlockingSleeper {
    clock: Arc<ManualMonotonicClock>,
}

impl ManualBlockingSleeper {
    /// Creates a blocking sleeper in the supplied manual clock domain.
    #[must_use]
    pub const fn from_clock(clock: Arc<ManualMonotonicClock>) -> Self {
        Self { clock }
    }
}

impl MonotonicClock for ManualBlockingSleeper {
    /// Delegates domain identity to the explicitly supplied manual clock.
    fn domain_id(&self) -> u64 {
        MonotonicClock::domain_id(self.clock.as_ref())
    }

    /// Delegates elapsed time to the explicitly supplied manual clock.
    fn elapsed_since_origin(&self) -> Duration {
        self.clock.elapsed_since_origin()
    }
}

impl BlockingSleeper for ManualBlockingSleeper {
    /// Blocks until explicit manual time reaches `deadline`.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        self.clock.wait_until_blocking(deadline)
    }
}
