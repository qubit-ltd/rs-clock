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
    /// Shared standard clock used for deadline conversion and elapsed time.
    clock: Arc<StdMonotonicClock>,
}

impl StdBlockingSleeper {
    /// Creates a sleeper with a newly allocated standard clock domain.
    ///
    /// Use [`Self::from_clock`] when another component must share the exact
    /// monotonic clock identity.
    ///
    /// # Returns
    ///
    /// A blocking sleeper paired with its own standard monotonic clock.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::from_clock(Arc::new(StdMonotonicClock::new()))
    }

    /// Creates a sleeper in the supplied clock's monotonic domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Shared standard monotonic clock paired with the sleeper.
    ///
    /// # Returns
    ///
    /// A blocking sleeper using the exact supplied clock.
    #[must_use]
    #[inline(always)]
    pub const fn from_clock(clock: Arc<StdMonotonicClock>) -> Self {
        Self { clock }
    }

    /// Converts a domain-scoped deadline into its native standard instant.
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
    /// The corresponding standard-library instant.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline and
    /// [`TimeError::InstantOverflow`] when the native instant cannot represent
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

impl Default for StdBlockingSleeper {
    /// Creates a sleeper with a newly allocated standard clock domain.
    ///
    /// # Returns
    ///
    /// A blocking sleeper with the same behavior as [`Self::new`].
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl BlockingSleeper for StdBlockingSleeper {
    /// Returns the standard monotonic clock driving this sleeper.
    ///
    /// # Returns
    ///
    /// The paired standard clock as a monotonic-clock trait object.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    /// Blocks the current thread until the native deadline is reached.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Instant to wait for in the paired clock domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the deadline is reached.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline and
    /// [`TimeError::InstantOverflow`] when its native instant is not
    /// representable.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        let deadline = self.native_deadline(deadline)?;
        let now = Instant::now();
        if let Some(remaining) = deadline.checked_duration_since(now) {
            thread::sleep(remaining);
        }
        Ok(())
    }
}
