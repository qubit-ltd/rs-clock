// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Measures Tokio Timer registration, cancellation, and completion costs.

use criterion::{
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_clock::{
    Timer,
    TokioTimer,
};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Number of futures registered and cancelled in one measured batch.
const REGISTRATIONS_PER_BATCH: usize = 256;

/// Paused-time deadline used by registration and cancellation batches.
const CANCELLATION_DEADLINE: Duration = Duration::from_secs(60);

/// Paused-time deadline advanced to completion in one iteration.
const COMPLETION_DEADLINE: Duration = Duration::from_millis(1);

/// Registers and cancels one batch, then drives cancellation cleanup.
///
/// # Parameters
///
/// * `timer` - Tokio Timer bound to `runtime`.
/// * `runtime` - Current-thread runtime that owns the timer tasks.
///
/// # Panics
///
/// Panics when a deadline cannot be registered.
fn register_and_cancel_batch(timer: &TokioTimer, runtime: &Runtime) {
    let futures = (0..REGISTRATIONS_PER_BATCH)
        .map(|_| {
            timer
                .after(CANCELLATION_DEADLINE)
                .expect("benchmark deadline should register")
        })
        .collect::<Vec<_>>();
    drop(futures);
    runtime.block_on(tokio::task::yield_now());
}

/// Registers and completes one future using paused Tokio time.
///
/// # Parameters
///
/// * `timer` - Tokio Timer bound to `runtime`.
/// * `runtime` - Current-thread runtime that owns the timer task.
///
/// # Panics
///
/// Panics when registration or completion reports an error.
fn complete_deadline(timer: &TokioTimer, runtime: &Runtime) {
    let future = timer
        .after(COMPLETION_DEADLINE)
        .expect("benchmark deadline should register");
    runtime.block_on(async move {
        tokio::time::advance(COMPLETION_DEADLINE).await;
        future.await.expect("benchmark deadline should complete");
    });
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
    let mut group = criterion.benchmark_group("tokio_timer");

    group.throughput(Throughput::Elements(REGISTRATIONS_PER_BATCH as u64));
    group.bench_function("registration_and_cancellation", |bencher| {
        bencher.iter(|| register_and_cancel_batch(&timer, &runtime));
    });

    group.throughput(Throughput::Elements(1));
    group.bench_function("deadline_completion", |bencher| {
        bencher.iter(|| complete_deadline(&timer, &runtime));
    });
    group.finish();
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
