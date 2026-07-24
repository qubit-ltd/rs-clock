// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Defines the one-bit notification state used while blocking on a Timer.

#[cfg(all(loom, feature = "loom-model"))]
use loom::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(all(loom, feature = "loom-model")))]
use std::sync::atomic::{AtomicBool, Ordering};

/// One-bit notification latch preventing wake-before-park loss.
pub(crate) struct NotificationLatch {
    /// Whether a notification is pending.
    notified: AtomicBool,
}

impl NotificationLatch {
    /// Creates a latch without a pending notification.
    ///
    /// # Returns
    ///
    /// A notification latch in its cleared state.
    #[must_use]
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            notified: AtomicBool::new(false),
        }
    }

    /// Clears a stale notification before another future poll.
    #[inline(always)]
    pub(crate) fn clear_notification(&self) {
        self.notified.store(false, Ordering::Release);
    }

    /// Latches a notification for a current or future observer.
    #[inline(always)]
    pub(crate) fn notify(&self) {
        self.notified.fetch_or(true, Ordering::Release);
    }

    /// Takes and clears the currently latched notification.
    ///
    /// # Returns
    ///
    /// `true` when a notification was pending.
    #[must_use]
    #[inline(always)]
    pub(crate) fn take_notification(&self) -> bool {
        self.notified.swap(false, Ordering::AcqRel)
    }
}
