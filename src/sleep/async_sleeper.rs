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
///
/// The clock returned by [`Self::clock`] defines the domain accepted by every
/// deadline operation. Repeated calls to [`Self::clock`] must expose clocks
/// whose instants belong to the same domain for this sleeper's lifetime.
pub trait AsyncSleeper: Send + Sync {
    /// Returns the monotonic clock paired with this sleeper.
    ///
    /// # Returns
    ///
    /// The paired clock. Its domain remains stable for this sleeper's entire
    /// lifetime.
    fn clock(&self) -> &dyn MonotonicClock;

    /// Returns a future completing when `deadline` is reached.
    ///
    /// The returned future owns its waiting state and does not borrow this
    /// sleeper. A reached deadline completes immediately.
    ///
    /// # Parameters
    ///
    /// * `deadline` - The instant to wait for. It must belong to the stable
    ///   domain exposed by [`Self::clock`].
    ///
    /// # Returns
    ///
    /// A `'static` future that resolves successfully when `deadline` is
    /// reached.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimeError::ClockDomainMismatch`] for a foreign
    /// deadline. An implementation may return [`TimeError::InstantOverflow`]
    /// when its native timer cannot represent `deadline`.
    ///
    /// # Cancellation
    ///
    /// Dropping an incomplete future cancels the wait. Implementations must
    /// release waiter registrations and other resources owned solely by that
    /// future without requiring another poll.
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture;

    /// Returns a future completing after `duration` in this sleeper's domain.
    ///
    /// The deadline is calculated when this method is called, before the
    /// returned future is first polled. The future owns its waiting state and
    /// has a `'static` lifetime.
    ///
    /// # Parameters
    ///
    /// * `duration` - The amount of monotonic time to wait.
    ///
    /// # Returns
    ///
    /// A future that resolves successfully after `duration` has elapsed.
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimeError::InstantOverflow`] when the computed
    /// deadline is not representable. Errors from [`Self::sleep_until_async`]
    /// are otherwise propagated.
    ///
    /// # Cancellation
    ///
    /// Dropping an incomplete future has the cancellation semantics specified
    /// by [`Self::sleep_until_async`].
    #[inline]
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
    ///
    /// # Returns
    ///
    /// The monotonic clock exposed by the wrapped sleeper.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        self.as_ref().clock()
    }

    /// Delegates asynchronous waiting to the shared sleeper object.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Domain-scoped deadline forwarded to the wrapped sleeper.
    ///
    /// # Returns
    ///
    /// The owned sleep future returned by the wrapped sleeper.
    ///
    /// # Errors
    ///
    /// The future propagates any [`TimeError`] produced by the wrapped sleeper.
    #[inline(always)]
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        self.as_ref().sleep_until_async(deadline)
    }
}

impl<T> AsyncSleeper for Box<T>
where
    T: AsyncSleeper + ?Sized,
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

    /// Delegates asynchronous waiting to the boxed sleeper object.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Domain-scoped deadline forwarded to the wrapped sleeper.
    ///
    /// # Returns
    ///
    /// The owned sleep future returned by the wrapped sleeper.
    ///
    /// # Errors
    ///
    /// The future propagates any [`TimeError`] produced by the wrapped sleeper.
    #[inline(always)]
    fn sleep_until_async(&self, deadline: MonotonicInstant) -> SleepFuture {
        self.as_ref().sleep_until_async(deadline)
    }
}

/// Creates an immediately ready sleep future for a precomputed result.
///
/// # Parameters
///
/// * `result` - Completion result returned by the future.
///
/// # Returns
///
/// An owned future that resolves immediately to `result`.
///
/// # Errors
///
/// The future returns the supplied [`TimeError`] when `result` is `Err`.
#[inline]
pub(crate) fn ready_sleep_result(result: Result<(), TimeError>) -> SleepFuture {
    Box::pin(async move { result })
}
