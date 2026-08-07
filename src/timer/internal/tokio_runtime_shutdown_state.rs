// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Publishes Tokio runtime shutdown without allocating per deadline.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use tokio::sync::Notify;
use tokio::sync::futures::OwnedNotified;

/// Shared shutdown flag and asynchronous notification.
#[derive(Debug)]
pub(crate) struct TokioRuntimeShutdownState {
    /// Whether the retained runtime has dropped its sentinel.
    shutdown: AtomicBool,
    /// Wakes pending timer futures after shutdown.
    notification: Arc<Notify>,
}

impl TokioRuntimeShutdownState {
    /// Creates live runtime state.
    ///
    /// # Returns
    ///
    /// State that has not yet observed runtime shutdown.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            shutdown: AtomicBool::new(false),
            notification: Arc::new(Notify::new()),
        }
    }

    /// Reports whether runtime shutdown has been published.
    ///
    /// # Returns
    ///
    /// `true` after the sentinel guard signals shutdown.
    #[must_use]
    #[inline]
    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Creates an owned future notified by runtime shutdown.
    ///
    /// # Returns
    ///
    /// A future that becomes ready when shutdown is published.
    pub(crate) fn notification(&self) -> OwnedNotified {
        Arc::clone(&self.notification).notified_owned()
    }

    /// Publishes runtime shutdown and wakes every current observer.
    pub(crate) fn signal(&self) {
        self.shutdown.store(true, Ordering::Release);
        self.notification.notify_waiters();
    }
}
