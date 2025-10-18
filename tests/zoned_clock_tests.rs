/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for the ZonedClock trait.

use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::America::New_York;
use chrono_tz::Asia::Shanghai;
use chrono_tz::Europe::London;
use chrono_tz::Tz;
use prism3_clock::{Clock, ControllableClock, MockClock, SystemClock, Zoned, ZonedClock};

// A simple test implementation that uses the default local_time() implementation
#[derive(Debug, Clone)]
struct TestZonedClock {
    clock: MockClock,
    tz: Tz,
}

impl TestZonedClock {
    fn new(tz: Tz) -> Self {
        TestZonedClock {
            clock: MockClock::new(),
            tz,
        }
    }
}

impl Clock for TestZonedClock {
    fn millis(&self) -> i64 {
        self.clock.millis()
    }

    fn time(&self) -> DateTime<Utc> {
        self.clock.time()
    }
}

impl ZonedClock for TestZonedClock {
    fn timezone(&self) -> Tz {
        self.tz
    }
    // Note: We deliberately don't override local_time() to test the default implementation
}

impl ControllableClock for TestZonedClock {
    fn set_time(&self, instant: DateTime<Utc>) {
        self.clock.set_time(instant);
    }

    fn add_duration(&self, duration: chrono::Duration) {
        self.clock.add_duration(duration);
    }

    fn reset(&self) {
        self.clock.reset();
    }
}

#[test]
fn test_zoned_clock_timezone() {
    let clock = Zoned::new(SystemClock::new(), Shanghai);
    assert_eq!(clock.timezone(), Shanghai);

    let clock = Zoned::new(SystemClock::new(), New_York);
    assert_eq!(clock.timezone(), New_York);
}

#[test]
fn test_zoned_clock_local_time() {
    let mock = MockClock::new();
    let clock = Zoned::new(mock, Shanghai);

    // Set to UTC midnight
    let utc_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(utc_time);

    let local = clock.local_time();

    // Shanghai is UTC+8, so local time should be 08:00
    assert_eq!(local.hour(), 8);
    assert_eq!(local.day(), 1);
    assert_eq!(local.month(), 1);
    assert_eq!(local.year(), 2024);
}

#[test]
fn test_zoned_clock_different_timezones() {
    let mock = MockClock::new();
    let utc_time = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(utc_time);

    // Test Shanghai (UTC+8)
    let shanghai_clock = Zoned::new(mock.clone(), Shanghai);
    let shanghai_time = shanghai_clock.local_time();
    assert_eq!(shanghai_time.hour(), 20); // 12 + 8

    // Test New York (UTC-5 in winter)
    let ny_clock = Zoned::new(mock.clone(), New_York);
    let ny_time = ny_clock.local_time();
    assert_eq!(ny_time.hour(), 7); // 12 - 5

    // Test London (UTC+0 in winter)
    let london_clock = Zoned::new(mock.clone(), London);
    let london_time = london_clock.local_time();
    assert_eq!(london_time.hour(), 12); // Same as UTC
}

#[test]
fn test_zoned_clock_utc_time_consistency() {
    let mock = MockClock::new();
    let clock = Zoned::new(mock, Shanghai);

    let utc_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(utc_time);

    // UTC time should remain the same
    assert_eq!(clock.time(), utc_time);

    // Local time should be different
    let local = clock.local_time();
    assert_ne!(local.hour(), utc_time.hour());
}

#[test]
fn test_zoned_clock_trait_object() {
    fn use_zoned_clock(clock: &dyn ZonedClock) -> i64 {
        let local = clock.local_time();
        local.timestamp_millis()
    }

    let clock = Zoned::new(SystemClock::new(), Shanghai);
    assert!(use_zoned_clock(&clock) > 0);
}

#[test]
fn test_zoned_clock_with_system_clock() {
    let clock = Zoned::new(SystemClock::new(), Shanghai);
    let utc = clock.time();
    let local = clock.local_time();

    // Both should represent the same instant in time
    assert_eq!(utc.timestamp_millis(), local.timestamp_millis());

    // But the hour might be different due to timezone
    // (unless it happens to be the same by coincidence)
}

