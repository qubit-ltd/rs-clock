// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Measures Tokio Timer registration, cancellation, and completion costs.

use criterion::{
    BatchSize,
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_clock::{
    Timer,
    TimerFuture,
    TokioTimer,
};
use std::{
    convert::Infallible,
    future::{
        Future,
        pending,
        poll_fn,
    },
    task::Poll,
    time::Duration,
};
use tokio::runtime::Runtime;
use tokio::task::JoinSet;

/// Batch sizes representing ordinary and high-cardinality timer use.
const BATCH_SIZES: [usize; 2] = [1_024, 10_240];

/// Timer populations representing monitor- and retry-style ownership.
const TIMER_COUNTS: [usize; 2] = [64, 1_024];

/// Paused-time deadline used by registration and cancellation batches.
const CANCELLATION_DEADLINE: Duration = Duration::from_secs(60);

/// Paused-time deadline advanced to completion in one iteration.
const COMPLETION_DEADLINE: Duration = Duration::from_millis(1);

/// Creates one boxed native Tokio sleep without a liveness sentinel.
///
/// # Parameters
///
/// * `duration` - Relative duration assigned to the sleep.
///
/// # Returns
///
/// A Timer-shaped future backed only by Tokio's native sleep.
fn native_sleep(duration: Duration) -> TimerFuture {
    Box::pin(async move {
        tokio::time::sleep(duration).await;
        Ok(())
    })
}

/// Creates the legacy per-deadline liveness-sentinel shape.
///
/// This benchmark-only implementation intentionally mirrors the production
/// design under evaluation so the baseline remains available after a possible
/// production optimization.
///
/// # Parameters
///
/// * `duration` - Relative duration assigned to the sleep.
///
/// # Returns
///
/// A Timer-shaped future backed by one sleep and one pending Tokio task.
fn sleep_with_per_deadline_sentinel(duration: Duration) -> TimerFuture {
    let mut sentinel = JoinSet::new();
    sentinel.spawn(pending::<Infallible>());
    let mut sleep = Box::pin(tokio::time::sleep(duration));
    Box::pin(poll_fn(move |context| {
        match sentinel.poll_join_next(context) {
            Poll::Pending => {}
            Poll::Ready(Some(Err(error))) if error.is_cancelled() => {
                panic!("benchmark runtime shut down before deadline completion")
            }
            Poll::Ready(Some(Err(error))) => {
                std::panic::resume_unwind(error.into_panic());
            }
            Poll::Ready(Some(Ok(never))) => match never {},
            Poll::Ready(None) => {
                panic!("benchmark sentinel task set became empty")
            }
        }
        sleep.as_mut().poll(context).map(Ok)
    }))
}

/// Registers and cancels one batch, then drives task cancellation cleanup.
///
/// # Parameters
///
/// * `batch_size` - Number of futures created in the measured batch.
/// * `runtime` - Current-thread runtime that owns native timer state and tasks.
/// * `register` - Factory creating one Timer-shaped future.
///
/// # Panics
///
/// Panics when the runtime cannot drive cancellation cleanup.
fn register_and_cancel_batch(
    batch_size: usize,
    runtime: &Runtime,
    mut register: impl FnMut() -> TimerFuture,
) {
    let futures = {
        let _runtime_guard = runtime.enter();
        (0..batch_size).map(|_| register()).collect::<Vec<_>>()
    };
    drop(futures);
    runtime.block_on(tokio::task::yield_now());
}

/// Registers and completes one batch using paused Tokio time.
///
/// # Parameters
///
/// * `batch_size` - Number of futures completed in the measured batch.
/// * `runtime` - Current-thread runtime that owns native timer state and tasks.
/// * `register` - Factory creating one Timer-shaped future.
///
/// # Panics
///
/// Panics when registration or completion reports an error.
fn complete_deadline_batch(
    batch_size: usize,
    runtime: &Runtime,
    mut register: impl FnMut() -> TimerFuture,
) {
    let futures = {
        let _runtime_guard = runtime.enter();
        (0..batch_size).map(|_| register()).collect::<Vec<_>>()
    };
    runtime.block_on(async move {
        tokio::time::advance(COMPLETION_DEADLINE).await;
        for future in futures {
            future.await.expect("benchmark deadline should complete");
        }
        tokio::task::yield_now().await;
    });
}

/// Counts Tokio tasks retained while one future batch remains resident.
///
/// # Parameters
///
/// * `batch_size` - Number of futures kept alive during the observation.
/// * `runtime` - Runtime whose live task count is sampled.
/// * `register` - Factory creating one Timer-shaped future.
///
/// # Returns
///
/// The increase in live runtime tasks while the futures remain resident.
///
/// # Panics
///
/// Panics if task cleanup cannot be driven by the benchmark runtime.
fn resident_task_increase(
    batch_size: usize,
    runtime: &Runtime,
    mut register: impl FnMut() -> TimerFuture,
) -> usize {
    let initial_tasks = runtime.metrics().num_alive_tasks();
    let futures = {
        let _runtime_guard = runtime.enter();
        (0..batch_size).map(|_| register()).collect::<Vec<_>>()
    };
    let resident_tasks = runtime
        .metrics()
        .num_alive_tasks()
        .saturating_sub(initial_tasks);
    drop(futures);
    runtime.block_on(tokio::task::yield_now());
    resident_tasks
}

/// Creates independent timers retaining one runtime handle.
///
/// # Parameters
///
/// * `timer_count` - Number of independent timers to create.
/// * `runtime` - Runtime whose handle is retained by each timer.
///
/// # Returns
///
/// Independent timers associated with the supplied runtime.
fn create_timers(timer_count: usize, runtime: &Runtime) -> Vec<TokioTimer> {
    let runtime_handle = runtime.handle().clone();
    (0..timer_count)
        .map(|_| TokioTimer::from_handle(runtime_handle.clone()))
        .collect()
}

/// Registers one deadline per timer, then releases all futures and timers.
///
/// # Parameters
///
/// * `timers` - Independent timers used for deadline registration.
/// * `runtime` - Runtime that owns the timer state and cleanup tasks.
///
/// # Panics
///
/// Panics when a deadline cannot be registered or cleanup cannot be driven.
fn register_and_cancel_many_timers(timers: Vec<TokioTimer>, runtime: &Runtime) {
    let futures = {
        let _runtime_guard = runtime.enter();
        timers
            .iter()
            .map(|timer| {
                timer
                    .after(CANCELLATION_DEADLINE)
                    .expect("Tokio Timer deadline should register")
            })
            .collect::<Vec<_>>()
    };
    drop(futures);
    drop(timers);
    runtime.block_on(tokio::task::yield_now());
}

/// Counts retained tasks for independent timers on one fresh runtime.
///
/// # Parameters
///
/// * `timer_count` - Number of independent timers observed.
///
/// # Returns
///
/// Increase in live runtime tasks while deadlines and timers remain resident.
///
/// # Panics
///
/// Panics when the observation runtime cannot be built, a deadline cannot be
/// registered, or cleanup cannot be driven.
fn many_timer_resident_task_increase(timer_count: usize) -> usize {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("task-observation runtime should build");
    let initial_tasks = runtime.metrics().num_alive_tasks();
    let timers = create_timers(timer_count, &runtime);
    let futures = {
        let _runtime_guard = runtime.enter();
        timers
            .iter()
            .map(|timer| {
                timer
                    .after(CANCELLATION_DEADLINE)
                    .expect("Tokio Timer deadline should register")
            })
            .collect::<Vec<_>>()
    };
    let resident_tasks = runtime
        .metrics()
        .num_alive_tasks()
        .saturating_sub(initial_tasks);
    drop(futures);
    drop(timers);
    runtime.block_on(tokio::task::yield_now());
    resident_tasks
}

/// Benchmarks Tokio Timer behavior affected by task-backed scheduling.
///
/// # Parameters
///
/// * `criterion` - Criterion registry receiving Tokio Timer measurements.
///
/// # Panics
///
/// Panics when the paused Tokio runtime cannot be built.
fn benchmark_tokio_timer(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("benchmark runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let largest_batch = BATCH_SIZES[BATCH_SIZES.len() - 1];
    let native_tasks = resident_task_increase(largest_batch, &runtime, || {
        native_sleep(CANCELLATION_DEADLINE)
    });
    let sentinel_tasks =
        resident_task_increase(largest_batch, &runtime, || {
            sleep_with_per_deadline_sentinel(CANCELLATION_DEADLINE)
        });
    let timer_tasks = resident_task_increase(largest_batch, &runtime, || {
        timer
            .after(CANCELLATION_DEADLINE)
            .expect("Tokio Timer deadline should register")
    });
    eprintln!(
        "resident task increase at {largest_batch} futures: native={native_tasks}, per_deadline_sentinel={sentinel_tasks}, tokio_timer={timer_tasks}",
    );
    let largest_timer_count = TIMER_COUNTS[TIMER_COUNTS.len() - 1];
    let many_timer_tasks =
        many_timer_resident_task_increase(largest_timer_count);
    eprintln!(
        "resident task increase at {largest_timer_count} independent timers: \
         tokio_timer={many_timer_tasks}",
    );

    let mut registration_group =
        criterion.benchmark_group("tokio_timer/registration_and_cancellation");
    for batch_size in BATCH_SIZES {
        registration_group.throughput(Throughput::Elements(batch_size as u64));
        registration_group.bench_with_input(
            BenchmarkId::new("native_sleep", batch_size),
            &batch_size,
            |bencher, batch_size| {
                bencher.iter(|| {
                    register_and_cancel_batch(*batch_size, &runtime, || {
                        native_sleep(CANCELLATION_DEADLINE)
                    });
                });
            },
        );
        registration_group.bench_with_input(
            BenchmarkId::new("per_deadline_sentinel", batch_size),
            &batch_size,
            |bencher, batch_size| {
                bencher.iter(|| {
                    register_and_cancel_batch(*batch_size, &runtime, || {
                        sleep_with_per_deadline_sentinel(CANCELLATION_DEADLINE)
                    });
                });
            },
        );
        registration_group.bench_with_input(
            BenchmarkId::new("tokio_timer", batch_size),
            &batch_size,
            |bencher, batch_size| {
                bencher.iter(|| {
                    register_and_cancel_batch(*batch_size, &runtime, || {
                        timer
                            .after(CANCELLATION_DEADLINE)
                            .expect("Tokio Timer deadline should register")
                    });
                });
            },
        );
    }
    registration_group.finish();

    let many_timer_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("many-timer benchmark runtime should build");
    let mut many_timer_group = criterion.benchmark_group(
        "tokio_timer/many_timer_registration_and_cancellation",
    );
    for timer_count in TIMER_COUNTS {
        many_timer_group.throughput(Throughput::Elements(timer_count as u64));
        many_timer_group.bench_with_input(
            BenchmarkId::from_parameter(timer_count),
            &timer_count,
            |bencher, &timer_count| {
                bencher.iter_batched(
                    || create_timers(timer_count, &many_timer_runtime),
                    |timers| {
                        register_and_cancel_many_timers(
                            timers,
                            &many_timer_runtime,
                        );
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    many_timer_group.finish();

    let mut completion_group =
        criterion.benchmark_group("tokio_timer/deadline_completion");
    for batch_size in BATCH_SIZES {
        completion_group.throughput(Throughput::Elements(batch_size as u64));
        completion_group.bench_with_input(
            BenchmarkId::new("native_sleep", batch_size),
            &batch_size,
            |bencher, batch_size| {
                bencher.iter(|| {
                    complete_deadline_batch(*batch_size, &runtime, || {
                        native_sleep(COMPLETION_DEADLINE)
                    });
                });
            },
        );
        completion_group.bench_with_input(
            BenchmarkId::new("per_deadline_sentinel", batch_size),
            &batch_size,
            |bencher, batch_size| {
                bencher.iter(|| {
                    complete_deadline_batch(*batch_size, &runtime, || {
                        sleep_with_per_deadline_sentinel(COMPLETION_DEADLINE)
                    });
                });
            },
        );
        completion_group.bench_with_input(
            BenchmarkId::new("tokio_timer", batch_size),
            &batch_size,
            |bencher, batch_size| {
                bencher.iter(|| {
                    complete_deadline_batch(*batch_size, &runtime, || {
                        timer
                            .after(COMPLETION_DEADLINE)
                            .expect("Tokio Timer deadline should register")
                    });
                });
            },
        );
    }
    completion_group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = benchmark_tokio_timer
}
criterion_main!(benches);
