// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Stores mutable state for a manual monotonic clock.

use std::collections::HashMap;
use std::sync::Arc;
use std::task::Waker;
use std::time::Duration;

/// Callback invoked after a successful manual time advance.
pub(crate) type AdvanceCallback = Arc<dyn Fn() + Send + Sync + 'static>;

/// Advance callbacks keyed by their registration identifier.
pub(crate) type AdvanceSubscribers = HashMap<u64, AdvanceCallback>;

/// Expected waiter counts and task wakers keyed by registration identifier.
pub(crate) type WaiterObservers = HashMap<u64, (usize, Option<Waker>)>;

/// Mutable time and waiter registrations protected by the owning clock.
pub(crate) struct ManualMonotonicState {
    /// Current logical duration from the manual clock origin.
    pub(crate) elapsed: Duration,
    /// Next identifier assigned to a blocking waiter.
    pub(crate) next_blocking_waiter_id: u64,
    /// Blocking waiter deadlines keyed by registration identifier.
    pub(crate) blocking_waiters: HashMap<u64, Duration>,
    /// Next identifier assigned to an async waiter.
    pub(crate) next_async_waiter_id: u64,
    /// Async deadlines and optional task wakers keyed by registration ID.
    pub(crate) async_waiters: HashMap<u64, (Duration, Option<Waker>)>,
    /// Next identifier assigned to an asynchronous waiter-count observer.
    pub(crate) next_waiter_observer_id: u64,
    /// Asynchronous observers waiting for a minimum total waiter count.
    pub(crate) waiter_observers: WaiterObservers,
    /// Next identifier assigned to an advance subscriber.
    pub(crate) next_advance_subscriber_id: u64,
    /// Callbacks invoked after manual time moves forward.
    pub(crate) advance_subscribers: AdvanceSubscribers,
}

impl ManualMonotonicState {
    /// Creates state at the clock domain origin.
    pub(crate) fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            next_blocking_waiter_id: 1,
            blocking_waiters: HashMap::new(),
            next_async_waiter_id: 1,
            async_waiters: HashMap::new(),
            next_waiter_observer_id: 1,
            waiter_observers: HashMap::new(),
            next_advance_subscriber_id: 1,
            advance_subscribers: HashMap::new(),
        }
    }

    /// Returns the number of blocking and asynchronous deadline waiters.
    pub(crate) fn waiter_count(&self) -> usize {
        self.blocking_waiters.len() + self.async_waiters.len()
    }
}
