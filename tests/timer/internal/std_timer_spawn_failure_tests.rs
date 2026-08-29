// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow coverage-cfg

#[cfg(coverage)]
use std::time::Duration;

#[cfg(coverage)]
use qubit_clock::StdMonotonicClock;
#[cfg(coverage)]
use qubit_clock::StdTimer;
#[cfg(coverage)]
use qubit_clock::TimeError;
#[cfg(coverage)]
use qubit_clock::Timer;
#[cfg(coverage)]
use qubit_clock::TimerUnavailableError;
#[cfg(coverage)]
use qubit_clock::fail_next_std_timer_worker_spawn;

/// Verifies native worker-spawn failure preserves its source, rolls back the
/// attempted registration, and permits a later worker startup.
#[cfg(coverage)]
#[test]
fn test_std_timer_worker_spawn_failure_rolls_back_and_preserves_source() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    fail_next_std_timer_worker_spawn();

    let error = match timer.after(Duration::from_secs(1)) {
        Ok(_) => panic!("injected worker spawn should fail"),
        Err(error) => error,
    };
    match error {
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::WorkerThreadSpawnFailed { source },
        } => {
            assert_eq!(source.kind(), std::io::ErrorKind::Other);
            assert_eq!(source.to_string(), "injected standard Timer worker spawn failure",);
        }
        other => panic!("expected worker spawn failure, got {other}"),
    }

    let registration = timer.after(Duration::from_secs(1));
    assert!(registration.is_ok(), "worker startup should recover");
}
