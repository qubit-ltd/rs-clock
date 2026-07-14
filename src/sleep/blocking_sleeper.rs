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
pub trait BlockingSleeper: Send + Sync {
    /// Returns the monotonic clock paired with this sleeper.
    fn clock(&self) -> &dyn MonotonicClock;

    /// Blocks the current thread until `deadline` is reached.
    ///
    /// A reached deadline completes immediately. Returns
    /// [`TimeError::ClockDomainMismatch`] for a foreign deadline.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError>;

    /// Blocks the current thread for `duration` in this sleeper's domain.
    ///
    /// Returns [`TimeError::InstantOverflow`] when the computed deadline is
    /// not representable.
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
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates the blocking wait to the shared sleeper object.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        self.as_ref().sleep_until(deadline)
    }
}

impl<T> BlockingSleeper for Box<T>
where
    T: BlockingSleeper + ?Sized,
{
    /// Delegates the paired clock to the boxed sleeper object.
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates the blocking wait to the boxed sleeper object.
    fn sleep_until(&self, deadline: MonotonicInstant) -> Result<(), TimeError> {
        self.as_ref().sleep_until(deadline)
    }
}
