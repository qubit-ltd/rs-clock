// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores callbacks observing successful manual time advances.

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
    pub(crate) fn new() -> Self {
        Self {
            next_subscriber_id: 1,
            subscribers: HashMap::new(),
        }
    }

    /// Registers callback and returns its subscription identifier.
    ///
    /// Panics when the registry cannot allocate another identifier.
    pub(crate) fn register(&mut self, callback: AdvanceCallback) -> u64 {
        let subscriber_id = self.next_subscriber_id;
        self.next_subscriber_id = self
            .next_subscriber_id
            .checked_add(1)
            .expect("manual advance subscriber identifiers exhausted");
        self.subscribers.insert(subscriber_id, callback);
        subscriber_id
    }

    /// Removes the callback identified by subscriber_id.
    pub(crate) fn unregister(&mut self, subscriber_id: u64) {
        self.subscribers.remove(&subscriber_id);
    }

    /// Clones callbacks for invocation after the clock releases its mutex.
    pub(crate) fn callbacks(&self) -> Vec<AdvanceCallback> {
        self.subscribers.values().cloned().collect()
    }
}
