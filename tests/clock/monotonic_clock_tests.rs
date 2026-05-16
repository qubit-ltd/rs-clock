/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Behavior-focused tests for the `MonotonicClock` type.

use chrono::Utc;
use qubit_clock::{
    Clock,
    MonotonicClock,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_monotonic_clock_starts_near_system_time() {
    let before = Utc::now().timestamp_millis();
    let clock = MonotonicClock::new();
    let observed = clock.millis();
    let after = Utc::now().timestamp_millis();

    assert!(
        (before..=after.saturating_add(1)).contains(&observed),
        "monotonic clock should anchor to creation wall time: before={before}, observed={observed}, after={after}",
    );
}

#[test]
fn test_monotonic_clock_elapsed_matches_monotonic_millis() {
    let clock = MonotonicClock::new();

    thread::sleep(Duration::from_millis(25));

    let elapsed_millis = i64::try_from(clock.elapsed().as_millis())
        .expect("short test elapsed duration should fit in i64");
    let monotonic_millis = clock.monotonic_millis();

    assert!(
        (elapsed_millis..=elapsed_millis.saturating_add(1)).contains(&monotonic_millis),
        "monotonic_millis should come from elapsed(): elapsed={elapsed_millis}, monotonic={monotonic_millis}",
    );
}

#[test]
fn test_monotonic_clock_clone_preserves_elapsed_base() {
    let clock = MonotonicClock::new();
    thread::sleep(Duration::from_millis(10));
    let cloned = clock.clone();
    thread::sleep(Duration::from_millis(20));

    let original_elapsed = clock.monotonic_millis();
    let cloned_elapsed = cloned.monotonic_millis();

    assert!(
        (original_elapsed - cloned_elapsed).abs() <= 1,
        "cloned clock should preserve the same monotonic base: original={original_elapsed}, cloned={cloned_elapsed}",
    );
}
