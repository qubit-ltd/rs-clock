// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Exposes the production notification latch to external Loom models.

use crate::sleep::internal::notification_latch::NotificationLatch;

/// Loom-facing adapter around the production notification latch.
pub struct LoomNotificationLatch {
    /// Production latch whose atomic operations are modeled by Loom.
    inner: NotificationLatch,
}

impl LoomNotificationLatch {
    /// Creates a latch without a pending notification.
    ///
    /// # Returns
    ///
    /// A model adapter containing the production latch.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: NotificationLatch::new(),
        }
    }

    /// Clears a stale notification before another modeled poll.
    #[inline(always)]
    pub fn clear_notification(&self) {
        self.inner.clear_notification();
    }

    /// Latches a notification for a current or future modeled observer.
    #[inline(always)]
    pub fn notify(&self) {
        self.inner.notify();
    }

    /// Takes and clears the currently latched notification.
    ///
    /// # Returns
    ///
    /// `true` when a notification was pending.
    #[must_use]
    #[inline(always)]
    pub fn take_notification(&self) -> bool {
        self.inner.take_notification()
    }
}
