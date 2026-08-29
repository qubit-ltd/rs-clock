// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Benchmarks manual-timer registration, cancellation, and deadline delivery.

use std::cell::Cell;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::TimerFuture;

/// Timer populations spanning small tests through high-cardinality workloads.
const WAITER_COUNTS: [usize; 8] = [1, 8, 32, 63, 64, 65, 128, 1_024];

/// Deadline shared by every waiter in the batch-completion scenario.
const BATCH_DEADLINE: Duration = Duration::from_secs(1);

/// Polls a timer future once and reports whether it completed successfully.
///
/// # Parameters
///
/// * `future` - Future whose manual deadline has already been reached.
/// * `context` - Poll context backed by a no-op waker.
///
/// # Returns
///
/// `true` when the future is ready with a successful result.
#[inline]
fn is_ready(future: &mut TimerFuture, context: &mut Context<'_>) -> bool {
    matches!(future.as_mut().poll(context), Poll::Ready(Ok(())))
}

/// Benchmarks eager registration followed by cancellation through `Drop`.
fn benchmark_registration_and_cancellation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("manual_timer/registration_and_cancellation");
    for waiter_count in WAITER_COUNTS {
        let clock = ManualMonotonicClock::new_shared();
        let timer = clock.new_timer();
        group.throughput(Throughput::Elements(waiter_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(waiter_count),
            &waiter_count,
            |bencher, &waiter_count| {
                let registration_succeeded = Cell::new(true);
                bencher.iter(|| {
                    let mut futures = Vec::with_capacity(waiter_count);
                    for _ in 0..waiter_count {
                        match timer.after(BATCH_DEADLINE) {
                            Ok(future) => futures.push(future),
                            Err(_) => {
                                registration_succeeded.set(false);
                                break;
                            }
                        }
                    }
                    drop(futures);
                });
                assert!(registration_succeeded.get(), "benchmark deadline should register");
            },
        );
    }
    group.finish();
}

/// Benchmarks waking and completing many waiters at one shared deadline.
fn benchmark_batch_deadline_completion(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("manual_timer/batch_deadline_completion");
    for waiter_count in WAITER_COUNTS {
        group.throughput(Throughput::Elements(waiter_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(waiter_count),
            &waiter_count,
            |bencher, &waiter_count| {
                let completion_succeeded = Cell::new(true);
                bencher.iter_batched(
                    || {
                        let clock = ManualMonotonicClock::new_shared();
                        let timer = clock.new_timer();
                        let futures = (0..waiter_count)
                            .map(|_| timer.after(BATCH_DEADLINE).expect("benchmark deadline should register"))
                            .collect::<Vec<_>>();
                        (clock, futures)
                    },
                    |(clock, mut futures)| {
                        if clock.advance(BATCH_DEADLINE).is_err() {
                            completion_succeeded.set(false);
                            return;
                        }
                        let waker = Waker::noop();
                        let mut context = Context::from_waker(waker);
                        for future in &mut futures {
                            if !is_ready(future, &mut context) {
                                completion_succeeded.set(false);
                            }
                        }
                    },
                    BatchSize::SmallInput,
                );
                assert!(completion_succeeded.get(), "manual timer should complete after advance");
            },
        );
    }
    group.finish();
}

/// Benchmarks delivering staggered deadlines through repeated small advances.
fn benchmark_sequential_deadline_completion(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("manual_timer/sequential_deadline_completion");
    for waiter_count in WAITER_COUNTS {
        group.throughput(Throughput::Elements(waiter_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(waiter_count),
            &waiter_count,
            |bencher, &waiter_count| {
                let completion_succeeded = Cell::new(true);
                bencher.iter_batched(
                    || {
                        let clock = ManualMonotonicClock::new_shared();
                        let timer = clock.new_timer();
                        let futures = (1..=waiter_count)
                            .map(|step| {
                                timer
                                    .after(Duration::from_nanos(step as u64))
                                    .expect("benchmark deadline should register")
                            })
                            .collect::<Vec<_>>();
                        (clock, futures)
                    },
                    |(clock, mut futures)| {
                        let waker = Waker::noop();
                        let mut context = Context::from_waker(waker);
                        for future in &mut futures {
                            if clock.advance(Duration::from_nanos(1)).is_err() || !is_ready(future, &mut context) {
                                completion_succeeded.set(false);
                            }
                        }
                    },
                    BatchSize::SmallInput,
                );
                assert!(
                    completion_succeeded.get(),
                    "manual timer should complete after each advance"
                );
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = manual_timer_benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets =
        benchmark_registration_and_cancellation,
        benchmark_batch_deadline_completion,
        benchmark_sequential_deadline_completion
}
criterion_main!(manual_timer_benches);
