// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores callbacks observing successful manual time advances.

use super::manual_waiter_registry::allocate_identifier;
use std::collections::HashMap;
use std::sync::Arc;

/// Callback invoked after a successful manual time advance.
pub(crate) type AdvanceCallback = Arc<dyn Fn() + Send + Sync + 'static>;

/// Advance callbacks registered against one manual monotonic timeline.
pub(crate) struct ManualAdvanceRegistry {
    /// Next identifier assigned to an advance subscriber.
    next_subscriber_id: u64,
    /// Callbacks invoked after manual time moves forward.
    subscribers: HashMap<u64, AdvanceCallback>,
}

impl ManualAdvanceRegistry {
    /// Creates an empty registry.
    ///
    /// # Returns
    ///
    /// A registry with no advance subscribers.
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            next_subscriber_id: 1,
            subscribers: HashMap::new(),
        }
    }

    /// Registers callback and returns its subscription identifier.
    ///
    /// Panics when the registry cannot allocate another identifier.
    ///
    /// # Parameters
    ///
    /// * `callback` - Shared callback to invoke after successful advances.
    ///
    /// # Returns
    ///
    /// The nonzero identifier assigned to the subscription.
    ///
    /// # Panics
    ///
    /// Panics when the subscriber identifier space is exhausted.
    #[inline]
    pub(crate) fn register(&mut self, callback: AdvanceCallback) -> u64 {
        let subscriber_id = allocate_identifier(
            &mut self.next_subscriber_id,
            "manual advance subscriber identifiers exhausted",
        );
        self.subscribers.insert(subscriber_id, callback);
        subscriber_id
    }

    /// Removes and returns the callback identified by `subscriber_id`.
    ///
    /// # Parameters
    ///
    /// * `subscriber_id` - Identifier of the callback to remove.
    ///
    /// # Returns
    ///
    /// The removed callback, or `None` when no registration has that identifier.
    #[inline(always)]
    pub(crate) fn unregister(
        &mut self,
        subscriber_id: u64,
    ) -> Option<AdvanceCallback> {
        self.subscribers.remove(&subscriber_id)
    }

    /// Clones callbacks for invocation after the clock releases its mutex.
    ///
    /// # Returns
    ///
    /// Shared handles to every currently registered callback.
    pub(crate) fn callbacks(&self) -> Vec<AdvanceCallback> {
        self.subscribers.values().cloned().collect()
    }
}
