// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a cancellation-safe subscription to manual time advances.

use crate::ManualMonotonicClock;
use std::fmt::{
    Debug,
    Formatter,
};
use std::sync::Weak;

/// A registration that observes successful manual time advances.
///
/// The callback runs synchronously after the clock releases its internal
/// mutex. It should only signal the subscriber's own waiting primitive and
/// should not perform expensive work. If callbacks panic, the clock attempts
/// every callback collected for that advance before resuming the first panic.
/// Dropping this handle unregisters future notifications, although an in-flight
/// advance that already captured the callback may still invoke it once.
/// The handle must therefore be retained for as long as notifications are
/// required.
///
/// Ignoring the returned subscription is rejected when `unused_must_use` is
/// denied:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use qubit_clock::ManualMonotonicClock;
/// use std::sync::Arc;
///
/// let clock = Arc::new(ManualMonotonicClock::new());
/// clock.subscribe_advances(|| {});
/// ```
#[must_use = "dropping the subscription unregisters the callback"]
pub struct ManualAdvanceSubscription {
    /// The weak reference to the manual clock.
    clock: Weak<ManualMonotonicClock>,
    /// The identifier of the subscriber.
    subscriber_id: u64,
}

impl ManualAdvanceSubscription {
    /// Creates a subscription for `subscriber_id` without retaining the clock.
    ///
    /// # Parameters
    ///
    /// * `clock` - Weak reference to the clock owning the registration.
    /// * `subscriber_id` - Identifier of the registered callback.
    ///
    /// # Returns
    ///
    /// A cancellation handle for the registered callback.
    #[inline(always)]
    pub(crate) const fn new(
        clock: Weak<ManualMonotonicClock>,
        subscriber_id: u64,
    ) -> Self {
        Self {
            clock,
            subscriber_id,
        }
    }
}

impl Debug for ManualAdvanceSubscription {
    /// Formats the subscriber identifier without locking the clock.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination formatter.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the subscription is formatted.
    ///
    /// # Errors
    ///
    /// Returns [`std::fmt::Error`] when the formatter rejects the output.
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualAdvanceSubscription")
            .field("subscriber_id", &self.subscriber_id)
            .finish_non_exhaustive()
    }
}

impl Drop for ManualAdvanceSubscription {
    /// Unregisters this subscriber if its manual clock still exists.
    ///
    /// # Panics
    ///
    /// Panics if destroying the callback or one of its captured values panics.
    #[inline]
    fn drop(&mut self) {
        if let Some(clock) = self.clock.upgrade() {
            clock.unregister_advance_subscriber(self.subscriber_id);
        }
    }
}
