/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use qubit_clock::timer::{
    AsyncTimer,
    BlockingTimer,
    MockTimer,
    MonotonicTimer,
    SystemTimer,
    TimerError,
    TimerInstant,
    TimerWaitOutcome,
};

struct ScriptedAsyncTimer {
    timer: MockTimer,
    outcomes: Mutex<VecDeque<TimerWaitOutcome>>,
}

impl ScriptedAsyncTimer {
    fn new(outcomes: impl IntoIterator<Item = TimerWaitOutcome>) -> Self {
        Self {
            timer: MockTimer::new(),
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        }
    }
}

impl MonotonicTimer for ScriptedAsyncTimer {
    fn timer_domain_id(&self) -> qubit_clock::timer::TimerDomainId {
        self.timer.timer_domain_id()
    }

    fn now(&self) -> qubit_clock::timer::TimerInstant {
        self.timer.now()
    }
}

impl AsyncTimer for ScriptedAsyncTimer {
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> Pin<Box<dyn Future<Output = Result<TimerWaitOutcome, TimerError>> + Send + 'a>> {
        let result = self.timer.duration_until(deadline).map(|_| {
            let mut outcomes = self
                .outcomes
                .lock()
                .expect("scripted timer outcomes should not be poisoned");
            outcomes
                .pop_front()
                .expect("scripted timer should have enough wait outcomes")
        });
        Box::pin(async move { result })
    }
}

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
        timer.notify_waiters();
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
async fn test_sleep_until_async_continues_after_notified_outcome() {
    let timer = ScriptedAsyncTimer::new([
        TimerWaitOutcome::Notified,
        TimerWaitOutcome::DeadlineReached,
    ]);
    let deadline = timer.deadline_after(Duration::from_millis(10));

    timer
        .sleep_until_async(deadline)
        .await
        .expect("scripted same-domain deadline should succeed");
}

#[tokio::test]
async fn test_sleep_until_async_propagates_foreign_deadline_error() {
    let timer = ScriptedAsyncTimer::new([]);
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
        timer.notify_waiters();
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
