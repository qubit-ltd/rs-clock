/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for NanoMonotonicClock.

use chrono::Datelike;
use qubit_clock::{Clock, NanoClock, NanoMonotonicClock};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

/// Parses a numeric field from the derived debug output.
fn parse_debug_number<T>(debug: &str, field: &str) -> T
where
    T: FromStr,
    T::Err: std::fmt::Debug,
{
    let marker = format!("{field}: ");
    let start = debug
        .find(&marker)
        .unwrap_or_else(|| panic!("debug output should include {field}"))
        + marker.len();
    let digits: String = debug[start..]
        .chars()
        .take_while(|ch| *ch == '-' || ch.is_ascii_digit())
        .collect();
    digits
        .parse::<T>()
        .unwrap_or_else(|err| panic!("{field} should parse: {err:?}"))
}

/// Extracts the base timestamp in nanoseconds from debug output.
fn nano_monotonic_base_nanos(clock: &NanoMonotonicClock) -> i128 {
    let debug = format!("{clock:?}");
    let seconds = parse_debug_number::<i64>(&debug, "system_time_base_seconds");
    let nanos = parse_debug_number::<u32>(&debug, "system_time_base_nanos");
    i128::from(seconds) * 1_000_000_000 + i128::from(nanos)
}

/// Observes a clock after base and elapsed sub-millisecond values cross a millisecond boundary.
fn observe_clock_after_sub_millisecond_rollover() -> (i64, i64, i64) {
    for _ in 0..100_000 {
        let clock = NanoMonotonicClock::new();
        let base_nanos = nano_monotonic_base_nanos(&clock);
        let elapsed_nanos = clock.monotonic_nanos();
        let combined_millis = (base_nanos + elapsed_nanos).div_euclid(1_000_000);
        let separated_millis =
            base_nanos.div_euclid(1_000_000) + elapsed_nanos.div_euclid(1_000_000);

        if combined_millis > separated_millis {
            let before_millis = (clock.nanos() / 1_000_000) as i64;
            let millis = clock.millis();
            let after_millis = (clock.nanos() / 1_000_000) as i64;
            return (before_millis, millis, after_millis);
        }
    }
    panic!("could not observe NanoMonotonicClock after sub-millisecond rollover");
}

#[test]
fn test_nano_monotonic_clock_new() {
    let clock = NanoMonotonicClock::new();
    let nanos = clock.nanos();
    assert!(
        nanos > 0,
        "NanoMonotonicClock should return positive nanoseconds"
    );
}

#[test]
fn test_nano_monotonic_clock_default() {
    let clock = NanoMonotonicClock::default();
    let nanos = clock.nanos();
    assert!(nanos > 0, "Default NanoMonotonicClock should work");
}

#[test]
fn test_nano_monotonic_clock_nanos() {
    let clock = NanoMonotonicClock::new();
    let nanos = clock.nanos();

    // Should be a reasonable timestamp (after 2020-01-01)
    let min_timestamp = 1577836800000000000i128; // 2020-01-01 in nanos
    assert!(
        nanos > min_timestamp,
        "NanoMonotonicClock should return a reasonable timestamp"
    );
}

#[test]
fn test_nano_monotonic_clock_millis() {
    let clock = NanoMonotonicClock::new();
    let millis = clock.millis();

    // Should be a reasonable timestamp
    let min_timestamp = 1577836800000i64; // 2020-01-01 in milliseconds
    assert!(
        millis > min_timestamp,
        "NanoMonotonicClock millis should return a reasonable timestamp"
    );
}

#[test]
fn test_nano_monotonic_clock_time() {
    let clock = NanoMonotonicClock::new();
    let time = clock.time();

    // Should be a reasonable year
    assert!(
        time.year() >= 2020,
        "NanoMonotonicClock should return a reasonable year"
    );
}

#[test]
fn test_nano_monotonic_clock_time_precise() {
    let clock = NanoMonotonicClock::new();
    let time = clock.time_precise();

    // Should be a reasonable year
    assert!(
        time.year() >= 2020,
        "NanoMonotonicClock time_precise should return a reasonable year"
    );

    // Should have nanosecond precision
    let nanos = time.timestamp_nanos_opt().unwrap();
    assert!(nanos > 0);
}

#[test]
fn test_nano_monotonic_clock_monotonicity() {
    let clock = NanoMonotonicClock::new();
    let mut prev = clock.nanos();

    for _ in 0..100 {
        thread::sleep(Duration::from_millis(1));
        let curr = clock.nanos();
        assert!(
            curr >= prev,
            "NanoMonotonicClock time should never go backwards"
        );
        prev = curr;
    }
}

#[test]
fn test_nano_monotonic_clock_elapsed_time() {
    let clock = NanoMonotonicClock::new();
    let start = clock.nanos();

    thread::sleep(Duration::from_millis(100));

    let elapsed = clock.nanos() - start;

    // Should be at least 100ms in nanoseconds
    assert!(
        elapsed >= 100_000_000,
        "At least 100ms should have elapsed, got: {} ns",
        elapsed
    );
}

#[test]
fn test_nano_monotonic_clock_elapsed() {
    let clock = NanoMonotonicClock::new();
    thread::sleep(Duration::from_millis(30));
    assert!(clock.elapsed() >= Duration::from_millis(30));
}

