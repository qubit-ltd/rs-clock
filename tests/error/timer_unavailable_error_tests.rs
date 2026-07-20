// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[cfg(feature = "tokio")]
use qubit_clock::{
    MonotonicClock,
    Timer,
    TokioMonotonicClock,
    TokioRuntimeError,
    TokioTimer,
};
use qubit_clock::{
    TimeError,
    TimerUnavailableError,
};
use std::{
    error::Error,
    io,
};

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

/// Verifies that Tokio runtime lookup failures remain in the source chain.
#[cfg(feature = "tokio")]
#[test]
fn test_timer_unavailable_error_retains_tokio_runtime_source() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let (timer, deadline) = runtime.block_on(async {
        let clock = TokioMonotonicClock::current();
        let deadline = clock.now();
        (TokioTimer::from_clock(&clock), deadline)
    });
    let error = match timer.at(deadline) {
        Ok(_) => panic!("runtime-less registration should fail"),
        Err(error) => error,
    };

    let timer_error = error
        .source()
        .expect("timer unavailability should be the outer source");
    let runtime_error = timer_error
        .source()
        .and_then(|source| source.downcast_ref::<TokioRuntimeError>())
        .expect("Tokio runtime error should be retained");
    let lookup_error = runtime_error
        .source()
        .and_then(|source| {
            source.downcast_ref::<tokio::runtime::TryCurrentError>()
        })
        .expect("Tokio runtime lookup error should be retained");
    assert!(lookup_error.is_missing_context());
}
