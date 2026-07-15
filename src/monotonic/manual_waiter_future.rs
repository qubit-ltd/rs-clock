// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a future that observes manual waiter registration.

use crate::ManualMonotonicClock;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
};

/// A future that completes when a manual clock has enough registered waiters.
///
/// This is a deterministic synchronization primitive for tests. Its observer
/// is registered when the future is created, before its first poll. Dropping
/// an incomplete future unregisters the observer from the clock.
#[derive(Debug)]
pub struct ManualWaiterFuture {
    /// The reference to the manual clock.
    clock: Arc<ManualMonotonicClock>,
    /// The identifier of the observer.
    observer_id: Option<u64>,
}

impl ManualWaiterFuture {
    /// Creates a waiter-count observer for `clock`.
    #[inline]
    pub(crate) fn new(
        clock: Arc<ManualMonotonicClock>,
        expected_count: usize,
    ) -> Self {
        let observer_id = clock.register_waiter_observer(expected_count);
        Self { clock, observer_id }
    }
}

impl Future for ManualWaiterFuture {
    type Output = ();

    /// Polls whether the requested waiter count has been reached.
    fn poll(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let Some(observer_id) = self.observer_id else {
            return Poll::Ready(());
        };
        let result = self.clock.poll_waiter_observer(observer_id, context);
        if result.is_ready() {
            self.observer_id = None;
        }
        result
    }
}

impl Drop for ManualWaiterFuture {
    /// Unregisters an incomplete waiter-count observer.
    #[inline]
    fn drop(&mut self) {
        if let Some(observer_id) = self.observer_id.take() {
            self.clock.unregister_waiter_observer(observer_id);
        }
    }
}
