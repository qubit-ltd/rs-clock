// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the notification latch used by a blocking sleeper.

use std::sync::Arc;
use std::task::Wake;

use super::notification_latch::NotificationLatch;

/// Thread notification latch used as a future waker.
pub(crate) struct ThreadWaker {
    /// Thread parked while its timer future remains pending.
    thread: std::thread::Thread,
    /// Notification bit preventing wake-before-park races.
    notification: NotificationLatch,
}

impl ThreadWaker {
    /// Creates a notification latch for `thread`.
    ///
    /// # Parameters
    ///
    /// * `thread` - Thread to unpark after a notification is latched.
    ///
    /// # Returns
    ///
    /// A latch with no pending notification.
    #[must_use]
    #[inline(always)]
    pub(crate) fn new(thread: std::thread::Thread) -> Self {
        Self {
            thread,
            notification: NotificationLatch::new(),
        }
    }

    /// Clears a stale notification before polling the future again.
    #[inline(always)]
    pub(crate) fn clear_notification(&self) {
        self.notification.clear_notification();
    }

    /// Takes and clears the currently latched notification.
    ///
    /// # Returns
    ///
    /// `true` when a wake occurred since the previous clear or take.
    #[must_use]
    #[inline(always)]
    pub(crate) fn take_notification(&self) -> bool {
        self.notification.take_notification()
    }
}

impl Wake for ThreadWaker {
    /// Latches a notification before unparking the blocked thread.
    #[inline(always)]
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Latches a notification before unparking the blocked thread.
    #[inline(always)]
    fn wake_by_ref(self: &Arc<Self>) {
        self.notification.notify();
        self.thread.unpark();
    }
}
