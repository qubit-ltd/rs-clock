/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::Duration;

use qubit_clock::timer::{
    AsyncTimer,
    MockTimer,
    SystemTimer,
    TimerDomain,
    TimerError,
    TimerWaitOutcome,
};

#[tokio::test]
async fn test_sleep_for_async_waits_until_mock_deadline_is_reached() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();

    let worker = tokio::spawn(async move {
        worker_timer
            .sleep_for_async(Duration::from_millis(100))
            .await
            .expect("sleeping with the timer's own deadline should succeed");
        worker_timer.now().elapsed_since_timer_start()
    });

    tokio::task::yield_now().await;
    assert!(
        !worker.is_finished(),
        "mock async sleep should not complete until time advances",
    );

    timer.advance(Duration::from_millis(100));

    assert_eq!(
        Duration::from_millis(100),
        worker.await.expect("worker task should finish cleanly"),
    );
}

#[tokio::test]
async fn test_sleep_until_async_ignores_notifications_until_deadline_is_reached() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();
    let deadline = timer.deadline_after(Duration::from_millis(100));

    let worker = tokio::spawn(async move {
        worker_timer
            .sleep_until_async(deadline)
            .await
            .expect("sleeping with the timer's own deadline should succeed");
        worker_timer.now().elapsed_since_timer_start()
    });

    tokio::task::yield_now().await;
    timer.notify_all_waiters();
    tokio::task::yield_now().await;
    assert!(
        !worker.is_finished(),
        "notification should not complete async sleep before the deadline",
    );

    timer.advance(Duration::from_millis(100));

    assert_eq!(
        Duration::from_millis(100),
        worker.await.expect("worker task should finish cleanly"),
    );
}

#[tokio::test]
async fn test_wait_until_async_can_be_notified_before_deadline() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();
    let deadline = timer.deadline_after(Duration::from_millis(100));

    let worker = tokio::spawn(async move {
        worker_timer
            .wait_until_async(deadline)
            .await
            .expect("deadline belongs to the worker timer clone")
    });

    tokio::task::yield_now().await;

    for _ in 0..100 {
        timer.notify_all_waiters();
        if worker.is_finished() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let outcome = tokio::time::timeout(Duration::from_secs(1), worker)
        .await
        .expect("notification should wake async wait promptly")
        .expect("worker task should finish cleanly");

    assert_eq!(TimerWaitOutcome::Notified, outcome);
}

#[tokio::test]
async fn test_wait_for_async_can_be_notified_before_deadline() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();

    let worker = tokio::spawn(async move {
        worker_timer
            .wait_for_async(Duration::from_millis(100))
            .await
            .expect("waiting with the timer's own deadline should succeed")
    });

    tokio::task::yield_now().await;
    for _ in 0..100 {
        timer.notify_all_waiters();
        if worker.is_finished() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let outcome = tokio::time::timeout(Duration::from_secs(1), worker)
        .await
        .expect("notification should wake async wait promptly")
        .expect("worker task should finish cleanly");

    assert_eq!(TimerWaitOutcome::Notified, outcome);
}

#[tokio::test]
async fn test_sleep_until_async_propagates_foreign_deadline_error() {
    let timer = MockTimer::new();
    let foreign_timer = MockTimer::new();

    let error = timer
        .sleep_until_async(foreign_timer.now())
        .await
        .expect_err("foreign deadline should be rejected");

    assert!(matches!(
        error,
        TimerError::TimerDomainMismatch {
            expected_domain_id: _,
            actual_domain_id: _
        }
    ));
}

#[tokio::test]
async fn test_system_timer_wait_until_async_reaches_deadline() {
    let timer = SystemTimer::new();

    assert_eq!(
        TimerWaitOutcome::DeadlineReached,
        timer
            .wait_until_async(timer.deadline_after(Duration::from_millis(1)))
            .await
            .expect("deadline belongs to this timer"),
    );
}

#[tokio::test]
async fn test_system_timer_wait_until_async_can_be_notified_before_deadline() {
    let timer = SystemTimer::new();
    let worker_timer = timer.clone();
    let deadline = timer.deadline_after(Duration::from_secs(5));

    let worker = tokio::spawn(async move {
        worker_timer
            .wait_until_async(deadline)
            .await
            .expect("deadline belongs to the worker timer clone")
    });

    tokio::task::yield_now().await;
    for _ in 0..100 {
        timer.notify_all_waiters();
        if worker.is_finished() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let outcome = tokio::time::timeout(Duration::from_secs(1), worker)
        .await
        .expect("notification should wake system async wait promptly")
        .expect("worker task should finish cleanly");

    assert_eq!(TimerWaitOutcome::Notified, outcome);
}
