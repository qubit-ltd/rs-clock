/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for MockNanoClock.

use chrono::{
    DateTime,
    Duration,
    Utc,
};
use qubit_clock::meter::NanoTimeMeter;
use qubit_clock::{
    Clock,
    ControllableClock,
    MockClockProgression,
    MockNanoClock,
    NanoClock,
};
use std::thread;

const NANOS_PER_SECOND: i128 = 1_000_000_000;

fn fixed_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("fixed test timestamp should parse")
        .with_timezone(&Utc)
}

fn nanos_of(instant: DateTime<Utc>) -> i128 {
    (instant.timestamp() as i128) * NANOS_PER_SECOND + instant.timestamp_subsec_nanos() as i128
}

#[test]
fn test_mock_nano_clock_new_freezes_by_default() {
    let clock = MockNanoClock::new();
    let start = clock.nanos();

    assert!(
        start > 0,
        "MockNanoClock should return positive nanoseconds"
    );

    thread::sleep(std::time::Duration::from_millis(20));

    assert_eq!(clock.nanos(), start);
}

#[test]
fn test_mock_nano_clock_default() {
    let clock = MockNanoClock::default();
    let nanos = clock.nanos();

    assert!(nanos > 0, "Default MockNanoClock should work");
}

#[test]
fn test_mock_nano_clock_set_time_preserves_nanoseconds_and_freezes() {
    let clock = MockNanoClock::new();
    let fixed = fixed_time("2024-01-01T00:00:00.123456789Z");

    clock.set_time(fixed);

    assert_eq!(clock.time_precise(), fixed);
    assert_eq!(clock.nanos(), nanos_of(fixed));

    thread::sleep(std::time::Duration::from_millis(20));

    assert_eq!(clock.time_precise(), fixed);
}

#[test]
fn test_mock_nano_clock_millis_uses_floor_timestamp_millis() {
    let clock = MockNanoClock::new();
    let fixed = fixed_time("1969-12-31T23:59:59.999999999Z");

    clock.set_time(fixed);

    assert_eq!(clock.millis(), fixed.timestamp_millis());
}

#[test]
fn test_mock_nano_clock_add_duration() {
    let clock = MockNanoClock::new();
    let fixed = fixed_time("2024-01-01T00:00:00.000000001Z");
    clock.set_time(fixed);

    clock.add_duration(Duration::nanoseconds(999));

    assert_eq!(clock.nanos(), nanos_of(fixed) + 999);
    assert_eq!(clock.time_precise(), fixed + Duration::nanoseconds(999));
}

#[test]
fn test_mock_nano_clock_advance_nanos() {
    let clock = MockNanoClock::new();
    let before = clock.nanos();

    clock.advance_nanos(1_500);

    assert_eq!(clock.nanos(), before + 1_500);
}

#[test]
fn test_mock_nano_clock_add_nanos_once() {
    let clock = MockNanoClock::new();
    let before = clock.nanos();

    clock.add_nanos(1_000, false);

    let after = clock.nanos();
    assert_eq!(after - before, 1_000);
    assert_eq!(clock.nanos(), after);
}

#[test]
fn test_mock_nano_clock_auto_advance() {
    let clock = MockNanoClock::new();
    let fixed = fixed_time("2024-01-01T00:00:00Z");
    clock.set_time(fixed);
    clock.set_auto_advance_nanos(100);

    let t1 = clock.nanos();
    let t2 = clock.nanos();
    let t3 = clock.nanos();

    assert_eq!(t1, nanos_of(fixed));
    assert_eq!(t2 - t1, 100);
    assert_eq!(t3 - t2, 100);
}

#[test]
fn test_mock_nano_clock_negative_auto_advance() {
    let clock = MockNanoClock::new();
    let fixed = fixed_time("2024-01-01T00:00:00Z");
    clock.set_time(fixed);
    clock.set_auto_advance_nanos(-100);

    let t1 = clock.nanos();
    let t2 = clock.nanos();
    let t3 = clock.nanos();

    assert_eq!(t1, nanos_of(fixed));
    assert_eq!(t2 - t1, -100);
    assert_eq!(t3 - t2, -100);
}

#[test]
fn test_mock_nano_clock_set_time_preserves_auto_advance() {
    let clock = MockNanoClock::new();
    let first = fixed_time("2024-01-01T00:00:00Z");
    let second = fixed_time("2024-02-01T00:00:00.000000042Z");

    clock.set_time(first);
    clock.set_auto_advance_nanos(250);
    assert_eq!(clock.nanos(), nanos_of(first));
    assert_eq!(clock.nanos(), nanos_of(first) + 250);

    clock.set_time(second);

    let read1 = clock.nanos();
    let read2 = clock.nanos();
    assert_eq!(read1, nanos_of(second));
    assert_eq!(read2, read1 + 250);
}

#[test]
fn test_mock_nano_clock_clear_auto_advance() {
    let clock = MockNanoClock::new();

    clock.set_auto_advance_nanos(100);
    let t1 = clock.nanos();
    let t2 = clock.nanos();
    assert_eq!(t2 - t1, 100);

    clock.clear_auto_advance();
    let t3 = clock.nanos();
    let t4 = clock.nanos();
    assert_eq!(t4, t3);
}

