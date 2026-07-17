// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    TimeError,
    Timer,
    TokioMonotonicClock,
    TokioTimer,
};
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn test_tokio_timer_latches_before_first_poll() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_secs(8))
        .expect("Tokio deadline should register");

    tokio::time::advance(Duration::from_secs(8)).await;
    future.await;
}

#[test]
fn test_tokio_timer_reports_missing_driver_at_registration() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);

    assert_eq!(
        Err(TimeError::TimerUnavailable),
        timer.after(Duration::from_secs(1)).map(drop),
    );
}

#[test]
fn test_tokio_timer_reports_disabled_time_driver_at_registration() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime should build");

    runtime.block_on(async {
        let clock = TokioMonotonicClock::new();
        let timer = TokioTimer::from_clock(&clock);
        assert_eq!(
            Err(TimeError::TimerUnavailable),
            timer.after(Duration::from_secs(1)).map(drop),
        );
    });
}

#[tokio::test]
async fn test_tokio_timer_returns_ready_future_for_reached_deadline() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);
    let deadline = clock.now();
    let future = timer
        .at(deadline)
        .expect("reached deadline should register successfully");

    future.await;
}

#[tokio::test]
async fn test_tokio_timer_rejects_foreign_deadline_immediately() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();

    let error = match timer.at(foreign) {
        Ok(_) => panic!("foreign deadline should fail at registration"),
        Err(error) => error,
    };

    assert_eq!(
        TimeError::ClockDomainMismatch {
            expected,
            actual: foreign.domain(),
        },
        error,
    );
}

#[tokio::test]
async fn test_tokio_timer_retains_domain_after_source_is_dropped() {
    let (timer, domain) = {
        let clock = TokioMonotonicClock::new();
        let domain = clock.now().domain();
        (TokioTimer::from_clock(&clock), domain)
    };

    assert_eq!(domain, timer.clock().now().domain());
}
