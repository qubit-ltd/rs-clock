// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    Timer,
    TokioMonotonicClock,
    TokioTimer,
};
use std::task::{
    Context,
    Poll,
    Waker,
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

#[test]
fn test_tokio_timer_returns_reached_deadline_outside_runtime() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);
    let deadline = clock.now();
    let mut future = timer
        .at(deadline)
        .expect("reached deadline should not require a runtime");
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Ready(()), future.as_mut().poll(&mut context));
}

/// Verifies that native overflow is reported before Tokio runtime validation.
#[test]
fn test_tokio_timer_reports_native_instant_overflow() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);
    let deadline = MonotonicInstant::new(clock.now().domain(), Duration::MAX);

    let error = match timer.at(deadline) {
        Ok(_) => panic!("overflowing native deadline should fail"),
        Err(error) => error,
    };

    assert_eq!(TimeError::InstantOverflow, error);
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
