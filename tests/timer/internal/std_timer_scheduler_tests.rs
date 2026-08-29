// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::Barrier;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;
use std::thread::Thread;
use std::thread::ThreadId;
use std::time::Duration;

use qubit_clock::StdMonotonicClock;
use qubit_clock::StdTimer;
use qubit_clock::Timer;
use qubit_clock::TimerFuture;

/// Number of caller threads used by shared-scheduler concurrency tests.
const CONCURRENT_WORKER_COUNT: usize = 16;

/// Long-lived registrations retained by each cancellation-churn worker.
const REGISTRATIONS_PER_WORKER: usize = 64;

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

    /// Returns the thread identifier recorded by the Timer wake, if any.
    ///
    /// # Returns
    ///
    /// The unique identifier of the thread that delivered the wake, or `None`
    /// when the future completed before registering its Waker.
    #[must_use]
    fn wake_thread(&self) -> Option<ThreadId> {
        *self
            .wake_thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(std::thread::current().id());
        self.notified.store(true, Ordering::Release);
        self.polling_thread.unpark();
    }
}

/// Blocks until `future` completes and returns its waking worker, if any.
///
/// # Parameters
///
/// * `future` - Eagerly registered Timer future to drive to completion.
///
/// # Returns
///
/// The worker that delivered the completion wake, or `None` when the future
/// was already ready before its first poll.
fn block_on_with_wake_thread(mut future: TimerFuture) -> Option<ThreadId> {
    let recorder = Arc::new(WakeThreadRecorder::new(std::thread::current()));
    let waker = Waker::from(Arc::clone(&recorder));
    let mut context = Context::from_waker(&waker);
    loop {
        recorder.prepare_to_poll();
        if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
            result.expect("standard timer should complete");
            return recorder.wake_thread();
        }
        recorder.park_until_notified();
    }
}

/// Registers deadlines until one observes the standard Timer worker.
///
/// # Parameters
///
/// * `timer` - Standard Timer whose process worker should be observed.
///
/// # Returns
///
/// The unique identifier of the worker that completed one deadline.
///
/// # Panics
///
/// Panics when deadline registration fails or every bounded attempt completes
/// before its first poll.
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

/// Verifies that an immediately ready future does not require a prior wake.
#[test]
fn test_block_on_with_wake_thread_accepts_immediate_ready_future() {
    let future: TimerFuture = Box::pin(std::future::ready(Ok(())));
    assert_eq!(block_on_with_wake_thread(future), None);
}

/// Verifies that independent standard Timers retain one process worker.
#[test]
fn test_std_timer_scheduler_shares_and_retains_process_worker() {
    let first_clock = StdMonotonicClock::new();
    let second_clock = StdMonotonicClock::new();
    let first_timer = Arc::new(StdTimer::from_clock(&first_clock));
    let second_timer = Arc::new(StdTimer::from_clock(&second_clock));

    let first_waiter = Arc::clone(&first_timer);
    let second_waiter = Arc::clone(&second_timer);
    let first = std::thread::spawn(move || observe_worker(first_waiter.as_ref()));
    let second = std::thread::spawn(move || observe_worker(second_waiter.as_ref()));

    let first_worker = first.join().expect("first waiter should finish");
    let second_worker = second.join().expect("second waiter should finish");
    assert_eq!(first_worker, second_worker);

    let retained_worker = observe_worker(first_timer.as_ref());

    assert_eq!(first_worker, retained_worker);
}

/// Verifies lock-style concurrent timeout cancellation leaves the shared
/// scheduler responsive.
#[test]
fn test_std_timer_scheduler_handles_parallel_lock_style_cancellation_churn() {
    let clock = StdMonotonicClock::new();
    let timer = Arc::new(StdTimer::from_clock(&clock));
    let start = Arc::new(Barrier::new(CONCURRENT_WORKER_COUNT));
    let registered = Arc::new(Barrier::new(CONCURRENT_WORKER_COUNT));

    std::thread::scope(|scope| {
        let mut workers = Vec::with_capacity(CONCURRENT_WORKER_COUNT);
        for _ in 0..CONCURRENT_WORKER_COUNT {
            let timer = Arc::clone(&timer);
            let start = Arc::clone(&start);
            let registered = Arc::clone(&registered);
            workers.push(scope.spawn(move || {
                start.wait();
                let mut futures = Vec::with_capacity(REGISTRATIONS_PER_WORKER);
                for _ in 0..REGISTRATIONS_PER_WORKER {
                    futures.push(
                        timer
                            .after(Duration::from_secs(60))
                            .expect("lock-style deadline should register"),
                    );
                }
                registered.wait();
                drop(futures);
            }));
        }
        for worker in workers {
            worker.join().expect("cancellation worker should finish");
        }
    });

    let survivor = timer
        .after(Duration::from_millis(5))
        .expect("post-cancellation deadline should register");
    block_on_with_wake_thread(survivor);
}

/// Verifies concurrent deadlines are completed by one process worker.
#[test]
fn test_std_timer_scheduler_uses_one_worker_for_concurrent_deadlines() {
    let clock = StdMonotonicClock::new();
    let timer = Arc::new(StdTimer::from_clock(&clock));
    let start = Arc::new(Barrier::new(CONCURRENT_WORKER_COUNT));

    let worker_threads = std::thread::scope(|scope| {
        let mut waiters = Vec::with_capacity(CONCURRENT_WORKER_COUNT);
        for _ in 0..CONCURRENT_WORKER_COUNT {
            let timer = Arc::clone(&timer);
            let start = Arc::clone(&start);
            waiters.push(scope.spawn(move || {
                start.wait();
                observe_worker(timer.as_ref())
            }));
        }
        waiters
            .into_iter()
            .map(|waiter| waiter.join().expect("deadline waiter should finish"))
            .collect::<Vec<_>>()
    });

    let first_worker = worker_threads
        .first()
        .expect("at least one worker thread should be observed");
    assert!(
        worker_threads.iter().all(|worker| worker == first_worker),
        "all concurrent deadlines should use one scheduler worker",
    );
}
