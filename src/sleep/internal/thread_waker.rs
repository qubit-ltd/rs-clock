// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the notification latch used by a blocking sleeper.

use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        Ordering,
    },
};
use std::task::Wake;

/// Thread notification latch used as a future waker.
pub(crate) struct ThreadWaker {
    /// Thread parked while its timer future remains pending.
    thread: std::thread::Thread,
    /// Notification bit preventing wake-before-park races.
    notified: AtomicBool,
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
    pub(crate) const fn new(thread: std::thread::Thread) -> Self {
        Self {
            thread,
            notified: AtomicBool::new(false),
        }
    }

    /// Clears a stale notification before polling the future again.
    #[inline(always)]
    pub(crate) fn clear_notification(&self) {
        self.notified.store(false, Ordering::Release);
    }

    /// Takes and clears the currently latched notification.
    ///
    /// # Returns
    ///
    /// `true` when a wake occurred since the previous clear or take.
    #[must_use]
    #[inline(always)]
    pub(crate) fn take_notification(&self) -> bool {
        self.notified.swap(false, Ordering::AcqRel)
    }
}

impl Wake for ThreadWaker {
    /// Latches a notification before unparking the blocked thread.
    fn wake(self: Arc<Self>) {
        self.notified.store(true, Ordering::Release);
        self.thread.unpark();
    }
}
