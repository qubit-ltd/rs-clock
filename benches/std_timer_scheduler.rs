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

/// Completes short deadlines registered by independent standard Timers.
///
/// # Parameters
///
/// * `timer_count` - Number of independent Timers registered in one batch.
fn complete_independent_timers(timer_count: usize) {
    let futures = (0..timer_count)
        .map(|_| {
            StdMonotonicClock::new()
                .new_timer()
                .after(Duration::from_micros(250))
                .expect("benchmark deadline should register")
        })
        .collect::<Vec<_>>();
    futures.into_iter().for_each(block_on_timer_future);
}

/// Completes waits separated by more than the former worker idle grace.
///
/// # Parameters
///
/// * `wait_count` - Number of sequential waits to complete.
fn complete_sparse_waits(wait_count: usize) {
    let timer = StdMonotonicClock::new().new_timer();
    for index in 0..wait_count {
        let future = timer
            .after(Duration::from_micros(250))
            .expect("benchmark deadline should register");
        block_on_timer_future(future);
        if index + 1 < wait_count {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// Benchmarks concurrent registrations from independent standard Timers.
fn benchmark_concurrent_independent_timers(criterion: &mut Criterion) {
    criterion.bench_function(
        "std_timer/concurrent_independent_timers",
        |bencher| {
            bencher.iter(|| complete_independent_timers(black_box(16)));
        },
    );
}

/// Benchmarks sequential waits separated by an idle interval.
fn benchmark_sparse_sequential_waits(criterion: &mut Criterion) {
    criterion.bench_function("std_timer/sparse_sequential_waits", |bencher| {
        bencher.iter(|| complete_sparse_waits(black_box(8)));
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets =
        benchmark_concurrent_independent_timers,
        benchmark_sparse_sequential_waits
}
criterion_main!(benches);
