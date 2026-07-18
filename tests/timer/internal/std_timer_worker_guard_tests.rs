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

struct WakeThreadRecorder {
    polling_thread: Thread,
    notified: AtomicBool,
    wake_thread: Mutex<Option<ThreadId>>,
}

impl WakeThreadRecorder {
    fn new(polling_thread: Thread) -> Self {
        Self {
            polling_thread,
            notified: AtomicBool::new(false),
            wake_thread: Mutex::new(None),
        }
    }

    fn prepare_to_poll(&self) {
        self.notified.store(false, Ordering::Release);
    }

    fn park_until_notified(&self) {
        while !self.notified.swap(false, Ordering::AcqRel) {
            std::thread::park();
        }
    }

    fn wake_thread(&self) -> Option<ThreadId> {
        *self
            .wake_thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Wake for WakeThreadRecorder {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

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

fn block_on_with_wake_thread(mut future: TimerFuture) -> Option<ThreadId> {
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

fn observe_worker(timer: &dyn Timer) -> ThreadId {
    for _ in 0..4 {
        let future = timer
            .after(Duration::from_millis(50))
            .expect("worker observation deadline should register");
        if let Some(worker) = block_on_with_wake_thread(future) {
            return worker;
        }
    }
    panic!("standard Timer worker should be observable within four attempts");
}

#[test]
fn test_std_timer_worker_guard_shares_and_retains_process_worker() {
    let first_clock = StdMonotonicClock::new();
    let second_clock = StdMonotonicClock::new();
    let first_timer = Arc::new(StdTimer::from_clock(&first_clock));
    let second_timer = Arc::new(StdTimer::from_clock(&second_clock));

    let first_waiter = Arc::clone(&first_timer);
    let second_waiter = Arc::clone(&second_timer);
    let first =
        std::thread::spawn(move || observe_worker(first_waiter.as_ref()));
    let second =
        std::thread::spawn(move || observe_worker(second_waiter.as_ref()));

    let first_worker = first.join().expect("first waiter should finish");
    let second_worker = second.join().expect("second waiter should finish");
    assert_eq!(first_worker, second_worker);
    assert_eq!(first_worker, observe_worker(first_timer.as_ref()));
}
