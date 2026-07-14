// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    TimeError,
};
use std::time::Duration;

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

    assert_eq!(
        Err(TimeError::InstantOverflow),
        maximum.checked_add(Duration::from_nanos(1)),
    );
}

#[test]
fn test_monotonic_instant_rejects_foreign_domain() {
    let first = ManualMonotonicClock::new().now();
    let second = ManualMonotonicClock::new().now();

    assert_eq!(
        Err(TimeError::ClockDomainMismatch {
            expected: first.domain(),
            actual: second.domain(),
        }),
        first.duration_since(second),
    );
    assert_eq!(None, first.partial_cmp(&second));
}

#[test]
fn test_monotonic_instant_reports_backward_duration() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();
    let end = start
        .checked_add(Duration::from_secs(1))
        .expect("short duration should be representable");

    assert_eq!(
        Err(TimeError::InvalidInstantOrder),
        start.duration_since(end),
    );
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
