/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for MonotonicClock.

use chrono::Datelike;
use qubit_clock::{Clock, MonotonicClock};
use std::thread;
use std::time::Duration;

#[test]
fn test_monotonic_clock_new() {
    let clock = MonotonicClock::new();
    let millis = clock.millis();
    assert!(
        millis > 0,
        "MonotonicClock should return positive milliseconds"
    );
}

#[test]
fn test_monotonic_clock_default() {
    let clock = MonotonicClock::default();
    let millis = clock.millis();
    assert!(millis > 0, "Default MonotonicClock should work");
}

#[test]
fn test_monotonic_clock_millis() {
    let clock = MonotonicClock::new();
    let millis = clock.millis();

    // Should be a reasonable timestamp
    let min_timestamp = 1577836800000i64; // 2020-01-01 in milliseconds
    assert!(
        millis > min_timestamp,
        "MonotonicClock should return a reasonable timestamp"
    );
}

#[test]
fn test_monotonic_clock_time() {
    let clock = MonotonicClock::new();
    let time = clock.time();

    // Should be a reasonable year
    assert!(
        time.year() >= 2020,
        "MonotonicClock should return a reasonable year"
    );
}

#[test]
fn test_monotonic_clock_monotonicity() {
    let clock = MonotonicClock::new();
    let mut prev = clock.millis();

    for _ in 0..100 {
        thread::sleep(Duration::from_millis(1));
        let curr = clock.millis();
        assert!(
            curr >= prev,
            "MonotonicClock time should never go backwards"
        );
        prev = curr;
    }
}

#[test]
fn test_monotonic_clock_elapsed_time() {
    let clock = MonotonicClock::new();
    let start = clock.millis();

    thread::sleep(Duration::from_millis(100));

    let elapsed = clock.millis() - start;

    assert!(
        elapsed >= 100,
        "At least 100ms should have elapsed, got: {}",
        elapsed
    );

    assert!(
        elapsed < 200,
        "Elapsed time should be less than 200ms, got: {}",
        elapsed
    );
}

#[test]
fn test_monotonic_clock_elapsed() {
    let clock = MonotonicClock::new();
    thread::sleep(Duration::from_millis(30));
    assert!(clock.elapsed() >= Duration::from_millis(30));
}

#[test]
fn test_monotonic_clock_monotonic_millis() {
    let clock = MonotonicClock::new();
    let start = clock.monotonic_millis();
    thread::sleep(Duration::from_millis(50));
    let end = clock.monotonic_millis();
    assert!(end >= start + 45);
}

#[test]
fn test_monotonic_clock_consistency() {
    let clock = MonotonicClock::new();
    let millis = clock.millis();
    let time = clock.time();

    // They should be very close
    let diff = (millis - time.timestamp_millis()).abs();
    assert!(
        diff < 10,
        "millis() and time() should be consistent, diff: {}",
        diff
    );
}

#[test]
fn test_monotonic_clock_independent_instances() {
    let clock1 = MonotonicClock::new();
    thread::sleep(Duration::from_millis(50));
    let clock2 = MonotonicClock::new();

    let time1 = clock1.millis();
    let time2 = clock2.millis();

    // clock2 was created later, so it should have a similar or slightly
    // higher base time
    let diff = (time2 - time1).abs();
    assert!(
        diff < 100,
        "Different MonotonicClock instances should have similar times"
    );
}

#[test]
fn test_monotonic_clock_clone() {
    let clock1 = MonotonicClock::new();
    let clock2 = clock1.clone();

    thread::sleep(Duration::from_millis(50));

    let time1 = clock1.millis();
    let time2 = clock2.millis();

    // Cloned clocks share the same base, so they should return the same time
    let diff = (time1 - time2).abs();
    assert!(
        diff < 10,
        "Cloned MonotonicClocks should return the same time"
    );
}

#[test]
fn test_monotonic_clock_long_running() {
    let clock = MonotonicClock::new();
    let start = clock.millis();

    // Simulate a longer running operation
    for _ in 0..5 {
        thread::sleep(Duration::from_millis(20));
        let current = clock.millis();
        assert!(current >= start, "Time should always be >= start time");
    }

    let total_elapsed = clock.millis() - start;
    assert!(total_elapsed >= 100, "At least 100ms should have elapsed");
}

#[test]
fn test_monotonic_clock_debug() {
    let clock = MonotonicClock::new();
    let debug_str = format!("{:?}", clock);
    assert!(
        debug_str.contains("MonotonicClock"),
        "Debug output should contain 'MonotonicClock'"
    );
}

#[test]
fn test_monotonic_clock_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<MonotonicClock>();
    assert_sync::<MonotonicClock>();
}

#[test]
fn test_monotonic_clock_in_thread() {
    use std::sync::Arc;

    let clock = Arc::new(MonotonicClock::new());
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
fn test_monotonic_clock_multiple_threads() {
    use std::sync::Arc;

    let clock = Arc::new(MonotonicClock::new());
    let start = clock.millis();
    let mut handles = vec![];

    for i in 0..10 {
        let clock_clone = clock.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(i * 10));
            clock_clone.millis()
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.join().unwrap());
    }

    // All results should be positive
    for result in &results {
        assert!(*result > 0);
    }

    // All results should be >= start time
    for result in &results {
        assert!(*result >= start, "All results should be >= start time");
    }

    // Find min and max values
    let min = *results.iter().min().unwrap();
    let max = *results.iter().max().unwrap();

    // The difference should be at least 50ms (since threads sleep for
    // different durations)
    let diff = max - min;
    assert!(
        diff >= 50,
        "Time should progress across threads, diff: {}",
        diff
    );
}
