/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for Zoned wrapper.

use chrono::{
    DateTime,
    Datelike,
    Duration,
    Timelike,
    Utc,
};
use chrono_tz::America::New_York;
use chrono_tz::Asia::Shanghai;
use chrono_tz::Europe::London;
use chrono_tz::Tz;
use qubit_clock::{
    Clock,
    ControllableClock,
    MockClock,
    MonotonicClock,
    NanoClock,
    NanoMonotonicClock,
    SystemClock,
    Zoned,
    ZonedClock,
};
use std::time::Duration as StdDuration;

const CLOCK_DRIFT_TOLERANCE_MS: i64 = 1_000;

/// Asserts that a clock reading is close to the expected logical time.
fn assert_close_to_expected_time(actual: DateTime<Utc>, expected: DateTime<Utc>) {
    let diff_ms = actual
        .timestamp_millis()
        .saturating_sub(expected.timestamp_millis());
    assert!(
        (0..CLOCK_DRIFT_TOLERANCE_MS).contains(&diff_ms),
        "time should be within {}ms after expected time, diff_ms: {}",
        CLOCK_DRIFT_TOLERANCE_MS,
        diff_ms
    );
}

#[test]
fn test_zoned_new() {
    let clock = Zoned::new(SystemClock::new(), Shanghai);
    assert_eq!(clock.timezone(), Shanghai);
}

#[test]
fn test_zoned_with_system_clock() {
    let clock = Zoned::new(SystemClock::new(), Shanghai);
    let utc = clock.time();
    let local = clock.local_time();

    // Both reads should represent nearly the same instant.
    let diff = (utc.timestamp_millis() - local.timestamp_millis()).abs();
    assert!(
        diff <= 10,
        "timestamp drift too large between UTC and local reads: diff={diff}ms"
    );
}

#[test]
fn test_zoned_with_monotonic_clock() {
    let clock = Zoned::new(MonotonicClock::new(), Shanghai);
    let utc = clock.time();
    let local = clock.local_time();

    // Both reads should represent nearly the same instant.
    let diff = (utc.timestamp_millis() - local.timestamp_millis()).abs();
    assert!(
        diff <= 10,
        "timestamp drift too large between UTC and local reads: diff={diff}ms"
    );
}

#[test]
fn test_zoned_with_mock_clock() {
    let mock = MockClock::new();
    let clock = Zoned::new(mock, Shanghai);

    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);

    let local = clock.local_time();
    assert_eq!(local.hour(), 8); // UTC+8
    assert_eq!(local.day(), 1);
}

#[test]
fn test_zoned_timezone_conversion() {
    let mock = MockClock::new();
    let utc_time = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(utc_time);

    // Test Shanghai (UTC+8)
    let shanghai = Zoned::new(mock.clone(), Shanghai);
    assert_eq!(shanghai.local_time().hour(), 20); // 12 + 8

    // Test New York (UTC-5 in winter)
    let ny = Zoned::new(mock.clone(), New_York);
    assert_eq!(ny.local_time().hour(), 7); // 12 - 5

    // Test London (UTC+0 in winter)
    let london = Zoned::new(mock.clone(), London);
    assert_eq!(london.local_time().hour(), 12); // Same as UTC
}

#[test]
fn test_zoned_deref_to_inner_clock() {
    let mock = MockClock::new();
    let clock = Zoned::new(mock, Shanghai);

    // Can call MockClock-specific methods directly via Deref.
    assert_eq!(clock.timeline().elapsed(), StdDuration::ZERO);
    clock.advance(StdDuration::from_millis(10));
    assert_eq!(clock.timeline().elapsed(), StdDuration::from_millis(10));

    // Can call controllable methods directly as well.
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);
    assert_eq!(clock.time(), fixed_time);

    clock.add_duration(Duration::hours(1));
    assert_eq!(clock.time(), fixed_time + Duration::hours(1));

    clock.reset();
}

#[test]
fn test_zoned_inner() {
    let mock = MockClock::new();
    let clock = Zoned::new(mock, Shanghai);

    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    // Access inner clock explicitly
    clock.inner().set_time(fixed_time);
    assert_eq!(clock.time(), fixed_time);
}

#[test]
fn test_zoned_into_inner() {
    let mock = MockClock::new();
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(fixed_time);

    let clock = Zoned::new(mock, Shanghai);

    // Consume the wrapper and get the inner clock
    let inner = clock.into_inner();
    assert_eq!(inner.time(), fixed_time);
}

#[test]
fn test_zoned_implements_clock() {
    let clock = Zoned::new(SystemClock::new(), Shanghai);

    // Should implement Clock trait
    let millis = clock.millis();
    let time = clock.time();

    assert!(millis > 0);
    assert!(time.timestamp_millis() > 0);
}

#[test]
fn test_zoned_implements_zoned_clock() {
    let clock = Zoned::new(SystemClock::new(), Shanghai);

    // Should implement ZonedClock trait
    let tz = clock.timezone();
    let local = clock.local_time();

    assert_eq!(tz, Shanghai);
    assert!(local.timestamp_millis() > 0);
}

