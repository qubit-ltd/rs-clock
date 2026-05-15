/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Behavior-focused tests for the `MockClock` type.

use chrono::{
    DateTime,
    Utc,
};
use qubit_clock::{
    Clock,
    ControllableClock,
    MockClock,
};

/// Parses a fixed UTC timestamp used by mock-clock behavior tests.
fn fixed_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("fixed test timestamp should parse")
        .with_timezone(&Utc)
}

#[test]
fn test_mock_clock_set_time_clears_auto_advance() {
    let clock = MockClock::new();
    let initial_time = fixed_time("2024-01-01T00:00:00Z");
    let replacement_time = fixed_time("2024-02-01T00:00:00Z");

    clock.set_time(initial_time);
    clock.set_auto_advance_millis(250);
    assert_eq!(clock.millis(), initial_time.timestamp_millis());
    assert_eq!(clock.millis(), initial_time.timestamp_millis() + 250);

    clock.set_time(replacement_time);

    let first_read = clock.millis();
    let second_read = clock.millis();
    assert_eq!(first_read, replacement_time.timestamp_millis());
    assert_eq!(second_read, first_read);
}

#[test]
fn test_mock_clock_clone_shares_manual_adjustments() {
    let clock = MockClock::new();
    let cloned = clock.clone();
    let initial_time = fixed_time("2024-03-01T00:00:00Z");

    clock.set_time(initial_time);
    cloned.advance_millis(1_500);

    assert_eq!(clock.millis(), initial_time.timestamp_millis() + 1_500);
    assert_eq!(cloned.millis(), clock.millis());
}

#[test]
fn test_mock_clock_negative_auto_advance_moves_backward_per_read() {
    let clock = MockClock::new();
    let initial_time = fixed_time("2024-04-01T00:00:00Z");

    clock.set_time(initial_time);
    clock.set_auto_advance_millis(-100);

    let first_read = clock.millis();
    let second_read = clock.millis();
    let third_read = clock.millis();

    assert_eq!(first_read, initial_time.timestamp_millis());
    assert_eq!(second_read, initial_time.timestamp_millis() - 100);
    assert_eq!(third_read, initial_time.timestamp_millis() - 200);
}
