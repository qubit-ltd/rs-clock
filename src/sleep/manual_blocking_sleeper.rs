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

    /// Returns the number of threads currently waiting through this domain.
    #[must_use]
    pub fn pending_waiters(&self) -> usize {
        self.clock.pending_blocking_waiters()
    }

    /// Returns the earliest pending blocking deadline.
    ///
    /// `None` means no blocking sleeper is currently registered.
    #[must_use]
    pub fn next_deadline(&self) -> Option<MonotonicInstant> {
        self.clock.next_blocking_deadline()
    }

    /// Waits in real time until at least `expected_count` waiters register.
    ///
    /// This method is a test coordination guard only; `real_timeout` never
    /// contributes to manual time. Returns `true` when the count is reached
    /// and `false` when the real-time guard expires first.
    #[must_use]
    pub fn wait_for_waiters(
        &self,
        expected_count: usize,
        real_timeout: Duration,
    ) -> bool {
        self.clock
            .wait_for_blocking_waiters(expected_count, real_timeout)
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
        deadline.ensure_domain(self.clock.domain_id())?;
        self.clock.wait_until_blocking(deadline)
    }
}
