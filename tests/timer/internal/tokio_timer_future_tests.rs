// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow coverage-cfg

use std::time::Duration;

use qubit_clock::Timer;
use qubit_clock::TokioTimer;
#[cfg(coverage)]
use qubit_clock::panic_next_tokio_timer_sleep_poll;

/// Verifies the concrete Tokio timer future completes its native sleep.
#[test]
fn test_tokio_timer_future_completes_at_deadline() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let future = timer.after(Duration::from_secs(1)).expect("deadline should register");

    runtime.block_on(async {
        tokio::time::advance(Duration::from_secs(1)).await;
        future.await.expect("deadline should complete");
    });
}

/// Verifies an unexpected sleep panic is resumed while the runtime is live.
#[cfg(coverage)]
#[test]
fn test_tokio_timer_future_does_not_mask_unexpected_sleep_panic() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let future = timer
        .after(Duration::from_secs(60))
        .expect("future deadline should register");
    panic_next_tokio_timer_sleep_poll();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(future)));

    assert!(result.is_err(), "unexpected Tokio sleep panic must resume");
}
