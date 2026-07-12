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

#[tokio::test]
async fn test_manual_async_sleeper_uses_supplied_clock_domain() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    assert_eq!(clock.now().domain_id(), sleeper.now().domain_id());
}

#[tokio::test]
async fn test_manual_async_sleeper_rejects_foreign_deadline() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let foreign = ManualMonotonicClock::new().now();

    assert_eq!(
        Err(TimeError::ClockDomainMismatch {
            expected: clock.now().domain_id(),
            actual: foreign.domain_id(),
        }),
        sleeper.sleep_until_async(foreign).await,
    );
    assert_eq!(0, sleeper.pending_waiters());
}

#[tokio::test]
async fn test_manual_async_sleeper_deadline_is_measured_at_call_time() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let sleep = sleeper.sleep_for_async(Duration::from_secs(10));

    assert_eq!(1, sleeper.pending_waiters());
    clock
        .advance(Duration::from_secs(5))
        .expect("short advance should succeed");

    tokio::pin!(sleep);
    tokio::select! {
        result = &mut sleep => panic!(
            "sleep completed before call-time deadline: {result:?}"
        ),
        () = tokio::task::yield_now() => {},
    }

    clock
        .advance(Duration::from_secs(5))
        .expect("short advance should succeed");
    sleep
        .await
        .expect("sleep should complete at call-time deadline");
}

#[tokio::test]
async fn test_manual_async_sleeper_wakes_all_waiters_at_deadline() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let first = sleeper.sleep_for_async(Duration::from_secs(3));
    let second = sleeper.sleep_for_async(Duration::from_secs(3));

    assert_eq!(2, sleeper.pending_waiters());
    assert_eq!(
        Some(
            clock
                .now()
                .checked_add(Duration::from_secs(3))
                .expect("short deadline should be representable"),
        ),
        sleeper.next_deadline(),
    );

    clock
        .advance(Duration::from_secs(3))
        .expect("short advance should succeed");
    let (first_result, second_result) = tokio::join!(first, second);
    first_result.expect("first sleep should complete");
    second_result.expect("second sleep should complete");
    assert_eq!(0, sleeper.pending_waiters());
}
