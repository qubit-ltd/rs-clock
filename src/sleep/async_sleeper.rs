// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines asynchronous monotonic sleep operations.

use crate::{
    MonotonicClock,
    MonotonicInstant,
    SleepFuture,
    TimeError,
};
use std::time::Duration;

/// Provides asynchronous waits in the implementor's monotonic clock domain.
pub trait AsyncSleeper: Send + Sync {
    /// Returns the monotonic clock paired with this sleeper.
    fn clock(&self) -> &dyn MonotonicClock;

    /// Returns a future completing when `deadline` is reached.
    ///
    /// A reached deadline completes immediately. A foreign deadline resolves
    /// to [`TimeError::ClockDomainMismatch`]. The returned future owns its
    /// waiting state and does not borrow this sleeper.
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture;

    /// Returns a future completing after `duration` in this sleeper's domain.
    ///
    /// The deadline is calculated when this method is called, before the
    /// returned future is first polled. The future owns its waiting state and
    /// has a `'static` lifetime.
    fn sleep_for_async(&self, duration: Duration) -> SleepFuture {
        match self.clock().now().checked_add(duration) {
            Ok(deadline) => self.sleep_until_async(deadline),
            Err(error) => ready_sleep_result(Err(error)),
        }
    }
}

impl<T> AsyncSleeper for std::sync::Arc<T>
where
    T: AsyncSleeper + ?Sized,
{
    /// Delegates the paired clock to the shared sleeper object.
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates asynchronous waiting to the shared sleeper object.
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        self.as_ref().sleep_until_async(deadline)
    }
}

impl<T> AsyncSleeper for Box<T>
where
    T: AsyncSleeper + ?Sized,
{
    /// Delegates the paired clock to the boxed sleeper object.
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates asynchronous waiting to the boxed sleeper object.
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        self.as_ref().sleep_until_async(deadline)
    }
}

/// Creates an immediately ready sleep future for a precomputed result.
pub(crate) fn ready_sleep_result(result: Result<(), TimeError>) -> SleepFuture {
    Box::pin(async move { result })
}
