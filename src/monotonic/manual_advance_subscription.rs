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
/// must not panic or perform expensive work. Dropping this handle unregisters
/// future notifications, although an in-flight advance that already captured
/// the callback may still invoke it once.
pub struct ManualAdvanceSubscription {
    clock: Weak<ManualMonotonicClock>,
    subscriber_id: u64,
}

impl ManualAdvanceSubscription {
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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualAdvanceSubscription")
            .field("subscriber_id", &self.subscriber_id)
            .finish_non_exhaustive()
    }
}

impl Drop for ManualAdvanceSubscription {
    fn drop(&mut self) {
        if let Some(clock) = self.clock.upgrade() {
            clock.unregister_advance_subscriber(self.subscriber_id);
        }
    }
}
