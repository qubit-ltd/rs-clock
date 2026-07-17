// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use criterion::{
    Criterion,
    criterion_group,
    criterion_main,
};
use qubit_clock::{
    MonotonicClock,
    StdMonotonicClock,
    TimerFuture,
};
use std::hint::black_box;
use std::sync::{
    Arc,
    Barrier,
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
use std::time::Duration;

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
    /// * `thread` - Thread that will park while a Timer future is pending.
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

    /// Parks until a Timer worker notification is consumed.
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
fn block_on_timer_future(mut future: TimerFuture) {
    let thread_waker = Arc::new(ThreadWaker::new(std::thread::current()));
    let waker = Waker::from(Arc::clone(&thread_waker));
    let mut context = Context::from_waker(&waker);
    loop {
        thread_waker.prepare_to_poll();
        if future.as_mut().poll(&mut context).is_ready() {
            return;
        }
        thread_waker.park_until_notified();
    }
}

/// Registers and cancels deadlines concurrently on independent standard Timers.
///
/// # Parameters
///
/// * `timer_count` - Number of independent Timers registering concurrently.
/// * `registrations_per_timer` - Registrations cancelled by each Timer.
///
/// # Panics
///
/// Panics when a deadline cannot be registered or a benchmark worker panics.
fn register_and_cancel_parallel_timers(
    timer_count: usize,
    registrations_per_timer: usize,
) {
    let barrier = Arc::new(Barrier::new(timer_count));
    std::thread::scope(|scope| {
        let mut workers = Vec::with_capacity(timer_count);
        for _ in 0..timer_count {
            let barrier = Arc::clone(&barrier);
            workers.push(scope.spawn(move || {
                let timer = StdMonotonicClock::new().new_timer();
                barrier.wait();
                for _ in 0..registrations_per_timer {
                    let future = timer
                        .after(Duration::from_secs(60))
                        .expect("benchmark deadline should register");
                    drop(future);
                }
            }));
        }
        for worker in workers {
            worker.join().expect("benchmark worker should finish");
        }
    });
}

/// Completes deadlines concurrently on independent standard Timers.
///
/// # Parameters
///
/// * `timer_count` - Number of independent Timers completing concurrently.
///
/// # Panics
///
/// Panics when a deadline cannot be registered or a benchmark worker panics.
fn complete_parallel_timers(timer_count: usize) {
    let barrier = Arc::new(Barrier::new(timer_count));
    std::thread::scope(|scope| {
        let mut workers = Vec::with_capacity(timer_count);
        for _ in 0..timer_count {
            let barrier = Arc::clone(&barrier);
            workers.push(scope.spawn(move || {
                let timer = StdMonotonicClock::new().new_timer();
                barrier.wait();
                let future = timer
                    .after(Duration::from_millis(1))
                    .expect("benchmark deadline should register");
                block_on_timer_future(future);
            }));
        }
        for worker in workers {
            worker.join().expect("benchmark worker should finish");
        }
    });
}

/// Benchmarks shared-scheduler contention during registration and cancellation.
fn benchmark_parallel_registration_and_cancellation(criterion: &mut Criterion) {
    criterion.bench_function(
        "std_timer/parallel_registration_and_cancellation",
        |bencher| {
            bencher.iter(|| {
                register_and_cancel_parallel_timers(
                    black_box(8),
                    black_box(32),
                );
            });
        },
    );
}

/// Benchmarks parallel deadline completion and Waker fanout.
fn benchmark_parallel_deadline_completion(criterion: &mut Criterion) {
    criterion.bench_function(
        "std_timer/parallel_deadline_completion",
        |bencher| {
            bencher.iter(|| complete_parallel_timers(black_box(16)));
        },
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets =
        benchmark_parallel_registration_and_cancellation,
        benchmark_parallel_deadline_completion
}
criterion_main!(benches);
