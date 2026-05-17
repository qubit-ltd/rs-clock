/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::{Duration, Instant};

use qubit_clock::sleep::{AsyncSleeper, MockSleeper, SystemSleeper};

#[tokio::test]
async fn test_system_async_sleep_for_waits_real_duration() {
    let sleeper = SystemSleeper::new();
    let start = Instant::now();

    sleeper.async_sleep_for(Duration::from_millis(2)).await;

    assert!(start.elapsed() >= Duration::from_millis(1));
}

#[tokio::test]
async fn test_mock_async_sleep_for_completes_after_advance() {
    let sleeper = MockSleeper::new();
    let sleep = sleeper.async_sleep_for(Duration::from_millis(100));

    sleeper.advance(Duration::from_millis(100));
    tokio::time::timeout(Duration::from_millis(50), sleep)
        .await
        .expect("mock async sleep should complete after time advances");
}

#[tokio::test]
async fn test_mock_async_sleep_for_uses_elapsed_at_call_time() {
    let sleeper = MockSleeper::new();
    sleeper.advance(Duration::from_millis(10));
    let sleep = sleeper.async_sleep_for(Duration::from_millis(100));
    tokio::pin!(sleep);

    sleeper.advance(Duration::from_millis(99));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut sleep)
            .await
            .is_err(),
        "mock async sleep should be relative to elapsed at call time",
    );

    sleeper.advance(Duration::from_millis(1));
    tokio::time::timeout(Duration::from_millis(50), &mut sleep)
        .await
        .expect("mock async sleep should complete after full relative duration");
}
