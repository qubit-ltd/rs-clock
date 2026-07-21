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
    StdMonotonicClock,
    StdTimer,
    TimeError,
    Timer,
};
use std::sync::Arc;
use std::task::{
    Context,
    Waker,
};
use std::time::Duration;

use super::internal::{
    DestructorPanickingWaker,
    PanickingWaker,
};
use super::support::block_on_timer_future;

#[test]
fn test_std_timer_new_registers_deadline() {
    let timer = StdTimer::new();
    let future = timer
        .after(Duration::from_millis(1))
        .expect("new timer should register a deadline");

    block_on_timer_future(future);
}

#[test]
fn test_std_timer_default_registers_deadline() {
    let timer = StdTimer::default();
    let future = timer
        .after(Duration::from_millis(1))
        .expect("default timer should register a deadline");

    block_on_timer_future(future);
}

#[test]
fn test_std_timer_returns_ready_future_for_reached_deadline() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let deadline = clock.now();
    std::thread::sleep(Duration::from_millis(1));
    let future = timer
        .at(deadline)
        .expect("reached deadline should register successfully");

    block_on_timer_future(future);
}

#[test]
fn test_std_timer_completes_real_short_deadline() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_millis(5))
        .expect("short deadline should register");

    block_on_timer_future(future);
}

#[test]
fn test_std_timer_continues_after_registered_waker_panics() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let mut panicking = timer
        .after(Duration::from_millis(10))
        .expect("panicking deadline should register");
    let waker = Waker::from(Arc::new(PanickingWaker));
    let mut context = Context::from_waker(&waker);
    assert!(panicking.as_mut().poll(&mut context).is_pending());

    let survivor = timer
        .after(Duration::from_millis(30))
        .expect("surviving deadline should register");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        block_on_timer_future(survivor);
        sender.send(()).expect("receiver should remain available");
    });

    assert_eq!(Ok(()), receiver.recv_timeout(Duration::from_secs(1)),);
}

#[test]
fn test_std_timer_survives_panicking_waker_payload_destructor() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let mut panicking = timer
        .after(Duration::from_millis(10))
        .expect("panicking deadline should register");
    let waker = Waker::from(Arc::new(DestructorPanickingWaker));
    let mut context = Context::from_waker(&waker);
    assert!(panicking.as_mut().poll(&mut context).is_pending());

    let survivor = timer
        .after(Duration::from_millis(30))
        .expect("surviving deadline should register");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        block_on_timer_future(survivor);
        sender.send(()).expect("receiver should remain available");
    });

    assert_eq!(Ok(()), receiver.recv_timeout(Duration::from_secs(1)),);
}

#[test]
fn test_std_timer_rejects_foreign_deadline_immediately() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();

    let error = match timer.at(foreign) {
        Ok(_) => panic!("foreign deadline should fail at registration"),
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
}

/// Verifies that an unrepresentable native deadline reports exact overflow.
#[test]
fn test_std_timer_reports_native_instant_overflow() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let deadline = MonotonicInstant::new(clock.now().domain(), Duration::MAX);

    let error = match timer.at(deadline) {
        Ok(_) => panic!("overflowing native deadline should fail"),
        Err(error) => error,
    };

    assert!(matches!(error, TimeError::InstantOverflow));
}

#[test]
fn test_std_timer_retains_clock_domain_after_source_is_dropped() {
    let (timer, domain) = {
        let clock = StdMonotonicClock::new();
        let domain = clock.now().domain();
        (StdTimer::from_clock(&clock), domain)
    };

    assert_eq!(domain, timer.clock().now().domain());
    assert!(format!("{timer:?}").starts_with("StdTimer"));
}
