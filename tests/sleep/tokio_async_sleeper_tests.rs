// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    AsyncSleeper,
    MonotonicClock,
    TimeError,
    TokioAsyncSleeper,
    TokioMonotonicClock,
};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_tokio_async_sleeper_future_can_be_created_outside_runtime() {
    let clock = Arc::new(TokioMonotonicClock::new());
    let sleeper = TokioAsyncSleeper::from_clock(clock);

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));

    drop(sleep);
}

#[tokio::test(start_paused = true)]
async fn test_tokio_async_sleeper_uses_supplied_clock_domain() {
    let clock = Arc::new(TokioMonotonicClock::new());
    let sleeper = TokioAsyncSleeper::from_clock(Arc::clone(&clock));
    assert_eq!(clock.now().domain_id(), sleeper.now().domain_id());
}

#[tokio::test(start_paused = true)]
async fn test_tokio_async_sleeper_follows_tokio_time() {
    let clock = Arc::new(TokioMonotonicClock::new());
    let sleeper = TokioAsyncSleeper::from_clock(Arc::clone(&clock));
    let start = sleeper.now();

    sleeper
        .sleep_for_async(Duration::from_secs(5))
        .await
        .expect("Tokio sleep should complete");

    assert_eq!(
        Duration::from_secs(5),
        sleeper
            .now()
            .duration_since(start)
            .expect("instants should share one domain"),
    );
}

#[tokio::test(start_paused = true)]
async fn test_tokio_async_sleeper_rejects_foreign_deadline() {
    let clock = Arc::new(TokioMonotonicClock::new());
    let sleeper = TokioAsyncSleeper::from_clock(Arc::clone(&clock));
    let foreign = TokioMonotonicClock::new().now();

    assert!(matches!(
        sleeper.sleep_until_async(foreign).await,
        Err(TimeError::ClockDomainMismatch { .. }),
    ));
}

#[tokio::test(start_paused = true)]
async fn test_tokio_async_sleeper_reports_native_deadline_overflow() {
    let clock = Arc::new(TokioMonotonicClock::new());
    let sleeper = TokioAsyncSleeper::from_clock(Arc::clone(&clock));
    let now = clock.now();
    let remaining = Duration::MAX
        .checked_sub(now.elapsed_since_origin())
        .expect("current elapsed should be below Duration maximum");
    let deadline = now
        .checked_add(remaining)
        .expect("maximum monotonic deadline should be representable");

    assert_eq!(
        Err(TimeError::InstantOverflow),
        sleeper.sleep_until_async(deadline).await,
    );
}
