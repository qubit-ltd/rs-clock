// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a standard-library blocking sleeper.

use crate::{
    BlockingSleeper,
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
    TimeError,
};
use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    Instant,
};

/// A blocking sleeper paired with one explicit [`StdMonotonicClock`].
#[derive(Debug)]
pub struct StdBlockingSleeper {
    clock: Arc<StdMonotonicClock>,
}

impl StdBlockingSleeper {
    /// Creates a sleeper in the supplied clock's monotonic domain.
    #[must_use]
    pub const fn from_clock(clock: Arc<StdMonotonicClock>) -> Self {
        Self { clock }
    }

    /// Converts a domain-scoped deadline into its native standard instant.
    ///
    /// Returns a domain mismatch or overflow error when conversion is not
    /// possible.
    fn native_deadline(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<Instant, TimeError> {
        deadline.ensure_domain(self.clock.domain_id())?;
        self.clock
            .origin()
            .checked_add(deadline.elapsed_since_origin())
            .ok_or(TimeError::InstantOverflow)
    }
}

impl MonotonicClock for StdBlockingSleeper {
    /// Delegates domain identity to the explicitly supplied clock.
    fn domain_id(&self) -> u64 {
        MonotonicClock::domain_id(self.clock.as_ref())
    }

    /// Delegates elapsed time to the explicitly supplied clock.
    fn elapsed_since_origin(&self) -> Duration {
        self.clock.elapsed_since_origin()
    }
}

impl BlockingSleeper for StdBlockingSleeper {
    /// Blocks the current thread until the native deadline is reached.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        let deadline = self.native_deadline(deadline)?;
        loop {
            let now = Instant::now();
            let Some(remaining) = deadline.checked_duration_since(now) else {
                return Ok(());
            };
            thread::sleep(remaining);
        }
    }
}
