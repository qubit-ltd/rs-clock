/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for MockClock.

use chrono::{DateTime, Duration, Utc};
use qubit_clock::{Clock, ControllableClock, MockClock};
use std::thread;

#[test]
fn test_mock_clock_new() {
    let clock = MockClock::new();
    let millis = clock.millis();
    assert!(millis > 0, "MockClock should return positive milliseconds");
}

#[test]
fn test_mock_clock_default() {
    let clock = MockClock::default();
    let millis = clock.millis();
    assert!(millis > 0, "Default MockClock should work");
}

#[test]
fn test_mock_clock_set_time() {
    let clock = MockClock::new();

    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    clock.set_time(fixed_time);

    let current = clock.time();
    assert_eq!(current, fixed_time);
}

#[test]
fn test_mock_clock_set_time_multiple() {
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
}

#[test]
fn test_mock_clock_add_duration() {
    let clock = MockClock::new();
    let initial = clock.time();

    clock.add_duration(Duration::hours(1));

    let after = clock.time();
    assert_eq!(after - initial, Duration::hours(1));
}

#[test]
fn test_mock_clock_add_millis_once() {
    let clock = MockClock::new();
    let before = clock.millis();

    clock.add_millis(1000, false);

    let after = clock.millis();
    assert_eq!(after - before, 1000);

    // Should not add again
    let after2 = clock.millis();
    let diff = (after2 - after).abs();
    assert!(diff < 10, "Should not add again");
}

#[test]
fn test_mock_clock_advance_millis() {
    let clock = MockClock::new();
    let before = clock.millis();

    clock.advance_millis(1500);

    let after = clock.millis();
    assert_eq!(after - before, 1500);
}

#[test]
fn test_mock_clock_add_millis_every_time() {
    let clock = MockClock::new();

    clock.add_millis(100, true);

    let t1 = clock.millis();
    let t2 = clock.millis();
    let t3 = clock.millis();

    assert_eq!(t2 - t1, 100);
    assert_eq!(t3 - t2, 100);
}

#[test]
fn test_mock_clock_set_and_clear_auto_advance() {
    let clock = MockClock::new();
    clock.set_auto_advance_millis(100);

    let t1 = clock.millis();
    let t2 = clock.millis();
    assert_eq!(t2 - t1, 100);

    clock.clear_auto_advance();
    let t3 = clock.millis();
    let t4 = clock.millis();
    assert!((t4 - t3).abs() < 10);
}

#[test]
fn test_mock_clock_add_millis_negative() {
    let clock = MockClock::new();
    let before = clock.millis();

    clock.add_millis(-1000, false);

    let after = clock.millis();
    assert_eq!(after - before, -1000);
}

#[test]
fn test_mock_clock_advance_millis_saturates_positive_overflow() {
    let clock = MockClock::new();

    clock.advance_millis(i64::MAX);
    clock.advance_millis(1);

    assert_eq!(clock.millis(), i64::MAX);
}

#[test]
fn test_mock_clock_advance_millis_saturates_negative_overflow() {
    let clock = MockClock::new();
    clock.set_time(DateTime::<Utc>::UNIX_EPOCH);

    clock.advance_millis(i64::MIN);
    clock.advance_millis(-1);

    let millis = clock.millis();
    assert!(
        millis <= i64::MIN.saturating_add(1_000),
        "negative overflow should stay near i64::MIN, got: {}",
        millis
    );
}

#[test]
fn test_mock_clock_auto_advance_saturates_positive_overflow() {
    let clock = MockClock::new();
    clock.set_time(DateTime::<Utc>::UNIX_EPOCH);
    clock.advance_millis(i64::MAX);
    clock.set_auto_advance_millis(1);

    assert_eq!(clock.millis(), i64::MAX);
    assert_eq!(clock.millis(), i64::MAX);
}

#[test]
fn test_mock_clock_reset() {
    let clock = MockClock::new();
    let initial = clock.time();

    clock.add_duration(Duration::hours(5));
    assert_ne!(clock.time(), initial);

    clock.reset();

    let after_reset = clock.time();
    let diff = (after_reset - initial).num_milliseconds().abs();
    assert!(
        diff < 100,
        "After reset, time should be close to initial time"
    );
}

