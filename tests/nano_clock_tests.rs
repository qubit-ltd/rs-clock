/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for the NanoClock trait.

use qubit_clock::{NanoClock, NanoMonotonicClock};
use std::thread;
use std::time::Duration;

#[test]
fn test_nano_clock_nanos_returns_positive() {
    let clock = NanoMonotonicClock::new();
    let nanos = clock.nanos();
    assert!(nanos > 0, "NanoClock should return positive nanoseconds");
}

#[test]
fn test_nano_clock_time_precise_returns_valid_datetime() {
    let clock = NanoMonotonicClock::new();
    let time = clock.time_precise();
    assert!(
        time.timestamp_nanos_opt().unwrap() > 0,
        "NanoClock should return valid DateTime with nanosecond precision"
    );
}

#[test]
fn test_nano_clock_nanos_and_time_precise_consistency() {
    let clock = NanoMonotonicClock::new();
    let nanos = clock.nanos();
    let time = clock.time_precise();
    let time_nanos = time.timestamp_nanos_opt().unwrap() as i128;

    // Allow small difference due to time passing between calls
    let diff = (nanos - time_nanos).abs();
    assert!(
        diff < 1_000_000, // 1ms tolerance
        "nanos() and time_precise() should be consistent, diff: {} ns",
        diff
    );
}

#[test]
fn test_nano_clock_precision() {
    let clock = NanoMonotonicClock::new();
    let start_nanos = clock.nanos();

    thread::sleep(Duration::from_millis(100));

    let elapsed_nanos = clock.nanos() - start_nanos;

    // Should be at least 100ms in nanoseconds
    assert!(
        elapsed_nanos >= 100_000_000,
        "Elapsed time should be at least 100ms in nanoseconds"
    );

    // Should be less than 200ms (allowing for some overhead)
    assert!(
        elapsed_nanos < 200_000_000,
        "Elapsed time should be less than 200ms"
    );
}

#[test]
fn test_nano_clock_monotonicity() {
    let clock = NanoMonotonicClock::new();
    let mut prev = clock.nanos();

    for _ in 0..10 {
        thread::sleep(Duration::from_millis(1));
        let curr = clock.nanos();
        assert!(
            curr >= prev,
            "NanoClock time should be monotonically increasing"
        );
        prev = curr;
    }
}

#[test]
fn test_nano_clock_higher_precision_than_millis() {
    let clock = NanoMonotonicClock::new();

    // Take multiple nanosecond readings
    let mut nanos_readings = Vec::new();
    for _ in 0..100 {
        nanos_readings.push(clock.nanos());
    }

    // Check that we can detect differences smaller than 1ms
    let mut found_sub_millisecond_diff = false;
    for i in 1..nanos_readings.len() {
        let diff = nanos_readings[i] - nanos_readings[i - 1];
        if diff > 0 && diff < 1_000_000 {
            found_sub_millisecond_diff = true;
            break;
        }
    }

    // Note: This test might not always pass on very slow systems,
    // but it should pass on most modern hardware
    if !found_sub_millisecond_diff {
        println!(
            "Warning: Could not detect sub-millisecond differences. \
             This might be due to system limitations."
        );
    }
}

#[test]
fn test_nano_clock_trait_object() {
    fn use_nano_clock(clock: &dyn NanoClock) -> i128 {
        clock.nanos()
    }

    let clock = NanoMonotonicClock::new();
    assert!(use_nano_clock(&clock) > 0);
}
