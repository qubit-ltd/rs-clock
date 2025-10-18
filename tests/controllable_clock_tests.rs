/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for the ControllableClock trait.

use chrono::{DateTime, Duration, Utc};
use prism3_clock::{Clock, ControllableClock, MockClock};

#[test]
fn test_controllable_clock_set_time() {
    let clock = MockClock::new();

    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    clock.set_time(fixed_time);

    let current = clock.time();
    assert_eq!(current, fixed_time);
}

#[test]
fn test_controllable_clock_set_time_multiple_times() {
    let clock = MockClock::new();

    let time1 = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(time1);
    assert_eq!(clock.time(), time1);

    let time2 = DateTime::parse_from_rfc3339("2024-06-15T12:30:45Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(time2);
    assert_eq!(clock.time(), time2);

    let time3 = DateTime::parse_from_rfc3339("2023-12-31T23:59:59Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(time3);
    assert_eq!(clock.time(), time3);
}

#[test]
fn test_controllable_clock_add_duration_positive() {
    let clock = MockClock::new();

    let initial = clock.time();
    clock.add_duration(Duration::hours(1));

    let after = clock.time();
    assert_eq!(after - initial, Duration::hours(1));
}

#[test]
fn test_controllable_clock_add_duration_negative() {
    let clock = MockClock::new();

    let initial = clock.time();
    clock.add_duration(Duration::hours(-1));

    let after = clock.time();
    assert_eq!(after - initial, Duration::hours(-1));
}

#[test]
fn test_controllable_clock_add_duration_multiple_times() {
    let clock = MockClock::new();

    let initial = clock.time();

    clock.add_duration(Duration::hours(1));
    clock.add_duration(Duration::minutes(30));
    clock.add_duration(Duration::seconds(45));

    let after = clock.time();
    let expected_duration = Duration::hours(1) + Duration::minutes(30) + Duration::seconds(45);

    let diff = (after - initial - expected_duration)
        .num_milliseconds()
        .abs();
    assert!(diff < 10, "Duration should accumulate correctly");
}

#[test]
fn test_controllable_clock_reset() {
    let clock = MockClock::new();
    let initial = clock.time();

    // Modify the time
    clock.add_duration(Duration::hours(5));
    assert_ne!(clock.time(), initial);

    // Reset
    clock.reset();

    // Should be close to initial time
    let after_reset = clock.time();
    let diff = (after_reset - initial).num_milliseconds().abs();
    assert!(
        diff < 100,
        "After reset, time should be close to initial time"
    );
}

#[test]
fn test_controllable_clock_reset_after_set_time() {
    let clock = MockClock::new();
    let initial = clock.time();

    // Set to a specific time
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);

    // Reset should go back to initial time, not the set time
    clock.reset();

    let after_reset = clock.time();
    let diff = (after_reset - initial).num_milliseconds().abs();
    assert!(
        diff < 100,
        "After reset, time should be close to initial time"
    );
}

#[test]
fn test_controllable_clock_complex_scenario() {
    let clock = MockClock::new();

    // Set to a known time
    let start_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(start_time);

    // Advance by 1 day
    clock.add_duration(Duration::days(1));
    let after_day = clock.time();
    assert_eq!(after_day, start_time + Duration::days(1));

    // Advance by 12 hours
    clock.add_duration(Duration::hours(12));
    let after_hours = clock.time();
    assert_eq!(
        after_hours,
        start_time + Duration::days(1) + Duration::hours(12)
    );

    // Go back 6 hours
    clock.add_duration(Duration::hours(-6));
    let after_back = clock.time();
    assert_eq!(
        after_back,
        start_time + Duration::days(1) + Duration::hours(6)
    );
}

#[test]
fn test_controllable_clock_trait_object() {
    fn control_time(clock: &dyn ControllableClock) {
        let initial = clock.time();
        clock.add_duration(Duration::hours(1));
        let after = clock.time();
        assert_eq!(after - initial, Duration::hours(1));
    }

    let clock = MockClock::new();
    control_time(&clock);
}
