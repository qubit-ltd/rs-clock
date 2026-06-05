// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for timeline-backed mock clocks.

use std::time::Duration as StdDuration;

use chrono::{
    DateTime,
    Utc,
};
use qubit_clock::{
    Clock,
    MockClock,
    MockTimeline,
};

/// Verifies the default clock constructor is available.
///
/// # Errors
/// The test fails if the default clock is not usable as a clock.
#[test]
fn test_default_creates_current_time_clock() {
    let clock = MockClock::default();

    assert!(clock.millis() > 0);
}

/// Verifies positive nanosecond overflow is clamped to chrono and millis
/// bounds.
///
/// # Errors
/// The test fails if mock clock reads overflow instead of saturating.
#[test]
fn test_time_and_millis_clamp_positive_overflow() {
    let timeline = MockTimeline::new();
    let clock =
        MockClock::with_timeline(DateTime::<Utc>::MAX_UTC, timeline.clone());

    timeline.advance(StdDuration::MAX);

    assert_eq!(DateTime::<Utc>::MAX_UTC, clock.time());
    assert_eq!(i64::MAX, clock.millis());
}

/// Verifies negative nanosecond overflow is clamped to chrono and millis
/// bounds.
///
/// # Errors
/// The test fails if negative mock clock reads overflow instead of saturating.
#[test]
fn test_time_and_millis_clamp_negative_overflow() {
    let timeline = MockTimeline::new();
    timeline.advance(StdDuration::MAX);
    let clock =
        MockClock::with_timeline(DateTime::<Utc>::MIN_UTC, timeline.clone());

    timeline
        .reset()
        .expect("timeline without waiters should reset");

    assert_eq!(DateTime::<Utc>::MIN_UTC, clock.time());
    assert_eq!(i64::MIN, clock.millis());
}

/// Verifies out-of-range negative chrono values clamp through the fallback
/// path.
///
/// # Errors
/// The test fails if values below chrono's minimum no longer clamp to
/// `MIN_UTC`.
#[test]
fn test_time_clamps_below_chrono_minimum() {
    let timeline = MockTimeline::new();
    timeline.advance(StdDuration::from_secs(86_400));
    let clock =
        MockClock::with_timeline(DateTime::<Utc>::MIN_UTC, timeline.clone());

    timeline
        .reset()
        .expect("timeline without waiters should reset");

    assert_eq!(DateTime::<Utc>::MIN_UTC, clock.time());
}