#[test]
fn test_zoned_clock_daylight_saving_time() {
    let mock = MockClock::new();

    // Test New York in winter (EST, UTC-5)
    let winter_time = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(winter_time);

    let ny_clock = Zoned::new(mock.clone(), New_York);
    let winter_local = ny_clock.local_time();
    assert_eq!(winter_local.hour(), 7); // 12 - 5

    // Test New York in summer (EDT, UTC-4)
    let summer_time = DateTime::parse_from_rfc3339("2024-07-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    mock.set_time(summer_time);

    let summer_local = ny_clock.local_time();
    assert_eq!(summer_local.hour(), 8); // 12 - 4
}

#[test]
fn test_zoned_clock_default_local_time_implementation() {
    // This test specifically tests the default implementation of local_time()
    // in the ZonedClock trait
    let clock = TestZonedClock::new(Shanghai);

    // Set to UTC midnight
    let utc_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(utc_time);

    // Test that the default implementation correctly converts UTC to local time
    let local = clock.local_time();

    // Shanghai is UTC+8, so local time should be 08:00
    assert_eq!(local.hour(), 8);
    assert_eq!(local.day(), 1);
    assert_eq!(local.month(), 1);
    assert_eq!(local.year(), 2024);

    // Verify the timestamp is the same
    assert_eq!(local.timestamp_millis(), utc_time.timestamp_millis());
}

#[test]
fn test_zoned_clock_default_local_time_with_different_timezones() {
    // Test the default implementation with multiple timezones
    let utc_time = DateTime::parse_from_rfc3339("2024-06-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    // Test Shanghai (UTC+8)
    let shanghai_clock = TestZonedClock::new(Shanghai);
    shanghai_clock.set_time(utc_time);
    let shanghai_local = shanghai_clock.local_time();
    assert_eq!(shanghai_local.hour(), 20); // 12 + 8
    assert_eq!(shanghai_local.day(), 15);

    // Test New York (UTC-4 in summer)
    let ny_clock = TestZonedClock::new(New_York);
    ny_clock.set_time(utc_time);
    let ny_local = ny_clock.local_time();
    assert_eq!(ny_local.hour(), 8); // 12 - 4
    assert_eq!(ny_local.day(), 15);

    // Test London (UTC+1 in summer)
    let london_clock = TestZonedClock::new(London);
    london_clock.set_time(utc_time);
    let london_local = london_clock.local_time();
    assert_eq!(london_local.hour(), 13); // 12 + 1
    assert_eq!(london_local.day(), 15);
}

#[test]
fn test_zoned_clock_default_local_time_edge_cases() {
    // Test edge cases with the default implementation
    let clock = TestZonedClock::new(Shanghai);

    // Test date boundary crossing
    let utc_time = DateTime::parse_from_rfc3339("2024-01-31T23:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(utc_time);

    let local = clock.local_time();
    // UTC 23:00 on Jan 31 -> Shanghai 07:00 on Feb 1
    assert_eq!(local.hour(), 7);
    assert_eq!(local.day(), 1);
    assert_eq!(local.month(), 2);
    assert_eq!(local.year(), 2024);
}

#[test]
fn test_zoned_clock_default_local_time_preserves_timestamp() {
    // Verify that the default implementation preserves the actual timestamp
    let clock = TestZonedClock::new(Shanghai);

    let utc_time = DateTime::parse_from_rfc3339("2024-03-15T14:30:45Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(utc_time);

    let local = clock.local_time();

    // The timestamp should be exactly the same
    assert_eq!(local.timestamp(), utc_time.timestamp());
    assert_eq!(local.timestamp_millis(), utc_time.timestamp_millis());
    assert_eq!(
        local.timestamp_nanos_opt(),
        utc_time.timestamp_nanos_opt()
    );

    // But the displayed time should be different
    assert_ne!(local.hour(), utc_time.hour());
}