#[test]
fn test_mock_nano_clock_reset() {
    let clock = MockNanoClock::new();
    let initial = clock.nanos();
    let fixed = fixed_time("2024-01-01T00:00:00Z");

    clock.set_time(fixed);
    clock.advance_nanos(1_000);
    clock.reset();

    assert_eq!(clock.nanos(), initial);
}

#[test]
fn test_mock_nano_clock_clone_shares_state() {
    let clock = MockNanoClock::new();
    let cloned = clock.clone();
    let fixed = fixed_time("2024-01-01T00:00:00Z");

    clock.set_time(fixed);
    cloned.advance_nanos(1_500);

    assert_eq!(clock.nanos(), nanos_of(fixed) + 1_500);
    assert_eq!(cloned.nanos(), clock.nanos());
}

#[test]
fn test_mock_nano_clock_saturating_advance() {
    let clock = MockNanoClock::new();

    clock.set_time(DateTime::<Utc>::UNIX_EPOCH);
    clock.advance_nanos(i128::MAX);
    clock.advance_nanos(1);
    assert_eq!(clock.nanos(), i128::MAX);

    clock.set_time(DateTime::<Utc>::UNIX_EPOCH);
    clock.advance_nanos(i128::MIN);
    clock.advance_nanos(-1);
    assert_eq!(clock.nanos(), i128::MIN);
}

#[test]
fn test_mock_nano_clock_millis_clamps_out_of_i64_range() {
    let clock = MockNanoClock::new();

    clock.set_time(DateTime::<Utc>::UNIX_EPOCH);
    clock.advance_nanos(i128::MAX);
    assert_eq!(clock.millis(), i64::MAX);

    clock.set_time(DateTime::<Utc>::UNIX_EPOCH);
    clock.advance_nanos(i128::MIN);
    assert_eq!(clock.millis(), i64::MIN);
}

#[test]
fn test_mock_nano_clock_trait_objects() {
    let clock = MockNanoClock::new();
    let fixed = fixed_time("2024-01-01T00:00:00.000000123Z");

    {
        let controllable: &dyn ControllableClock = &clock;
        controllable.set_time(fixed);
    }

    let clock_trait: &dyn Clock = &clock;
    assert_eq!(clock_trait.millis(), fixed.timestamp_millis());

    let nano_trait: &dyn NanoClock = &clock;
    assert_eq!(nano_trait.nanos(), nanos_of(fixed));
}

#[test]
fn test_mock_nano_clock_with_nano_time_meter() {
    let clock = MockNanoClock::new();
    let mut meter = NanoTimeMeter::with_clock(clock.clone());

    meter.start();
    clock.advance_nanos(42_000);
    meter.stop();

    assert_eq!(meter.nanos(), 42_000);
}

#[test]
fn test_mock_nano_clock_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<MockNanoClock>();
    assert_sync::<MockNanoClock>();
}

#[test]
fn test_mock_nano_clock_monotonic_progression_can_be_enabled() {
    let clock = MockNanoClock::with_progression(MockClockProgression::Monotonic);
    assert_eq!(clock.progression(), MockClockProgression::Monotonic);
    assert!(clock.monotonic_progression_enabled());

    let start = clock.nanos();
    thread::sleep(std::time::Duration::from_millis(20));
    let elapsed = clock.nanos() - start;

    assert!(
        elapsed >= 20_000_000,
        "Monotonic progression should advance with elapsed time, got: {}",
        elapsed
    );
}

#[test]
fn test_mock_nano_clock_set_time_uses_current_progression_mode() {
    let clock = MockNanoClock::new();
    let fixed = fixed_time("2024-01-01T00:00:00.000000123Z");

    clock.set_time(fixed);
    thread::sleep(std::time::Duration::from_millis(10));
    assert_eq!(clock.time_precise(), fixed);

    clock.set_monotonic_progression_enabled(true);
    assert_eq!(clock.progression(), MockClockProgression::Monotonic);
    clock.set_time(fixed);
    thread::sleep(std::time::Duration::from_millis(20));

    assert_eq!(clock.progression(), MockClockProgression::Monotonic);
    assert!(clock.monotonic_progression_enabled());
    let diff = clock.nanos() - nanos_of(fixed);
    assert!(
        diff >= 20_000_000,
        "set_time should progress when monotonic mode is enabled, diff: {}",
        diff
    );
}

#[test]
fn test_mock_nano_clock_disabling_monotonic_progression_freezes_current_reading() {
    let clock = MockNanoClock::with_progression(MockClockProgression::Monotonic);

    thread::sleep(std::time::Duration::from_millis(10));
    clock.set_progression(MockClockProgression::Frozen);
    assert_eq!(clock.progression(), MockClockProgression::Frozen);

    let frozen = clock.nanos();
    thread::sleep(std::time::Duration::from_millis(10));

    assert_eq!(clock.nanos(), frozen);
}

#[test]
fn test_mock_nano_clock_reset_restores_initial_progression() {
    let clock = MockNanoClock::with_progression(MockClockProgression::Monotonic);

    clock.set_progression(MockClockProgression::Frozen);
    clock.add_duration(Duration::seconds(1));
    clock.reset();

    assert_eq!(clock.progression(), MockClockProgression::Monotonic);

    let start = clock.nanos();
    thread::sleep(std::time::Duration::from_millis(10));
    assert!(clock.nanos() - start >= 10_000_000);
}
