// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
    measurement::WallTime,
};
use qubit_clock::{MonotonicClock, StdMonotonicClock, TimerFuture};
use std::cell::Cell;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicBool, Ordering},
};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

/// Persistent caller-thread counts used to expose scheduler scaling behavior.
const CONCURRENT_WORKER_COUNTS: [usize; 5] = [1, 2, 4, 8, 16];

/// Timeout futures retained by each lock-style cancellation worker.
const REGISTRATIONS_PER_WORKER: usize = 64;

/// Long deadline cancelled before it can complete.
const CANCELLATION_DEADLINE: Duration = Duration::from_secs(60);

/// Real deadline used to benchmark completion and Waker fanout.
const COMPLETION_DEADLINE: Duration = Duration::from_millis(1);

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

/// Blocks the current thread until `future` becomes ready and reports success.
///
/// # Parameters
///
/// * `future` - Eagerly registered Timer future to drive to completion.
///
/// # Returns
///
/// `true` when the future completes without a timer error.
fn block_on_timer_future(mut future: TimerFuture) -> bool {
    let thread_waker = Arc::new(ThreadWaker::new(std::thread::current()));
    let waker = Waker::from(Arc::clone(&thread_waker));
    let mut context = Context::from_waker(&waker);
    loop {
        thread_waker.prepare_to_poll();
        if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
            return result.is_ok();
        }
        thread_waker.park_until_notified();
    }
}

/// Benchmarks serial registration and cancellation without caller barriers.
///
/// # Parameters
///
/// * `group` - Criterion group receiving the throughput benchmark.
fn benchmark_serial_registration_and_cancellation(group: &mut BenchmarkGroup<'_, WallTime>) {
    let timer = StdMonotonicClock::new().new_timer();
    group.throughput(Throughput::Elements(REGISTRATIONS_PER_WORKER as u64));
    group.bench_function("serial_registration_and_cancellation", |bencher| {
        let registration_succeeded = Cell::new(true);
        bencher.iter(|| {
            let mut futures = Vec::with_capacity(REGISTRATIONS_PER_WORKER);
            for _ in 0..REGISTRATIONS_PER_WORKER {
                match timer.after(CANCELLATION_DEADLINE) {
                    Ok(future) => futures.push(future),
                    Err(_) => {
                        registration_succeeded.set(false);
                        break;
                    }
                }
            }
            drop(futures);
        });
        assert!(
            registration_succeeded.get(),
            "benchmark deadline should register"
        );
    });
}

/// Benchmarks lock-style concurrent registration and cancellation.
///
/// # Parameters
///
/// * `group` - Criterion group receiving the throughput benchmark.
///
/// # Panics
///
/// Panics when a deadline cannot be registered or a benchmark worker panics.
fn benchmark_parallel_registration_and_cancellation(group: &mut BenchmarkGroup<'_, WallTime>) {
    for worker_count in CONCURRENT_WORKER_COUNTS {
        let timer = StdMonotonicClock::new().new_timer();
        let start = Arc::new(Barrier::new(worker_count + 1));
        let registered = Arc::new(Barrier::new(worker_count));
        let finished = Arc::new(Barrier::new(worker_count + 1));
        let stopping = Arc::new(AtomicBool::new(false));
        let registration_succeeded = Arc::new(AtomicBool::new(true));

        std::thread::scope(|scope| {
            for _ in 0..worker_count {
                let timer = Arc::clone(&timer);
                let start = Arc::clone(&start);
                let registered = Arc::clone(&registered);
                let finished = Arc::clone(&finished);
                let stopping = Arc::clone(&stopping);
                let registration_succeeded = Arc::clone(&registration_succeeded);
                scope.spawn(move || {
                    loop {
                        start.wait();
                        if stopping.load(Ordering::Acquire) {
                            return;
                        }
                        let mut futures = Vec::with_capacity(REGISTRATIONS_PER_WORKER);
                        for _ in 0..REGISTRATIONS_PER_WORKER {
                            match timer.after(CANCELLATION_DEADLINE) {
                                Ok(future) => futures.push(future),
                                Err(_) => {
                                    registration_succeeded.store(false, Ordering::Release);
                                    break;
                                }
                            }
                        }
                        registered.wait();
                        drop(futures);
                        finished.wait();
                    }
                });
            }

            let elements = (worker_count * REGISTRATIONS_PER_WORKER) as u64;
            group.throughput(Throughput::Elements(elements));
            group.bench_with_input(
                BenchmarkId::new("lock_style_registration_and_cancellation", worker_count),
                &worker_count,
                |bencher, _worker_count| {
                    bencher.iter(|| {
                        start.wait();
                        finished.wait();
                    });
                },
            );

            stopping.store(true, Ordering::Release);
            start.wait();
            assert!(
                registration_succeeded.load(Ordering::Acquire),
                "benchmark deadline should register"
            );
        });
    }
}

/// Benchmarks concurrent deadline completion and Waker fanout.
///
/// # Parameters
///
/// * `group` - Criterion group receiving the throughput benchmark.
///
/// # Panics
///
/// Panics when a deadline cannot be registered or a benchmark worker panics.
fn benchmark_parallel_deadline_completion(group: &mut BenchmarkGroup<'_, WallTime>) {
    for worker_count in CONCURRENT_WORKER_COUNTS {
        let timer = StdMonotonicClock::new().new_timer();
        let start = Arc::new(Barrier::new(worker_count + 1));
        let finished = Arc::new(Barrier::new(worker_count + 1));
        let stopping = Arc::new(AtomicBool::new(false));
        let completion_succeeded = Arc::new(AtomicBool::new(true));

        std::thread::scope(|scope| {
            for _ in 0..worker_count {
                let timer = Arc::clone(&timer);
                let start = Arc::clone(&start);
                let finished = Arc::clone(&finished);
                let stopping = Arc::clone(&stopping);
                let completion_succeeded = Arc::clone(&completion_succeeded);
                scope.spawn(move || {
                    loop {
                        start.wait();
                        if stopping.load(Ordering::Acquire) {
                            return;
                        }
                        match timer.after(COMPLETION_DEADLINE) {
                            Ok(future) => {
                                if !block_on_timer_future(future) {
                                    completion_succeeded.store(false, Ordering::Release);
                                }
                            }
                            Err(_) => completion_succeeded.store(false, Ordering::Release),
                        }
                        finished.wait();
                    }
                });
            }

            group.throughput(Throughput::Elements(worker_count as u64));
            group.bench_with_input(
                BenchmarkId::new("parallel_deadline_completion", worker_count),
                &worker_count,
                |bencher, _worker_count| {
                    bencher.iter(|| {
                        start.wait();
                        finished.wait();
                    });
                },
            );

            stopping.store(true, Ordering::Release);
            start.wait();
            assert!(
                completion_succeeded.load(Ordering::Acquire),
                "benchmark deadline should complete"
            );
        });
    }
}

/// Benchmarks shared standard Timer behavior under downstream-style
/// concurrency.
///
/// # Parameters
///
/// * `criterion` - Criterion registry receiving the standard Timer group.
fn benchmark_std_timer_scheduler(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("std_timer");
    benchmark_serial_registration_and_cancellation(&mut group);
    benchmark_parallel_registration_and_cancellation(&mut group);
    benchmark_parallel_deadline_completion(&mut group);
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = benchmark_std_timer_scheduler
}
criterion_main!(benches);
