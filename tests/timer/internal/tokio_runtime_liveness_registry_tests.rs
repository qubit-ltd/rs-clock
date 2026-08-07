// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_clock::Timer;
use qubit_clock::TokioTimer;

/// Verifies registry reuse is observable across independent timers.
#[test]
fn test_tokio_runtime_liveness_registry_reuses_one_sentinel() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let initial_tasks = runtime.metrics().num_alive_tasks();
    let first_timer = TokioTimer::from_handle(runtime.handle().clone());
    let second_timer = TokioTimer::from_handle(runtime.handle().clone());
    let first_future = first_timer
        .after(Duration::from_secs(60))
        .expect("first deadline should register");
    let second_future = second_timer
        .after(Duration::from_secs(60))
        .expect("second deadline should register");

    assert_eq!(initial_tasks + 1, runtime.metrics().num_alive_tasks());

    drop(first_future);
    drop(second_future);
    drop(first_timer);
    drop(second_timer);
    runtime.block_on(tokio::task::yield_now());
    assert_eq!(initial_tasks, runtime.metrics().num_alive_tasks());
}
