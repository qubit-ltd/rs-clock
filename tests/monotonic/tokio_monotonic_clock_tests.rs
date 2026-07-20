// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    MonotonicClock,
    TokioMonotonicClock,
    TokioRuntimeError,
};
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
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
    let clock = TokioMonotonicClock::try_current()
        .expect("entered runtime should create a Tokio clock");
    let other = TokioMonotonicClock::current();
    assert_ne!(clock.now().domain(), other.now().domain());
}

#[tokio::test(start_paused = true)]
async fn test_tokio_monotonic_clock_creates_same_domain_timer_directly() {
    let clock = TokioMonotonicClock::current();

    let timer = clock.new_timer();

    assert_eq!(clock.now().domain(), timer.clock().now().domain());
}

/// Verifies that fallible sampling reports a missing runtime context.
#[test]
fn test_tokio_monotonic_clock_try_now_reports_missing_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let clock = runtime.block_on(async { TokioMonotonicClock::current() });

    assert!(matches!(
        clock.try_now(),
        Err(TokioRuntimeError::NotEntered { .. }),
    ));
}

/// Verifies that fallible sampling rejects an independent runtime.
#[test]
fn test_tokio_monotonic_clock_try_now_rejects_different_runtime() {
    let first = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("first runtime should build");
    let second = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("second runtime should build");
    let expected = first.handle().id();
    let actual = second.handle().id();
    let clock = first.block_on(async { TokioMonotonicClock::current() });

    let error = second
        .block_on(async { clock.try_now() })
        .expect_err("a different runtime should be rejected");

    assert!(matches!(
        error,
        TokioRuntimeError::Mismatch {
            expected: actual_expected,
            actual: actual_runtime,
        } if actual_expected == expected && actual_runtime == actual
    ));
}

/// Verifies that trait sampling rejects an independent runtime by panicking.
#[test]
fn test_tokio_monotonic_clock_trait_now_panics_in_different_runtime() {
    let first = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("first runtime should build");
    let second = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("second runtime should build");
    let clock = first.block_on(async { TokioMonotonicClock::current() });

    let result = second.block_on(async {
        catch_unwind(AssertUnwindSafe(|| MonotonicClock::now(&clock)))
    });

    assert!(
        result.is_err(),
        "trait sampling should reject another runtime"
    );
}
