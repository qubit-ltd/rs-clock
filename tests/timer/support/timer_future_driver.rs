// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Drives one Timer future to completion on the current thread.

use qubit_clock::TimerFuture;
use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        Ordering,
    },
};
use std::task::{
    Context,
    Poll,
    Wake,
    Waker,
};

/// Notifies a thread that is synchronously polling one Timer future.
struct ThreadWaker {
    /// Thread parked between future polls.
    thread: std::thread::Thread,
    /// Notification latch preventing wake-before-park races.
    notified: AtomicBool,
}

impl ThreadWaker {
    /// Creates an unnotified waker for `thread`.
    ///
    /// # Parameters
    ///
    /// * `thread` - Thread that will park while its Timer future is pending.
    ///
    /// # Returns
    ///
    /// A notification latch bound to `thread`.
    #[must_use]
    #[inline]
    fn new(thread: std::thread::Thread) -> Self {
        Self {
            thread,
            notified: AtomicBool::new(false),
        }
    }

    /// Clears an earlier notification immediately before polling.
    #[inline(always)]
    fn prepare_to_poll(&self) {
        self.notified.store(false, Ordering::Release);
    }

    /// Parks until a notification is consumed.
    fn park_until_notified(&self) {
        while !self.notified.swap(false, Ordering::AcqRel) {
            std::thread::park();
        }
    }
}

impl Wake for ThreadWaker {
    /// Latches a notification before unparking the polling thread.
    #[inline(always)]
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Latches a notification before unparking the polling thread.
    #[inline(always)]
    fn wake_by_ref(self: &Arc<Self>) {
        self.notified.store(true, Ordering::Release);
        self.thread.unpark();
    }
}

/// Blocks the current thread until `future` becomes ready.
///
/// # Parameters
///
/// * `future` - Eagerly registered Timer future to drive to completion.
pub(crate) fn block_on_timer_future(mut future: TimerFuture) {
    let thread_waker = Arc::new(ThreadWaker::new(std::thread::current()));
    let waker = Waker::from(Arc::clone(&thread_waker));
    let mut context = Context::from_waker(&waker);
    loop {
        thread_waker.prepare_to_poll();
        if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
            return result.expect("timer should complete");
        }
        thread_waker.park_until_notified();
    }
}
