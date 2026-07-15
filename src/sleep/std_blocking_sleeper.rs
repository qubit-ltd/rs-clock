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
use std::time::Instant;

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
        deadline.ensure_domain(self.clock.domain())?;
        self.clock
            .origin()
            .checked_add(deadline.elapsed_since_origin())
            .ok_or(TimeError::InstantOverflow)
    }
}

impl BlockingSleeper for StdBlockingSleeper {
    /// Returns the standard monotonic clock driving this sleeper.
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Blocks the current thread until the native deadline is reached.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        let deadline = self.native_deadline(deadline)?;
        let now = Instant::now();
        if let Some(remaining) = deadline.checked_duration_since(now) {
            thread::sleep(remaining);
        }
        Ok(())
    }
}
