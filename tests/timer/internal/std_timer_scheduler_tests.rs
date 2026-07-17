// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    StdMonotonicClock,
    StdTimer,
    Timer,
    TimerFuture,
};
use std::sync::{
    Arc,
    Mutex,
    atomic::{
        AtomicBool,
        Ordering,
    },
};
use std::task::{
    Context,
    Wake,
    Waker,
};
use std::thread::{
    Thread,
    ThreadId,
};
use std::time::Duration;

/// Records the worker thread that wakes a synchronously polled Timer future.
struct WakeThreadRecorder {
    /// Thread parked between future polls.
    polling_thread: Thread,
    /// Notification latch preventing wake-before-park races.
    notified: AtomicBool,
    /// Identifier of the most recent thread that delivered a wake.
    wake_thread: Mutex<Option<ThreadId>>,
}

impl WakeThreadRecorder {
    /// Creates an empty recorder for `polling_thread`.
    ///
    /// # Parameters
    ///
    /// * `polling_thread` - Thread parked while the Timer future is pending.
    ///
    /// # Returns
    ///
    /// A recorder without a delivered wake.
    #[must_use]
    #[inline]
    fn new(polling_thread: Thread) -> Self {
        Self {
            polling_thread,
            notified: AtomicBool::new(false),
            wake_thread: Mutex::new(None),
        }
    }

    /// Clears an earlier notification immediately before polling.
    #[inline(always)]
    fn prepare_to_poll(&self) {
        self.notified.store(false, Ordering::Release);
    }

    /// Parks until a Timer worker notification is consumed.
    fn park_until_notified(&self) {
        while !self.notified.swap(false, Ordering::AcqRel) {
            std::thread::park();
        }
    }

    /// Returns the thread identifier recorded by the Timer wake.
    ///
    /// # Returns
    ///
    /// The unique identifier of the thread that delivered the wake.
    ///
    /// # Panics
    ///
    /// Panics when the future completed without a registered wake.
    #[must_use]
    fn wake_thread(&self) -> ThreadId {
        self.wake_thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .expect("pending Timer future should record its waking thread")
    }
}

impl Wake for WakeThreadRecorder {
    /// Records the current worker and unparks the polling thread.
    #[inline(always)]
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Records the current worker and unparks the polling thread.
    #[inline(always)]
    fn wake_by_ref(self: &Arc<Self>) {
        *self
            .wake_thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(std::thread::current().id());
        self.notified.store(true, Ordering::Release);
        self.polling_thread.unpark();
    }
}

/// Blocks until `future` completes and returns its waking worker thread.
///
/// # Parameters
///
/// * `future` - Eagerly registered Timer future to drive to completion.
///
/// # Returns
///
/// The unique identifier of the thread that delivered the completion wake.
fn block_on_with_wake_thread(mut future: TimerFuture) -> ThreadId {
    let recorder = Arc::new(WakeThreadRecorder::new(std::thread::current()));
    let waker = Waker::from(Arc::clone(&recorder));
    let mut context = Context::from_waker(&waker);
    loop {
        recorder.prepare_to_poll();
        if future.as_mut().poll(&mut context).is_ready() {
            return recorder.wake_thread();
        }
        recorder.park_until_notified();
    }
}

/// Verifies that independent standard Timers retain one process worker.
#[test]
fn test_std_timer_scheduler_shares_and_retains_process_worker() {
    let first_clock = StdMonotonicClock::new();
    let second_clock = StdMonotonicClock::new();
    let first_timer = StdTimer::from_clock(&first_clock);
    let second_timer = StdTimer::from_clock(&second_clock);
    let first_future = first_timer
        .after(Duration::from_millis(100))
        .expect("first deadline should register");
    let second_future = second_timer
        .after(Duration::from_millis(100))
        .expect("second deadline should register");

    let first =
        std::thread::spawn(move || block_on_with_wake_thread(first_future));
    let second =
        std::thread::spawn(move || block_on_with_wake_thread(second_future));

    let first_worker = first.join().expect("first waiter should finish");
    let second_worker = second.join().expect("second waiter should finish");
    assert_eq!(first_worker, second_worker);

    std::thread::sleep(Duration::from_millis(10));
    let later_future = first_timer
        .after(Duration::from_millis(5))
        .expect("later deadline should register");
    let retained_worker = block_on_with_wake_thread(later_future);

    assert_eq!(first_worker, retained_worker);
}