#[test]
fn test_mock_clock_reset_after_set_time() {
    let clock = MockClock::new();
    let initial = clock.time();

    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);

    clock.reset();

    let after_reset = clock.time();
    let diff = (after_reset - initial).num_milliseconds().abs();
    assert!(
        diff < 100,
        "After reset, time should be close to initial time"
    );
}

#[test]
fn test_mock_clock_reset_clears_add_every_time() {
    let clock = MockClock::new();

    clock.add_millis(100, true);
    let t1 = clock.millis();
    let t2 = clock.millis();
    assert_eq!(t2 - t1, 100);

    clock.reset();

    let t3 = clock.millis();
    let t4 = clock.millis();
    let diff = (t4 - t3).abs();
    assert!(diff < 10, "After reset, should not add every time");
}

#[test]
fn test_mock_clock_complex_scenario() {
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
fn test_mock_clock_clone() {
    let clock1 = MockClock::new();
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock1.set_time(fixed_time);

    let clock2 = clock1.clone();

    // Both clocks should share the same internal state
    assert_eq!(clock1.time(), clock2.time());

    // Modifying one should affect the other
    clock1.add_duration(Duration::hours(1));
    assert_eq!(clock1.time(), clock2.time());
}

#[test]
fn test_mock_clock_debug() {
    let clock = MockClock::new();
    let debug_str = format!("{:?}", clock);
    assert!(
        debug_str.contains("MockClock"),
        "Debug output should contain 'MockClock'"
    );
}

#[test]
fn test_mock_clock_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<MockClock>();
    assert_sync::<MockClock>();
}

#[test]
fn test_mock_clock_in_thread() {
    use std::sync::Arc;

    let clock = Arc::new(MockClock::new());
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);

    let clock_clone = clock.clone();

    let handle = thread::spawn(move || {
        clock_clone.add_duration(Duration::hours(1));
        clock_clone.time()
    });

    let result = handle.join().unwrap();
    assert_eq!(result, fixed_time + Duration::hours(1));

    // Main thread should see the change
    assert_eq!(clock.time(), fixed_time + Duration::hours(1));
}

#[test]
fn test_mock_clock_multiple_threads() {
    use std::sync::Arc;

    let clock = Arc::new(MockClock::new());
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);

    let mut handles = vec![];

    for i in 0..10 {
        let clock_clone = clock.clone();
        let handle = thread::spawn(move || {
            clock_clone.add_duration(Duration::hours(i));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // All additions should have accumulated
    let final_time = clock.time();
    let expected_hours = (0..10).sum::<i64>();
    let expected_base = fixed_time + Duration::hours(expected_hours);
    let diff_ms = (final_time - expected_base).num_milliseconds();
    assert!(
        (0..1000).contains(&diff_ms),
        "Final time should be within 1s after expected time, diff_ms: {}",
        diff_ms
    );
}

#[test]
fn test_mock_clock_progresses_after_set_time() {
    let clock = MockClock::new();
    let fixed_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    clock.set_time(fixed_time);

    // Sleep and verify time has progressed naturally
    thread::sleep(std::time::Duration::from_millis(50));

    let current = clock.time();
    let diff = (current - fixed_time).num_milliseconds();
    assert!(
        diff >= 50,
        "Time should progress naturally after set_time, diff: {}",
        diff
    );
    assert!(
        diff < 150,
        "Time should not progress too much, diff: {}",
        diff
    );
}

#[test]
fn test_mock_clock_progresses_naturally() {
    let clock = MockClock::new();
    let start = clock.millis();

    // Without any manipulation, time should progress naturally
    // (based on the internal monotonic clock)
    thread::sleep(std::time::Duration::from_millis(50));

    let elapsed = clock.millis() - start;
    assert!(
        elapsed >= 50,
        "Time should progress naturally, elapsed: {}",
        elapsed
    );
}
