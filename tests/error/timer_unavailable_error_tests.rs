// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;
use std::io;

use qubit_clock::TimeError;
use qubit_clock::TimerUnavailableError;

/// Verifies that worker-spawn failures retain their original I/O source.
#[test]
fn test_timer_unavailable_error_retains_worker_spawn_source() {
    let error = TimeError::TimerUnavailable {
        source: TimerUnavailableError::WorkerThreadSpawnFailed {
            source: io::Error::new(
                io::ErrorKind::OutOfMemory,
                "scheduler capacity exhausted",
            ),
        },
    };

    assert_eq!(
        "monotonic timer is unavailable: the scheduler worker thread could \
         not be spawned: scheduler capacity exhausted",
        error.to_string(),
    );
    let timer_error = error
        .source()
        .expect("timer unavailability should be the outer source");
    let io_error = timer_error
        .source()
        .and_then(|source| source.downcast_ref::<io::Error>())
        .expect("worker-spawn failure should retain io::Error");
    assert_eq!(io::ErrorKind::OutOfMemory, io_error.kind());
}

#[test]
fn test_timer_unavailable_error_reports_worker_termination() {
    let error = TimerUnavailableError::SchedulerWorkerTerminated;

    assert_eq!(
        "the scheduler worker thread terminated unexpectedly",
        error.to_string()
    );
    assert!(error.source().is_none());
}

/// Verifies that custom backends identify themselves and retain their source.
#[test]
fn test_timer_unavailable_error_retains_custom_backend_source() {
    let error = TimerUnavailableError::BackendUnavailable {
        backend: "test",
        source: Box::new(io::Error::other("offline")),
    };

    assert_eq!(
        "timer backend 'test' is unavailable: offline",
        error.to_string(),
    );
    let io_error = error
        .source()
        .and_then(|source| source.downcast_ref::<io::Error>())
        .expect("custom backend failure should retain io::Error");
    assert_eq!(io::ErrorKind::Other, io_error.kind());
}

/// Verifies that timer unavailability errors satisfy standard error bounds.
#[test]
fn test_timer_unavailable_error_implements_std_error() {
    fn assert_error<T: Error>() {}
    assert_error::<TimerUnavailableError>();
}

/// Verifies that disabled Tokio time drivers have a stable typed error.
#[cfg(feature = "tokio")]
#[test]
fn test_timer_unavailable_error_reports_disabled_time_driver() {
    let error = TimerUnavailableError::TimeDriverDisabled;

    assert_eq!(
        "the asynchronous runtime time driver is disabled",
        error.to_string(),
    );
    assert!(error.source().is_none());
}

/// Verifies that retained Tokio runtime shutdown has a stable typed error.
#[cfg(feature = "tokio")]
#[test]
fn test_timer_unavailable_error_reports_runtime_shutdown() {
    let error = TimerUnavailableError::RuntimeShuttingDown;

    assert_eq!(
        "the asynchronous runtime shut down before the timer future completed",
        error.to_string()
    );
    assert!(error.source().is_none());
}
