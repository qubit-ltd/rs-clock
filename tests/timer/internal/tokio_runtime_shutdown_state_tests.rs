// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{TimeError, Timer, TimerUnavailableError, TokioTimer};
use std::time::Duration;

/// Verifies one shutdown publication reaches every pending deadline.
#[test]
fn test_tokio_runtime_shutdown_state_notifies_all_pending_deadlines() {
    const DEADLINE_COUNT: usize = 8;

    let futures = {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("retained runtime should build");
        let timer = TokioTimer::from_handle(runtime.handle().clone());
        (0..DEADLINE_COUNT)
            .map(|_| {
                timer
                    .after(Duration::from_secs(60))
                    .expect("deadline should register")
            })
            .collect::<Vec<_>>()
    };
    let polling_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("polling runtime should build");

    for future in futures {
        let error = polling_runtime
            .block_on(future)
            .expect_err("dropped runtime should fail every deadline");
        assert!(matches!(
            error,
            TimeError::TimerUnavailable {
                source: TimerUnavailableError::RuntimeShuttingDown,
            },
        ));
    }
}
