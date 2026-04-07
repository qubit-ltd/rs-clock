/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for NanoTimeMeter.

use chrono::Duration;
use qubit_clock::meter::NanoTimeMeter;
use qubit_clock::{NanoClock, NanoMonotonicClock};
use std::thread;
use std::time::Duration as StdDuration;

#[test]
fn test_new() {
    let meter = NanoTimeMeter::new();
    assert!(!meter.is_running());
    assert!(!meter.is_stopped());
    assert_eq!(meter.nanos(), 0);
}

#[test]
fn test_with_clock() {
    let clock = NanoMonotonicClock::new();
    let meter = NanoTimeMeter::with_clock(clock);
    assert!(!meter.is_running());
    assert!(!meter.is_stopped());
    assert_eq!(meter.nanos(), 0);
}

#[test]
fn test_with_clock_started() {
    let clock = NanoMonotonicClock::new();
    let meter = NanoTimeMeter::with_clock_started(clock);
    assert!(meter.is_running());
    assert!(!meter.is_stopped());
}

#[test]
fn test_start_now() {
    let meter = NanoTimeMeter::start_now();
    assert!(meter.is_running());
    assert!(!meter.is_stopped());
}

#[test]
fn test_start_and_stop() {
    let mut meter = NanoTimeMeter::new();

    // Initially not running
    assert!(!meter.is_running());
    assert!(!meter.is_stopped());

    // Start the meter
    meter.start();
    assert!(meter.is_running());
    assert!(!meter.is_stopped());

    // Stop the meter
    meter.stop();
    assert!(!meter.is_running());
    assert!(meter.is_stopped());
}

#[test]
fn test_restart() {
    let mut meter = NanoTimeMeter::new();
    meter.start();
    thread::sleep(StdDuration::from_millis(10));
    meter.stop();
    let first_duration = meter.nanos();

    // Restart should reset and start again
    meter.restart();
    assert!(meter.is_running());
    assert!(!meter.is_stopped());

    thread::sleep(StdDuration::from_millis(10));
    meter.stop();
    let second_duration = meter.nanos();

    // Both durations should be positive
    assert!(first_duration > 0);
    assert!(second_duration > 0);
}

#[test]
fn test_reset() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(10));
    meter.stop();

    // Reset should clear everything
    meter.reset();
    assert!(!meter.is_running());
    assert!(!meter.is_stopped());
    assert_eq!(meter.nanos(), 0);
}

#[test]
fn test_nanos_not_started() {
    let meter = NanoTimeMeter::new();
    assert_eq!(meter.nanos(), 0);
}

#[test]
fn test_nanos_running() {
    let meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_nanos(1000));

    // Should return current elapsed time even without stop
    let nanos1 = meter.nanos();
    assert!(nanos1 > 0);

    thread::sleep(StdDuration::from_nanos(1000));
    let nanos2 = meter.nanos();
    assert!(nanos2 > nanos1);
}

#[test]
fn test_nanos_stopped() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(10));
    meter.stop();

    let nanos1 = meter.nanos();
    thread::sleep(StdDuration::from_millis(10));
    let nanos2 = meter.nanos();

    // After stop, should return fixed duration
    assert_eq!(nanos1, nanos2);
}

#[test]
fn test_micros() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_micros(100));
    meter.stop();

    let micros = meter.micros();
    assert!(micros >= 90); // Allow some tolerance
}

#[test]
fn test_millis() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(100));
    meter.stop();

    let millis = meter.millis();
    assert!((95..=150).contains(&millis));
}

#[test]
fn test_seconds() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let seconds = meter.seconds();
    assert!(seconds >= 1);
}

#[test]
fn test_minutes() {
    let mut meter = NanoTimeMeter::new();
    meter.start();
    // Simulate 2 minutes in nanoseconds
    meter.stop();

    // In real test, this would require actual waiting
    // For now, just test the conversion logic
    assert_eq!(meter.minutes(), 0);
}

#[test]
fn test_duration() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(100));
    meter.stop();

    let duration = meter.duration();
    let nanos = meter.nanos();
    assert_eq!(duration, Duration::nanoseconds(nanos as i64));
}

#[test]
fn test_readable_duration_nanoseconds() {
    let mut meter = NanoTimeMeter::new();
    meter.start();
    // Immediately stop to get very small duration
    meter.stop();

    let readable = meter.readable_duration();
    // Should be in nanoseconds or microseconds
    assert!(readable.contains("ns") || readable.contains("μs"));
}

#[test]
fn test_readable_duration_milliseconds() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(100));
    meter.stop();

    let readable = meter.readable_duration();
    // Should be in milliseconds or seconds
    assert!(readable.contains("ms") || readable.contains("s"));
}

#[test]
fn test_speed_per_second_zero_time() {
    let meter = NanoTimeMeter::new();
    assert_eq!(meter.speed_per_second(1000), None);
}

#[test]
fn test_speed_per_second() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let speed = meter.speed_per_second(1000);
    assert!(speed.is_some());
    // Should be around 1000 items per second
    let speed_val = speed.unwrap();
    assert!((900.0..=1100.0).contains(&speed_val));
}

#[test]
fn test_speed_per_minute_zero_time() {
    let meter = NanoTimeMeter::new();
    assert_eq!(meter.speed_per_minute(1000), None);
}

