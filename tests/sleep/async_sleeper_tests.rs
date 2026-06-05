// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::time::{
    Duration,
    Instant,
};

use qubit_clock::sleep::{
    AsyncSleeper,
    MockSleeper,
    SystemSleeper,
};
use qubit_clock::{
    MockTimeError,
    MockWaiterKind,
};

#[tokio::test]
async fn test_system_sleep_for_async_waits_real_duration() {
    let sleeper = SystemSleeper::new();
    let start = Instant::now();

    sleeper.sleep_for_async(Duration::from_millis(2)).await;

    assert!(start.elapsed() >= Duration::from_millis(1));
}

#[tokio::test]
async fn test_mock_sleep_for_async_completes_after_advance() {
    let sleeper = MockSleeper::new();
    let sleep = sleeper.sleep_for_async(Duration::from_millis(100));

    sleeper.timeline().advance(Duration::from_millis(100));
    tokio::time::timeout(Duration::from_millis(50), sleep)
        .await
        .expect("mock async sleep should complete after time advances");
}

#[tokio::test]
async fn test_mock_sleep_for_async_zero_duration_completes_immediately() {
    let sleeper = MockSleeper::new();

    tokio::time::timeout(
        Duration::from_millis(50),
        sleeper.sleep_for_async(Duration::ZERO),
    )
    .await
    .expect("zero-duration mock async sleep should complete immediately");
    sleeper.timeline().reset().expect(
        "completed zero-duration sleep should not register an active waiter",
    );
}

#[tokio::test]
async fn test_mock_sleep_for_async_registers_active_waiter_until_completion() {
    let sleeper = MockSleeper::new();
    let timeline = sleeper.timeline();
    let sleep = sleeper.sleep_for_async(Duration::from_millis(100));

    assert!(
        timeline.wait_for_blocked_waiters(
            MockWaiterKind::Sleep,
            1,
            Duration::from_millis(20)
        ),
        "async sleep should register as an active mock waiter at call time",
    );
    assert_eq!(Err(MockTimeError::ActiveWaiters), timeline.reset());

    drop(sleep);
    timeline
        .reset()
        .expect("dropping async sleep should unregister the waiter");
}

#[tokio::test]
async fn test_mock_sleep_for_async_uses_elapsed_at_call_time() {
    let sleeper = MockSleeper::new();
    sleeper.timeline().advance(Duration::from_millis(10));
    let sleep = sleeper.sleep_for_async(Duration::from_millis(100));
    tokio::pin!(sleep);

    sleeper.timeline().advance(Duration::from_millis(99));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut sleep)
            .await
            .is_err(),
        "mock async sleep should be relative to elapsed at call time",
    );

    sleeper.timeline().advance(Duration::from_millis(1));
    tokio::time::timeout(Duration::from_millis(50), &mut sleep)
        .await
        .expect(
            "mock async sleep should complete after full relative duration",
        );
}
