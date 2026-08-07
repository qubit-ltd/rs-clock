// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::TimeError;

#[test]
fn test_monotonic_instant_checked_add_preserves_domain() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();
    let end = start
        .checked_add(Duration::from_millis(25))
        .expect("short duration should be representable");

    assert_eq!(start.domain(), end.domain());
    assert_eq!(
        Duration::from_millis(25),
        end.duration_since(start)
            .expect("instants should share one domain"),
    );
}

#[test]
fn test_monotonic_instant_checked_add_reports_overflow() {
    let clock = ManualMonotonicClock::new();
    let maximum = clock
        .now()
        .checked_add(Duration::MAX)
        .expect("duration maximum should fit from zero");

    assert!(matches!(
        maximum.checked_add(Duration::from_nanos(1)),
        Err(TimeError::InstantOverflow),
    ));
}

#[test]
fn test_monotonic_instant_validate_domain_accepts_same_domain() {
    let instant = ManualMonotonicClock::new().now();

    instant
        .validate_domain(instant.domain())
        .expect("an instant should validate against its own domain");
}

#[test]
fn test_monotonic_instant_validate_domain_rejects_foreign_domain() {
    let expected = ManualMonotonicClock::new().now();
    let actual = ManualMonotonicClock::new().now();

    let Err(TimeError::ClockDomainMismatch {
        expected: error_expected,
        actual: error_actual,
    }) = actual.validate_domain(expected.domain())
    else {
        panic!("a foreign expected domain should report a mismatch");
    };
    assert_eq!(expected.domain(), error_expected);
    assert_eq!(actual.domain(), error_actual);
}

#[test]
fn test_monotonic_instant_rejects_foreign_domain() {
    let first = ManualMonotonicClock::new().now();
    let second = ManualMonotonicClock::new().now();

    let Err(TimeError::ClockDomainMismatch { expected, actual }) =
        first.duration_since(second)
    else {
        panic!("cross-domain duration should report a domain mismatch");
    };
    assert_eq!(first.domain(), expected);
    assert_eq!(second.domain(), actual);
    assert_eq!(None, first.partial_cmp(&second));
}

#[test]
fn test_monotonic_instant_reports_backward_duration() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();
    let end = start
        .checked_add(Duration::from_secs(1))
        .expect("short duration should be representable");

    let Err(TimeError::InvalidInstantOrder {
        current_elapsed,
        earlier_elapsed,
    }) = start.duration_since(end)
    else {
        panic!("backward duration should report both elapsed values");
    };
    assert_eq!(Duration::ZERO, current_elapsed);
    assert_eq!(Duration::from_secs(1), earlier_elapsed);
}

#[test]
fn test_monotonic_instant_orders_values_in_same_domain() {
    let clock = ManualMonotonicClock::new();
    let first = clock.now();
    let second = first
        .checked_add(Duration::from_nanos(1))
        .expect("one nanosecond should be representable");

    assert_eq!(Some(std::cmp::Ordering::Less), first.partial_cmp(&second));
}
