/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for the Clock trait.

use qubit_clock::{Clock, MockClock, MonotonicClock, SystemClock};
use std::thread;
use std::time::Duration;

#[test]
fn test_clock_millis_returns_positive() {
    let clocks: Vec<Box<dyn Clock>> = vec![
        Box::new(SystemClock::new()),
        Box::new(MonotonicClock::new()),
        Box::new(MockClock::new()),
    ];

    for clock in clocks {
        let millis = clock.millis();
        assert!(millis > 0, "Clock should return positive milliseconds");
    }
}

#[test]
fn test_clock_time_returns_valid_datetime() {
    let clocks: Vec<Box<dyn Clock>> = vec![
        Box::new(SystemClock::new()),
        Box::new(MonotonicClock::new()),
        Box::new(MockClock::new()),
    ];

    for clock in clocks {
        let time = clock.time();
        assert!(
            time.timestamp_millis() > 0,
            "Clock should return valid DateTime"
        );
    }
}

#[test]
fn test_clock_millis_and_time_consistency() {
    let clocks: Vec<Box<dyn Clock>> = vec![
        Box::new(SystemClock::new()),
        Box::new(MonotonicClock::new()),
        Box::new(MockClock::new()),
    ];

    for clock in clocks {
        let millis = clock.millis();
        let time = clock.time();
        let time_millis = time.timestamp_millis();

        // Allow small difference due to time passing between calls
        let diff = (millis - time_millis).abs();
        assert!(
            diff < 10,
            "millis() and time() should be consistent, diff: {}",
            diff
        );
    }
}

#[test]
fn test_clock_time_progresses() {
    let clocks: Vec<Box<dyn Clock>> = vec![
        Box::new(SystemClock::new()),
        Box::new(MonotonicClock::new()),
    ];

    for clock in clocks {
        let time1 = clock.millis();
        thread::sleep(Duration::from_millis(50));
        let time2 = clock.millis();

        assert!(time2 >= time1, "Time should progress or stay the same");
    }
}

#[test]
fn test_clock_trait_object() {
    fn use_clock(clock: &dyn Clock) -> i64 {
        clock.millis()
    }

    let system = SystemClock::new();
    let monotonic = MonotonicClock::new();
    let mock = MockClock::new();

    assert!(use_clock(&system) > 0);
    assert!(use_clock(&monotonic) > 0);
    assert!(use_clock(&mock) > 0);
}

#[test]
fn test_clock_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<SystemClock>();
    assert_send_sync::<MonotonicClock>();
    assert_send_sync::<MockClock>();
}