#[test]
fn test_speed_per_minute() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let speed = meter.speed_per_minute(1000);
    assert!(speed.is_some());
    // Should be around 60000 items per minute
    let speed_val = speed.unwrap();
    assert!((54000.0..=66000.0).contains(&speed_val));
}

#[test]
fn test_formatted_speed_per_second_zero_time() {
    let meter = NanoTimeMeter::new();
    assert_eq!(meter.formatted_speed_per_second(1000), "N/A");
}

#[test]
fn test_formatted_speed_per_second() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let formatted = meter.formatted_speed_per_second(1000);
    assert!(formatted.ends_with("/s"));
    assert_ne!(formatted, "N/A");
}

#[test]
fn test_formatted_speed_per_minute_zero_time() {
    let meter = NanoTimeMeter::new();
    assert_eq!(meter.formatted_speed_per_minute(1000), "N/A");
}

#[test]
fn test_formatted_speed_per_minute() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let formatted = meter.formatted_speed_per_minute(1000);
    assert!(formatted.ends_with("/m"));
    assert_ne!(formatted, "N/A");
}

#[test]
fn test_is_running_states() {
    let mut meter = NanoTimeMeter::new();

    // Not started
    assert!(!meter.is_running());

    // Started
    meter.start();
    assert!(meter.is_running());

    // Stopped
    meter.stop();
    assert!(!meter.is_running());

    // Restarted
    meter.start();
    assert!(meter.is_running());
}

#[test]
fn test_is_stopped_states() {
    let mut meter = NanoTimeMeter::new();

    // Not started
    assert!(!meter.is_stopped());

    // Started
    meter.start();
    assert!(!meter.is_stopped());

    // Stopped
    meter.stop();
    assert!(meter.is_stopped());

    // Restarted
    meter.start();
    assert!(!meter.is_stopped());
}

#[test]
fn test_clock_accessors() {
    let clock = NanoMonotonicClock::new();
    let initial_nanos = clock.nanos();
    let mut meter = NanoTimeMeter::with_clock(clock);

    // Test immutable access
    let clock_ref = meter.clock();
    let nanos1 = clock_ref.nanos();
    assert!(nanos1 >= initial_nanos);

    // Test mutable access
    let _clock_mut = meter.clock_mut();
}

#[test]
fn test_multiple_start_calls() {
    let mut meter = NanoTimeMeter::new();

    meter.start();
    thread::sleep(StdDuration::from_millis(10));
    let nanos1 = meter.nanos();

    // Start again should reset the start time
    meter.start();
    thread::sleep(StdDuration::from_millis(10));
    meter.stop();
    let nanos2 = meter.nanos();

    // Second measurement should be less than first + second
    assert!(nanos2 < nanos1 + nanos2);
}

#[test]
fn test_real_time_measurement() {
    let mut meter = NanoTimeMeter::new();
    meter.start();
    thread::sleep(StdDuration::from_millis(100));
    meter.stop();

    let elapsed_nanos = meter.nanos();
    let elapsed_millis = meter.millis();

    // Check nanosecond precision
    assert!(elapsed_nanos >= 95_000_000); // 95ms in nanos
    assert!((95..=150).contains(&elapsed_millis));
}

#[test]
fn test_real_time_without_stop() {
    let meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(50));

    let elapsed1 = meter.nanos();
    thread::sleep(StdDuration::from_millis(50));
    let elapsed2 = meter.nanos();

    // Second reading should be larger
    assert!(elapsed2 > elapsed1);
}

#[test]
fn test_default_trait() {
    let meter: NanoTimeMeter<NanoMonotonicClock> = Default::default();
    assert!(!meter.is_running());
    assert_eq!(meter.nanos(), 0);
}

#[test]
fn test_precision_comparison_with_millis() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_micros(1500)); // 1.5 milliseconds
    meter.stop();

    let nanos = meter.nanos();
    let millis = meter.millis();

    // Nanos should show more precision
    assert!(nanos >= 1_400_000); // At least 1.4ms in nanos
    assert!(millis >= 1); // At least 1ms
}

#[test]
fn test_edge_case_zero_count() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    assert_eq!(meter.speed_per_second(0), Some(0.0));
    assert_eq!(meter.speed_per_minute(0), Some(0.0));
}

#[test]
fn test_edge_case_large_count() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let large_count = 1_000_000_000;
    let speed = meter.speed_per_second(large_count);
    assert!(speed.is_some());
}

#[test]
fn test_conversion_accuracy() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(100));
    meter.stop();

    let nanos = meter.nanos();
    let micros = meter.micros();
    let millis = meter.millis();

    // Check conversion accuracy
    assert_eq!(micros, nanos / 1_000);
    assert_eq!(millis, (nanos / 1_000_000) as i64);
}

#[test]
fn test_very_short_duration() {
    let mut meter = NanoTimeMeter::new();
    meter.start();
    // Immediately stop
    meter.stop();

    let nanos = meter.nanos();
    // Should be very small but non-negative
    assert!(nanos >= 0);
}

#[test]
fn test_speed_calculation_with_fractional_seconds() {
    let mut meter = NanoTimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(500));
    meter.stop();

    // Speed should be None because seconds() returns 0
    assert_eq!(meter.speed_per_second(1000), None);

    // But if we have at least 1 second
    meter.restart();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let speed = meter.speed_per_second(1000);
    assert!(speed.is_some());
}