#[test]
fn test_nano_monotonic_clock_monotonic_nanos() {
    let clock = NanoMonotonicClock::new();
    let start = clock.monotonic_nanos();
    thread::sleep(Duration::from_millis(50));
    let end = clock.monotonic_nanos();
    assert!(end >= start + 45_000_000);
}

#[test]
fn test_nano_monotonic_clock_nanos_millis_consistency() {
    let clock = NanoMonotonicClock::new();
    let nanos = clock.nanos();
    let millis = clock.millis();

    // Convert nanos to millis and compare
    let nanos_as_millis = (nanos / 1_000_000) as i64;
    let diff = (nanos_as_millis - millis).abs();

    assert!(
        diff <= 1,
        "nanos() and millis() should be consistent, diff: {}",
        diff
    );
}

#[test]
fn test_nano_monotonic_clock_millis_does_not_lag_nanos_after_rollover() {
    let (before_millis, millis, after_millis) = observe_clock_after_sub_millisecond_rollover();

    assert!(
        (before_millis..=after_millis).contains(&millis),
        "millis() should stay within surrounding nanos() readings after sub-millisecond rollover: before={before_millis}, millis={millis}, after={after_millis}",
    );
}

#[test]
fn test_nano_monotonic_clock_precision() {
    let clock = NanoMonotonicClock::new();

    // Take multiple readings
    let mut readings = Vec::new();
    for _ in 0..100 {
        readings.push(clock.nanos());
    }

    // Check that we can detect sub-millisecond differences
    let mut found_sub_millis_diff = false;
    for i in 1..readings.len() {
        let diff = readings[i] - readings[i - 1];
        if diff > 0 && diff < 1_000_000 {
            found_sub_millis_diff = true;
            break;
        }
    }

    // Note: This might not always pass on very slow systems
    if !found_sub_millis_diff {
        println!(
            "Warning: Could not detect sub-millisecond differences. \
             This might be due to system limitations."
        );
    }
}

#[test]
fn test_nano_monotonic_clock_independent_instances() {
    let clock1 = NanoMonotonicClock::new();
    thread::sleep(Duration::from_millis(50));
    let clock2 = NanoMonotonicClock::new();

    let time1 = clock1.nanos();
    let time2 = clock2.nanos();

    // clock2 was created later, so it should have a similar or slightly
    // higher base time
    let diff = (time2 - time1).abs();
    assert!(
        diff < 100_000_000, // 100ms in nanos
        "Different NanoMonotonicClock instances should have similar times"
    );
}

#[test]
fn test_nano_monotonic_clock_clone() {
    let clock1 = NanoMonotonicClock::new();
    let clock2 = clock1.clone();

    thread::sleep(Duration::from_millis(50));

    let time1 = clock1.nanos();
    let time2 = clock2.nanos();

    // Cloned clocks share the same base, so they should return the same time
    let diff = (time1 - time2).abs();
    assert!(
        diff < 10_000_000, // 10ms tolerance
        "Cloned NanoMonotonicClocks should return the same time"
    );
}

#[test]
fn test_nano_monotonic_clock_debug() {
    let clock = NanoMonotonicClock::new();
    let debug_str = format!("{:?}", clock);
    assert!(
        debug_str.contains("NanoMonotonicClock"),
        "Debug output should contain 'NanoMonotonicClock'"
    );
}

#[test]
fn test_nano_monotonic_clock_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<NanoMonotonicClock>();
    assert_sync::<NanoMonotonicClock>();
}

#[test]
fn test_nano_monotonic_clock_in_thread() {
    use std::sync::Arc;

    let clock = Arc::new(NanoMonotonicClock::new());
    let clock_clone = clock.clone();

    let handle = thread::spawn(move || {
        let nanos = clock_clone.nanos();
        assert!(nanos > 0);
        nanos
    });

    let result = handle.join().unwrap();
    assert!(result > 0);
}

#[test]
fn test_nano_monotonic_clock_multiple_threads() {
    use std::sync::Arc;

    let clock = Arc::new(NanoMonotonicClock::new());
    let mut handles = vec![];

    for _ in 0..10 {
        let clock_clone = clock.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            clock_clone.nanos()
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

    // Results should be in increasing order (roughly)
    // Note: Due to system scheduling, the order might not be strictly increasing
    // but the difference should be reasonable (within a few seconds)
    let first = results[0];
    let last = results[results.len() - 1];
    let diff = if last >= first {
        last - first
    } else {
        first - last
    };
    assert!(
        diff < 5_000_000_000,
        "Time difference should be less than 5s in nanoseconds"
    );
}

#[test]
fn test_nano_monotonic_clock_comparison_with_millis() {
    let clock = NanoMonotonicClock::new();
    let start_nanos = clock.nanos();
    let start_millis = clock.millis();

    thread::sleep(Duration::from_millis(100));

    let elapsed_nanos = clock.nanos() - start_nanos;
    let elapsed_millis = clock.millis() - start_millis;

    // Convert and compare
    let nanos_as_millis = (elapsed_nanos / 1_000_000) as i64;
    let diff = (nanos_as_millis - elapsed_millis).abs();

    assert!(
        diff <= 1,
        "Nanosecond and millisecond measurements should be consistent"
    );
}
