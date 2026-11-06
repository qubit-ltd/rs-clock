// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_clock::TimeError;
use qubit_clock::Timer;
use qubit_clock::TimerUnavailableError;
use qubit_clock::TokioTimer;

/// Verifies dropping the runtime sentinel publishes structured shutdown.
#[test]
fn test_tokio_runtime_shutdown_guard_signals_when_runtime_drops() {
    let future = {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("retained runtime should build");
        TokioTimer::from_handle(runtime.handle().clone())
            .after(Duration::from_secs(60))
            .expect("deadline should register")
    };
    let polling_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("polling runtime should build");

    let error = polling_runtime
        .block_on(future)
        .expect_err("dropped runtime should fail its deadline");

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::RuntimeShuttingDown,
        },
    ));
}
