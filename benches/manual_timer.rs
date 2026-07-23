// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Benchmarks manual-timer registration, cancellation, and deadline delivery.

use criterion::{
    BatchSize,
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    TimerFuture,
};
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;

/// Timer populations spanning small tests through high-cardinality workloads.
const WAITER_COUNTS: [usize; 8] = [1, 8, 32, 63, 64, 65, 128, 1_024];

/// Waiter population immediately before the ordered deadline index is enabled.
const DEADLINE_INDEX_THRESHOLD: usize = 64;

/// Deadline shared by every waiter in the batch-completion scenario.
const BATCH_DEADLINE: Duration = Duration::from_secs(1);

/// Polls a timer future once and requires successful completion.
///
/// # Parameters
///
/// * `future` - Future whose manual deadline has already been reached.
/// * `context` - Poll context backed by a no-op waker.
///
/// # Panics
///
/// Panics when the future remains pending or reports a timer error.
#[inline]
fn require_ready(future: &mut TimerFuture, context: &mut Context<'_>) {
    match future.as_mut().poll(context) {
        Poll::Ready(result) => result.expect("manual timer should complete"),
        Poll::Pending => panic!("manual timer should be ready after advance"),
    }
}

/// Benchmarks eager registration followed by cancellation through `Drop`.
fn benchmark_registration_and_cancellation(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("manual_timer/registration_and_cancellation");
    for waiter_count in WAITER_COUNTS {
        let clock = ManualMonotonicClock::new_shared();
        let timer = clock.new_timer();
        group.throughput(Throughput::Elements(waiter_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(waiter_count),
            &waiter_count,
            |bencher, &waiter_count| {
                bencher.iter(|| {
                    let mut futures = Vec::with_capacity(waiter_count);
                    for _ in 0..waiter_count {
                        futures.push(
                            timer
                                .after(BATCH_DEADLINE)
                                .expect("benchmark deadline should register"),
                        );
                    }
                    drop(futures);
                });
            },
        );
    }
    group.finish();
}

/// Benchmarks waking and completing many waiters at one shared deadline.
fn benchmark_batch_deadline_completion(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("manual_timer/batch_deadline_completion");
    for waiter_count in WAITER_COUNTS {
        group.throughput(Throughput::Elements(waiter_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(waiter_count),
            &waiter_count,
            |bencher, &waiter_count| {
                bencher.iter_batched(
                    || {
                        let clock = ManualMonotonicClock::new_shared();
                        let timer = clock.new_timer();
                        let futures = (0..waiter_count)
                            .map(|_| {
                                timer.after(BATCH_DEADLINE).expect(
                                    "benchmark deadline should register",
                                )
                            })
                            .collect::<Vec<_>>();
                        (clock, futures)
                    },
                    |(clock, mut futures)| {
                        clock
                            .advance(BATCH_DEADLINE)
                            .expect("manual clock should advance");
                        let waker = Waker::noop();
                        let mut context = Context::from_waker(waker);
                        for future in &mut futures {
                            require_ready(future, &mut context);
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks delivering staggered deadlines through repeated small advances.
fn benchmark_sequential_deadline_completion(criterion: &mut Criterion) {
    let mut group = criterion
        .benchmark_group("manual_timer/sequential_deadline_completion");
    for waiter_count in WAITER_COUNTS {
        group.throughput(Throughput::Elements(waiter_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(waiter_count),
            &waiter_count,
            |bencher, &waiter_count| {
                bencher.iter_batched(
                    || {
                        let clock = ManualMonotonicClock::new_shared();
                        let timer = clock.new_timer();
                        let futures = (1..=waiter_count)
                            .map(|step| {
                                timer
                                    .after(Duration::from_nanos(step as u64))
                                    .expect(
                                        "benchmark deadline should register",
                                    )
                            })
                            .collect::<Vec<_>>();
                        (clock, futures)
                    },
                    |(clock, mut futures)| {
                        let waker = Waker::noop();
                        let mut context = Context::from_waker(waker);
                        for future in &mut futures {
                            clock
                                .advance(Duration::from_nanos(1))
                                .expect("manual clock should advance");
                            require_ready(future, &mut context);
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks repeated registration and cancellation across the index boundary.
///
/// # Parameters
///
/// * `criterion` - Criterion registry receiving the threshold measurement.
///
/// # Panics
///
/// Panics when a manual Timer deadline cannot be registered.
fn benchmark_index_threshold_churn(criterion: &mut Criterion) {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let retained = (0..DEADLINE_INDEX_THRESHOLD)
        .map(|_| {
            timer
                .after(BATCH_DEADLINE)
                .expect("benchmark deadline should register")
        })
        .collect::<Vec<_>>();
    criterion.bench_function("manual_timer/index_threshold_churn", |bencher| {
        bencher.iter(|| {
            let threshold_crossing = timer
                .after(BATCH_DEADLINE)
                .expect("threshold-crossing deadline should register");
            drop(threshold_crossing);
        });
    });
    drop(retained);
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
        benchmark_sequential_deadline_completion,
        benchmark_index_threshold_churn
}
criterion_main!(manual_timer_benches);
