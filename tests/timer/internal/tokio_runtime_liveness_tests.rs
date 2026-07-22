// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow coverage-cfg

#[cfg(coverage)]
use qubit_clock::panic_next_tokio_timer_sleep_poll;
use qubit_clock::{
    Timer,
    TokioTimer,
};
use std::{
    sync::{
        Arc,
        Barrier,
    },
    thread,
    time::Duration,
};

/// Number of registrations used to expose per-deadline liveness tasks.
const LIVENESS_REGISTRATION_COUNT: usize = 1_024;

/// Verifies that all pending deadlines share one runtime-liveness task.
#[test]
fn test_tokio_runtime_liveness_is_shared_across_pending_deadlines() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let initial_tasks = runtime.metrics().num_alive_tasks();

    let futures = (0..LIVENESS_REGISTRATION_COUNT)
        .map(|_| {
            timer
                .after(Duration::from_secs(60))
                .expect("future deadline should register")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        initial_tasks + 1,
        runtime.metrics().num_alive_tasks(),
        "pending deadlines should retain one shared liveness task",
    );
    drop(futures);
    drop(timer);
    runtime.block_on(tokio::task::yield_now());
    assert_eq!(initial_tasks, runtime.metrics().num_alive_tasks());
}

/// Verifies that dropping one deadline leaves liveness retained by its timer.
#[test]
fn test_tokio_runtime_liveness_is_retained_by_timer_after_future_drop() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let initial_tasks = runtime.metrics().num_alive_tasks();
    let cancelled_future = timer
        .after(Duration::from_secs(60))
        .expect("future deadline should register");

    assert_eq!(initial_tasks + 1, runtime.metrics().num_alive_tasks());
    drop(cancelled_future);
    assert_eq!(
        initial_tasks + 1,
        runtime.metrics().num_alive_tasks(),
        "timer should retain the shared liveness task after a deadline is dropped",
    );

    let future = timer
        .after(Duration::from_secs(1))
        .expect("later future deadline should register");
    runtime.block_on(async {
        tokio::time::advance(Duration::from_secs(1)).await;
        future.await.expect("later future should complete");
    });

    drop(timer);
    runtime.block_on(tokio::task::yield_now());
    assert_eq!(initial_tasks, runtime.metrics().num_alive_tasks());
}

/// Verifies that concurrent first use initializes only one liveness task.
#[test]
fn test_tokio_runtime_liveness_initializes_once_under_concurrency() {
    const THREAD_COUNT: usize = 8;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer = Arc::new(TokioTimer::from_handle(runtime.handle().clone()));
    let barrier = Arc::new(Barrier::new(THREAD_COUNT));
    let initial_tasks = runtime.metrics().num_alive_tasks();
    let threads = (0..THREAD_COUNT)
        .map(|_| {
            let timer = Arc::clone(&timer);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                timer
                    .after(Duration::from_secs(60))
                    .expect("concurrent deadline should register")
            })
        })
        .collect::<Vec<_>>();
    let futures = threads
        .into_iter()
        .map(|thread| thread.join().expect("registration thread should finish"))
        .collect::<Vec<_>>();

    assert_eq!(initial_tasks + 1, runtime.metrics().num_alive_tasks());
    drop(futures);
    drop(timer);
    runtime.block_on(tokio::task::yield_now());
    assert_eq!(initial_tasks, runtime.metrics().num_alive_tasks());
}

/// Verifies that a pending deadline retains shared liveness after its timer is
/// dropped.
#[test]
fn test_tokio_runtime_liveness_is_retained_by_pending_future() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let future = timer
        .after(Duration::from_secs(1))
        .expect("future deadline should register");
    drop(timer);

    runtime.block_on(async {
        tokio::time::advance(Duration::from_secs(1)).await;
        future.await.expect("retained future should complete");
    });
}

/// Verifies that an unexpected sleep panic is resumed while the runtime is
/// still live.
#[cfg(coverage)]
#[test]
fn test_tokio_runtime_liveness_does_not_mask_unexpected_sleep_panic() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let future = timer
        .after(Duration::from_secs(60))
        .expect("future deadline should register");
    panic_next_tokio_timer_sleep_poll();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(future)
    }));

    assert!(result.is_err(), "unexpected Tokio sleep panic must resume");
}
