// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a blocking adapter over the asynchronous Timer capability.

use super::internal::ThreadWaker;
use crate::{
    MonotonicInstant,
    TimeError,
    Timer,
    TimerFuture,
};
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;

/// Adapts any [`Timer`] into synchronous blocking sleep operations.
///
/// This type owns no clock or scheduling policy of its own. It composes a
/// shared timer and blocks only the calling thread while polling the timer's
/// future. Clones share that same timer.
///
/// # Progress
///
/// The timer backend must still make progress after the calling thread parks.
/// [`StdTimer`](crate::StdTimer) has its own worker. A
/// [`ManualTimer`](crate::ManualTimer) requires another thread or controller to
/// advance its clock. A `TokioTimer` requires its retained runtime to remain
/// alive and be driven independently. Blocking the sole driver thread of a
/// current-thread Tokio runtime while waiting on that same runtime's timer
/// prevents the deadline from completing.
#[derive(Clone)]
pub struct BlockingSleeper {
    /// Timer providing eager deadline registration and completion futures.
    timer: Arc<dyn Timer>,
}

impl BlockingSleeper {
    /// Creates a blocking adapter over a shared timer.
    ///
    /// # Parameters
    ///
    /// * `timer` - Timer used for every blocking deadline. Its backend must
    ///   progress independently while the calling thread is parked.
    ///
    /// # Returns
    ///
    /// A cloneable blocking sleeper composing `timer`.
    #[must_use]
    #[inline(always)]
    pub const fn new(timer: Arc<dyn Timer>) -> Self {
        Self { timer }
    }

    /// Returns the timer composed by this adapter.
    ///
    /// # Returns
    ///
    /// The timer used to register blocking sleeps.
    #[must_use]
    #[inline(always)]
    pub fn timer(&self) -> &dyn Timer {
        self.timer.as_ref()
    }

    /// Blocks the current thread until an absolute deadline is reached.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Deadline in the composed timer's clock domain.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the deadline future completes.
    ///
    /// # Errors
    ///
    /// Returns any error produced while registering or completing the
    /// deadline.
    ///
    /// # Panics
    ///
    /// Panics when the composed timer panics during registration or its
    /// returned future panics while being polled.
    ///
    /// # Blocking
    ///
    /// Parks the calling thread while the composed timer is pending. The timer
    /// backend must be driven independently during that interval.
    #[inline(always)]
    pub fn sleep_until(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<(), TimeError> {
        let future = self.timer.at(deadline)?;
        Self::block_on(future)
    }

    /// Blocks the current thread for a relative duration.
    ///
    /// The timer fixes the absolute deadline before this method begins polling
    /// and parking.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration measured by the composed timer's clock.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the deadline future completes.
    ///
    /// # Errors
    ///
    /// Returns deadline overflow, registration failure, or a backend failure
    /// reported while waiting.
    ///
    /// # Panics
    ///
    /// Panics when the composed timer panics during registration or its
    /// returned future panics while being polled.
    ///
    /// # Blocking
    ///
    /// Parks the calling thread while the composed timer is pending. The timer
    /// backend must be driven independently during that interval.
    #[inline(always)]
    pub fn sleep_for(&self, duration: Duration) -> Result<(), TimeError> {
        let future = self.timer.after(duration)?;
        Self::block_on(future)
    }

    /// Polls one timer future, parking between incomplete polls.
    ///
    /// # Parameters
    ///
    /// * `future` - Eagerly registered timer future to drive to completion.
    ///
    /// # Returns
    ///
    /// The completion result produced by `future`.
    ///
    /// # Errors
    ///
    /// Returns the completion error reported by `future`.
    ///
    /// # Panics
    ///
    /// Panics when polling the timer future panics.
    fn block_on(mut future: TimerFuture) -> Result<(), TimeError> {
        let thread_waker = Arc::new(ThreadWaker::new(std::thread::current()));
        let waker = Waker::from(Arc::clone(&thread_waker));
        let mut context = Context::from_waker(&waker);
        loop {
            thread_waker.clear_notification();
            if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
                return result;
            }
            while !thread_waker.take_notification() {
                std::thread::park();
            }
        }
    }
}

impl std::fmt::Debug for BlockingSleeper {
    /// Formats this adapter without requiring the timer trait object to be
    /// debug-formattable.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination formatter.
    ///
    /// # Returns
    ///
    /// `Ok(())` when formatting succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`std::fmt::Error`] when the destination rejects output.
    #[inline]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BlockingSleeper")
            .finish_non_exhaustive()
    }
}
