// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Behavior-focused tests for the `NanoMonotonicClock` type.

use qubit_clock::{
    Clock,
    NanoClock,
    NanoMonotonicClock,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_nano_monotonic_clock_time_precise_matches_surrounding_nanos() {
    let clock = NanoMonotonicClock::new();

    let before_nanos = clock.nanos();
    let precise_nanos = clock
        .time_precise()
        .timestamp_nanos_opt()
        .expect("current timestamp should fit in chrono nanosecond range");
    let after_nanos = clock.nanos();

    assert!(
        (before_nanos..=after_nanos).contains(&i128::from(precise_nanos)),
        "time_precise should be derived from the clock's nanos(): before={before_nanos}, precise={precise_nanos}, after={after_nanos}",
    );
}

#[test]
fn test_nano_monotonic_clock_millis_matches_surrounding_nanos() {
    let clock = NanoMonotonicClock::new();

    let before_millis = clock.nanos().div_euclid(1_000_000);
    let millis = clock.millis();
    let after_millis = clock.nanos().div_euclid(1_000_000);

    assert!(
        (before_millis..=after_millis).contains(&i128::from(millis)),
        "millis should be derived from nanos(): before={before_millis}, millis={millis}, after={after_millis}",
    );
}

#[test]
fn test_nano_monotonic_clock_monotonic_nanos_tracks_elapsed() {
    let clock = NanoMonotonicClock::new();
    let start = clock.monotonic_nanos();

    thread::sleep(Duration::from_millis(20));

    let elapsed = clock.monotonic_nanos() - start;
    assert!(
        elapsed >= 20_000_000,
        "monotonic_nanos should advance by at least the slept duration, elapsed={elapsed}",
    );
}
