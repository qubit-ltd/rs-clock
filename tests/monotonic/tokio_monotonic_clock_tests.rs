// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{MonotonicClock, TokioMonotonicClock, TokioRuntimeError};
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn test_tokio_monotonic_clock_follows_tokio_time() {
    let clock = TokioMonotonicClock::current();
    let start = clock.now();

    tokio::time::advance(Duration::from_secs(5)).await;

    assert_eq!(
        Duration::from_secs(5),
        clock
            .now()
            .duration_since(start)
            .expect("instants should share one domain"),
    );
}

/// Verifies that fallible construction reports a missing runtime context.
#[test]
fn test_tokio_monotonic_clock_try_current_reports_missing_runtime() {
    assert!(matches!(
        TokioMonotonicClock::try_current(),
        Err(TokioRuntimeError::NotEntered { .. }),
    ));
}

/// Verifies that infallible construction rejects a missing runtime context.
#[test]
#[should_panic(expected = "cannot create Tokio monotonic clock")]
fn test_tokio_monotonic_clock_current_panics_outside_runtime() {
    let _ = TokioMonotonicClock::current();
}

/// Verifies that fallible construction allocates an independent clock domain.
#[tokio::test(start_paused = true)]
async fn test_tokio_monotonic_clock_try_current_creates_clock() {
    let clock =
        TokioMonotonicClock::try_current().expect("entered runtime should create a Tokio clock");
    let other = TokioMonotonicClock::current();
    assert_ne!(clock.now().domain(), other.now().domain());
}

#[tokio::test(start_paused = true)]
async fn test_tokio_monotonic_clock_creates_same_domain_timer_directly() {
    let clock = TokioMonotonicClock::current();

    let timer = clock.new_timer();

    assert_eq!(clock.now().domain(), timer.clock().now().domain());
}

/// Verifies that an explicit handle remains usable without an ambient runtime.
#[test]
fn test_tokio_monotonic_clock_from_handle_follows_target_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("paused Tokio runtime should build");
    let clock = TokioMonotonicClock::from_handle(runtime.handle().clone());
    let start = clock.now();

    runtime.block_on(tokio::time::advance(Duration::from_secs(5)));

    assert_eq!(
        Duration::from_secs(5),
        clock
            .now()
            .duration_since(start)
            .expect("instants should share one domain"),
    );
}

/// Verifies that sampling uses the retained handle instead of the caller's
/// ambient runtime.
#[test]
fn test_tokio_monotonic_clock_samples_target_time_inside_another_runtime() {
    let target = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("target runtime should build");
    let other = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("other runtime should build");
    let clock = TokioMonotonicClock::from_handle(target.handle().clone());
    let start = other.block_on(async { clock.now() });

    target.block_on(tokio::time::advance(Duration::from_secs(7)));

    let end = other.block_on(async { clock.now() });

    assert_eq!(
        Duration::from_secs(7),
        end.duration_since(start)
            .expect("instants should share one domain"),
    );
}
