// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines blocking monotonic sleep operations.

use crate::{
    MonotonicClock,
    MonotonicInstant,
    TimeError,
};
use std::time::Duration;

/// Provides blocking waits in the implementor's monotonic clock domain.
///
/// The clock returned by [`Self::clock`] defines the domain accepted by every
/// deadline operation. Repeated calls to [`Self::clock`] must expose clocks
/// whose instants belong to the same domain for this sleeper's lifetime.
pub trait BlockingSleeper: Send + Sync {
    /// Returns the monotonic clock paired with this sleeper.
    ///
    /// # Returns
    ///
    /// The paired clock. Its domain remains stable for this sleeper's entire
    /// lifetime.
    #[must_use = "the paired clock should be used to sample a deadline"]
    fn clock(&self) -> &dyn MonotonicClock;

    /// Blocks the current thread until `deadline` is reached.
    ///
    /// A reached deadline completes immediately.
    ///
    /// # Parameters
    ///
    /// * `deadline` - The instant to wait for. It must belong to the stable
    ///   domain exposed by [`Self::clock`].
    ///
    /// # Returns
    ///
    /// `Ok(())` after `deadline` is reached.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline. An
    /// implementation may return [`TimeError::InstantOverflow`] when its
    /// native timer cannot represent `deadline`.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError>;

    /// Blocks the current thread for `duration` in this sleeper's domain.
    ///
    /// The deadline is calculated when this method is called.
    ///
    /// # Parameters
    ///
    /// * `duration` - The amount of monotonic time to wait.
    ///
    /// # Returns
    ///
    /// `Ok(())` after `duration` has elapsed.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when the computed deadline is
    /// not representable. Errors from [`Self::sleep_until`] are otherwise
    /// propagated.
    #[inline]
    fn sleep_for(&self, duration: Duration) -> Result<(), TimeError> {
        let deadline = self.clock().now().checked_add(duration)?;
        self.sleep_until(deadline)
    }
}

impl<T> BlockingSleeper for std::sync::Arc<T>
where
    T: BlockingSleeper + ?Sized,
{
    /// Delegates the paired clock to the shared sleeper object.
    ///
    /// # Returns
    ///
    /// The monotonic clock exposed by the wrapped sleeper.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates the blocking wait to the shared sleeper object.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Domain-scoped deadline forwarded to the wrapped sleeper.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the wrapped sleeper reaches the deadline.
    ///
    /// # Errors
    ///
    /// Returns any [`TimeError`] produced by the wrapped sleeper.
    #[inline(always)]
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        self.as_ref().sleep_until(deadline)
    }
}

impl<T> BlockingSleeper for Box<T>
where
    T: BlockingSleeper + ?Sized,
{
    /// Delegates the paired clock to the boxed sleeper object.
    ///
    /// # Returns
    ///
    /// The monotonic clock exposed by the wrapped sleeper.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates the blocking wait to the boxed sleeper object.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Domain-scoped deadline forwarded to the wrapped sleeper.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the wrapped sleeper reaches the deadline.
    ///
    /// # Errors
    ///
    /// Returns any [`TimeError`] produced by the wrapped sleeper.
    #[inline(always)]
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        self.as_ref().sleep_until(deadline)
    }
}
