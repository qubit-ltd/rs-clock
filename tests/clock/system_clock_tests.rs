// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavior-focused tests for the `SystemClock` type.

use chrono::Utc;
use qubit_clock::{
    Clock,
    SystemClock,
};

#[test]
fn test_system_clock_millis_matches_surrounding_utc_now() {
    let clock = SystemClock::new();

    let before = Utc::now().timestamp_millis();
    let observed = clock.millis();
    let after = Utc::now().timestamp_millis();

    assert!(
        (before..=after).contains(&observed),
        "SystemClock::millis should read the current UTC wall clock: before={before}, observed={observed}, after={after}",
    );
}

#[test]
fn test_system_clock_time_matches_surrounding_utc_now() {
    let clock = SystemClock::new();

    let before = Utc::now();
    let observed = clock.time();
    let after = Utc::now();

    assert!(
        (before..=after).contains(&observed),
        "SystemClock::time should read the current UTC wall clock: before={before}, observed={observed}, after={after}",
    );
}

#[test]
fn test_system_clock_is_zero_sized() {
    assert_eq!(std::mem::size_of::<SystemClock>(), 0);
}
