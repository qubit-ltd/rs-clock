// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    ManualTimer,
    MonotonicClock,
    TimeError,
    Timer,
};
use std::task::{
    Context,
    Poll,
    Waker,
};

#[test]
fn test_manual_timer_returns_ready_future_for_reached_deadline() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let mut future = timer
        .at(clock.now())
        .expect("reached deadline should register successfully");
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(()),
    ));
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_timer_rejects_foreign_deadline_at_registration() {
    let clock = ManualMonotonicClock::new_shared();
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();
    let timer = ManualTimer::from_clock(clock.as_ref());

    let error = match timer.at(foreign) {
        Ok(_) => panic!("foreign deadline should be rejected immediately"),
        Err(error) => error,
    };

    let TimeError::ClockDomainMismatch {
        expected: actual_expected,
        actual,
    } = error
    else {
        panic!("foreign deadline should report a domain mismatch");
    };
    assert_eq!(expected, actual_expected);
    assert_eq!(foreign.domain(), actual);
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_timer_retains_same_domain_after_source_clock_is_dropped() {
    let (timer, expected_domain) = {
        let clock = ManualMonotonicClock::new();
        let expected_domain = clock.now().domain();
        (ManualTimer::from_clock(&clock), expected_domain)
    };

    assert_eq!(expected_domain, timer.clock().now().domain());
}
