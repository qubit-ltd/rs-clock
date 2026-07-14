// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    AsyncSleeper,
    ManualAsyncSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    TimeError,
};
use std::sync::Arc;
use std::time::Duration;

/// Requires a value that owns everything needed for its full lifetime.
fn assert_static<T: 'static>(_value: T) {}

#[tokio::test]
async fn test_async_sleeper_supports_trait_object() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper: Arc<dyn AsyncSleeper> =
        Arc::new(ManualAsyncSleeper::from_clock(Arc::clone(&clock)));

    assert_eq!(clock.now().domain(), sleeper.clock().now().domain());
    sleeper
        .sleep_for_async(Duration::ZERO)
        .await
        .expect("zero sleep should complete immediately");
}

#[tokio::test]
async fn test_async_sleeper_box_delegates_to_inner_sleeper() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper: Box<dyn AsyncSleeper> =
        Box::new(ManualAsyncSleeper::from_clock(Arc::clone(&clock)));

    assert_eq!(clock.now().domain(), sleeper.clock().now().domain());
    sleeper
        .sleep_until_async(clock.now())
        .await
        .expect("reached deadline should complete immediately");
}

#[tokio::test]
async fn test_async_sleeper_reports_relative_deadline_overflow() {
    let clock = Arc::new(ManualMonotonicClock::new());
    clock
        .advance(Duration::MAX)
        .expect("maximum duration should fit from zero");
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));

    assert_eq!(
        Err(TimeError::InstantOverflow),
        sleeper.sleep_for_async(Duration::from_nanos(1)).await,
    );
}

#[test]
fn test_async_sleeper_returns_static_future() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(clock);
    let sleep = sleeper.sleep_for_async(Duration::ZERO);

    assert_static(sleep);
}
