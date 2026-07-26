// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{Timer, TokioTimer};
#[cfg(tokio_unstable)]
use std::sync::{
    OnceLock, Weak,
    atomic::{AtomicUsize, Ordering},
};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

/// Number of registrations used to expose per-deadline liveness tasks.
const LIVENESS_REGISTRATION_COUNT: usize = 1_024;

/// Number of independent timers used to expose per-timer liveness tasks.
const LIVENESS_TIMER_COUNT: usize = 64;

/// Verifies a Tokio spawn hook can register a deadline on the same timer.
#[cfg(tokio_unstable)]
#[test]
fn test_tokio_runtime_liveness_allows_reentrant_task_hook() {
    let timer_slot = Arc::new(OnceLock::<Weak<TokioTimer>>::new());
    let hook_registrations = Arc::new(AtomicUsize::new(0));
    let mut runtime_builder = tokio::runtime::Builder::new_current_thread();
    runtime_builder.enable_time();
    runtime_builder.on_task_spawn({
        let timer_slot = Arc::clone(&timer_slot);
        let hook_registrations = Arc::clone(&hook_registrations);
        move |_| {
            hook_registrations.fetch_add(1, Ordering::Relaxed);
            let timer = timer_slot
                .get()
                .expect("timer should be published before spawning")
                .upgrade()
                .expect("timer should remain alive while registering");
            drop(
                timer
                    .after(Duration::from_secs(60))
                    .expect("spawn hook should register a reentrant deadline"),
            );
        }
    });
    let runtime = runtime_builder.build().expect("runtime should build");
    let timer = Arc::new(TokioTimer::from_handle(runtime.handle().clone()));
    timer_slot
        .set(Arc::downgrade(&timer))
        .expect("timer should be published once");
    let initial_tasks = runtime.metrics().num_alive_tasks();

    let future = timer
        .after(Duration::from_secs(60))
        .expect("initial deadline should register");

    assert_eq!(1, hook_registrations.load(Ordering::Relaxed));
    assert_eq!(initial_tasks + 1, runtime.metrics().num_alive_tasks());
    drop(future);
    drop(timer);
    drop(timer_slot);
    runtime.block_on(tokio::task::yield_now());
    assert_eq!(initial_tasks, runtime.metrics().num_alive_tasks());
}

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

/// Verifies independent timers on one runtime share one liveness task.
#[test]
fn test_tokio_runtime_liveness_is_shared_across_timers() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let initial_tasks = runtime.metrics().num_alive_tasks();
    let timers = (0..LIVENESS_TIMER_COUNT)
        .map(|_| TokioTimer::from_handle(runtime.handle().clone()))
        .collect::<Vec<_>>();
    let futures = timers
        .iter()
        .map(|timer| {
            timer
                .after(Duration::from_secs(60))
                .expect("future deadline should register")
        })
        .collect::<Vec<_>>();

    assert_eq!(
        initial_tasks + 1,
        runtime.metrics().num_alive_tasks(),
        "one runtime should retain one liveness task across timers",
    );
    drop(futures);
    drop(timers);
    runtime.block_on(tokio::task::yield_now());
    assert_eq!(initial_tasks, runtime.metrics().num_alive_tasks());
}

/// Verifies independent runtimes never share one liveness task.
#[test]
fn test_tokio_runtime_liveness_is_not_shared_across_runtimes() {
    let first_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("first runtime should build");
    let second_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("second runtime should build");
    let first_initial_tasks = first_runtime.metrics().num_alive_tasks();
    let second_initial_tasks = second_runtime.metrics().num_alive_tasks();
    let first_timer = TokioTimer::from_handle(first_runtime.handle().clone());
    let second_timer = TokioTimer::from_handle(second_runtime.handle().clone());
    let first_future = first_timer
        .after(Duration::from_secs(60))
        .expect("first deadline should register");
    let second_future = second_timer
        .after(Duration::from_secs(60))
        .expect("second deadline should register");

    assert_eq!(
        first_initial_tasks + 1,
        first_runtime.metrics().num_alive_tasks(),
    );
    assert_eq!(
        second_initial_tasks + 1,
        second_runtime.metrics().num_alive_tasks(),
    );

    drop(first_future);
    drop(first_timer);
    first_runtime.block_on(tokio::task::yield_now());
    assert_eq!(
        first_initial_tasks,
        first_runtime.metrics().num_alive_tasks(),
    );
    drop(second_future);
    drop(second_timer);
    second_runtime.block_on(tokio::task::yield_now());
    assert_eq!(
        second_initial_tasks,
        second_runtime.metrics().num_alive_tasks(),
    );
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

/// Verifies concurrent first use shares liveness across independent timers.
#[test]
fn test_tokio_runtime_liveness_concurrent_timers_share_one_task() {
    const THREAD_COUNT: usize = 8;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let barrier = Arc::new(Barrier::new(THREAD_COUNT));
    let initial_tasks = runtime.metrics().num_alive_tasks();
    let threads = (0..THREAD_COUNT)
        .map(|_| {
            let runtime_handle = runtime.handle().clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let timer = TokioTimer::from_handle(runtime_handle);
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

    assert_eq!(
        initial_tasks + 1,
        runtime.metrics().num_alive_tasks(),
        "one runtime should retain one liveness task after concurrent first use",
    );
    drop(futures);
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
