/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for SystemClock.

use chrono::Datelike;
use prism3_clock::{Clock, SystemClock};
use std::thread;
use std::time::Duration;

#[test]
fn test_system_clock_new() {
    let clock = SystemClock::new();
    let millis = clock.millis();
    assert!(
        millis > 0,
        "SystemClock should return positive milliseconds"
    );
}

#[test]
fn test_system_clock_default() {
    let clock = SystemClock;
    let millis = clock.millis();
    assert!(millis > 0, "Default SystemClock should work");
}

#[test]
fn test_system_clock_millis() {
    let clock = SystemClock::new();
    let millis = clock.millis();

    // Should be a reasonable timestamp (after 2020-01-01)
    let min_timestamp = 1577836800000i64; // 2020-01-01 in milliseconds
    assert!(
        millis > min_timestamp,
        "SystemClock should return a reasonable timestamp"
    );
}

#[test]
fn test_system_clock_time() {
    let clock = SystemClock::new();
    let time = clock.time();

    // Should be a reasonable year
    assert!(
        time.year() >= 2020,
        "SystemClock should return a reasonable year"
    );
}

#[test]
fn test_system_clock_consistency() {
    let clock = SystemClock::new();
    let millis = clock.millis();
    let time = clock.time();

    // They should be very close (within a few milliseconds)
    let diff = (millis - time.timestamp_millis()).abs();
    assert!(
        diff < 10,
        "millis() and time() should be consistent, diff: {}",
        diff
    );
}

#[test]
fn test_system_clock_progresses() {
    let clock = SystemClock::new();
    let time1 = clock.millis();

    thread::sleep(Duration::from_millis(50));

    let time2 = clock.millis();

    assert!(time2 >= time1, "SystemClock time should progress");

    let elapsed = time2 - time1;
    assert!(
        elapsed >= 50,
        "At least 50ms should have elapsed, got: {}",
        elapsed
    );
}

#[test]
fn test_system_clock_multiple_instances() {
    let clock1 = SystemClock::new();
    let clock2 = SystemClock::new();

    let time1 = clock1.millis();
    let time2 = clock2.millis();

    // Should be very close
    let diff = (time1 - time2).abs();
    assert!(
        diff < 10,
        "Multiple SystemClock instances should return similar times"
    );
}

#[test]
fn test_system_clock_clone() {
    let clock1 = SystemClock::new();
    let clock2 = clock1;

    let time1 = clock1.millis();
    let time2 = clock2.millis();

    // Should be very close
    let diff = (time1 - time2).abs();
    assert!(diff < 10, "Cloned clocks should return similar times");
}

#[test]
fn test_system_clock_copy() {
    let clock1 = SystemClock::new();
    let clock2 = clock1; // Copy, not move

    let time1 = clock1.millis();
    let time2 = clock2.millis();

    // Should be very close
    let diff = (time1 - time2).abs();
    assert!(diff < 10, "Copied clocks should return similar times");
}

#[test]
fn test_system_clock_debug() {
    let clock = SystemClock::new();
    let debug_str = format!("{:?}", clock);
    assert!(
        debug_str.contains("SystemClock"),
        "Debug output should contain 'SystemClock'"
    );
}

#[test]
fn test_system_clock_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<SystemClock>();
    assert_sync::<SystemClock>();
}

#[test]
fn test_system_clock_in_thread() {
    use std::sync::Arc;

    let clock = Arc::new(SystemClock::new());
    let clock_clone = clock.clone();

    let handle = thread::spawn(move || {
        let millis = clock_clone.millis();
        assert!(millis > 0);
        millis
    });

    let result = handle.join().unwrap();
    assert!(result > 0);
}

#[test]
fn test_system_clock_multiple_threads() {
    use std::sync::Arc;

    let clock = Arc::new(SystemClock::new());
    let mut handles = vec![];

    for _ in 0..10 {
        let clock_clone = clock.clone();
        let handle = thread::spawn(move || clock_clone.millis());
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result > 0);
    }
}