#[test]
fn test_zoned_clone() {
    let mock = MockClock::new();
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(fixed_time);

    let clock1 = Zoned::new(mock, Shanghai);
    let clock2 = clock1.clone();

    // Both should expose the same timezone and logical time.
    assert_eq!(clock1.timezone(), clock2.timezone());
    assert_close_to_expected_time(clock1.time(), fixed_time);
    assert_close_to_expected_time(clock2.time(), fixed_time);
    assert_close_to_expected_time(clock1.local_time().with_timezone(&Utc), fixed_time);
    assert_close_to_expected_time(clock2.local_time().with_timezone(&Utc), fixed_time);

    // Modifying one should affect the other (shared MockClock)
    clock1.add_duration(Duration::hours(1));
    let expected_time = fixed_time + Duration::hours(1);
    assert_close_to_expected_time(clock2.time(), expected_time);
}

#[test]
fn test_zoned_debug() {
    let clock = Zoned::new(SystemClock::new(), Shanghai);
    let debug_str = format!("{:?}", clock);
    assert!(
        debug_str.contains("Zoned"),
        "Debug output should contain 'Zoned'"
    );
}

#[test]
fn test_zoned_with_different_timezones() {
    let timezones: Vec<Tz> = vec![
        Shanghai,
        New_York,
        London,
        chrono_tz::Asia::Tokyo,
        chrono_tz::Europe::Paris,
    ];

    let mock = MockClock::new();
    let utc_time = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(utc_time);

    for tz in timezones {
        let clock = Zoned::new(mock.clone(), tz);
        let local = clock.local_time();

        // All should represent the same instant, allowing tiny scheduling jitter.
        let diff = (local.timestamp_millis() - utc_time.timestamp_millis()).abs();
        assert!(
            diff <= 10,
            "timestamp drift too large for timezone {tz}: diff={diff}ms"
        );

        // But hours might be different
        println!("{}: {} (UTC: {})", tz, local.hour(), utc_time.hour());
    }
}

#[test]
fn test_zoned_trait_object() {
    fn use_zoned_clock(clock: &dyn ZonedClock) -> i64 {
        clock.local_time().timestamp_millis()
    }

    let clock = Zoned::new(SystemClock::new(), Shanghai);
    assert!(use_zoned_clock(&clock) > 0);
}

#[test]
fn test_zoned_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<Zoned<SystemClock>>();
    assert_sync::<Zoned<SystemClock>>();
    assert_send::<Zoned<MockClock>>();
    assert_sync::<Zoned<MockClock>>();
}

#[test]
fn test_zoned_in_thread() {
    use std::sync::Arc;

    let mock = MockClock::new();
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(fixed_time);

    let clock = Arc::new(Zoned::new(mock, Shanghai));
    let clock_clone = clock.clone();

    let handle = std::thread::spawn(move || clock_clone.local_time());

    let result = handle.join().unwrap();
    assert_eq!(result.hour(), 8); // UTC+8
}

#[test]
fn test_zoned_daylight_saving_time() {
    let mock = MockClock::new();

    // Winter time in New York (EST, UTC-5)
    let winter = DateTime::parse_from_rfc3339("2024-01-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(winter);

    let ny_clock = Zoned::new(mock.clone(), New_York);
    let winter_local = ny_clock.local_time();
    assert_eq!(winter_local.hour(), 7); // 12 - 5

    // Summer time in New York (EDT, UTC-4)
    let summer = DateTime::parse_from_rfc3339("2024-07-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(summer);

    let summer_local = ny_clock.local_time();
    assert_eq!(summer_local.hour(), 8); // 12 - 4
}

#[test]
fn test_zoned_preserves_controllable_clock_interface() {
    let mock = MockClock::new();
    let clock = Zoned::new(mock, Shanghai);

    // Can use ControllableClock methods
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);
    clock.add_duration(Duration::hours(1));
    clock.reset();

    // All should work without error
}

#[test]
fn test_zoned_preserves_controllable_clock_trait_object() {
    fn use_controllable_clock(clock: &dyn ControllableClock, fixed_time: DateTime<Utc>) {
        clock.set_time(fixed_time);
        clock.add_duration(Duration::hours(1));
        assert_close_to_expected_time(clock.time(), fixed_time + Duration::hours(1));
    }

    let clock = Zoned::new(MockClock::new(), Shanghai);
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    use_controllable_clock(&clock, fixed_time);
}

#[test]
fn test_zoned_preserves_nano_clock_interface() {
    fn use_nano_clock(clock: &dyn NanoClock) -> i128 {
        clock.nanos()
    }

    let clock = Zoned::new(NanoMonotonicClock::new(), Shanghai);
    let nanos = use_nano_clock(&clock);

    assert!(nanos > 0);
    assert!(clock.time_precise().timestamp_millis() > 0);
}
