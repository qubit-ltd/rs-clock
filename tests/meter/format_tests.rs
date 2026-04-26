/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for duration and speed formatting utilities.

use qubit_clock::meter::{format_duration_millis, format_duration_nanos, format_speed};

#[test]
fn test_format_duration_millis_negative() {
    assert_eq!(format_duration_millis(-100), "0 ms");
    assert_eq!(format_duration_millis(-1), "0 ms");
}

#[test]
fn test_format_duration_millis_zero() {
    assert_eq!(format_duration_millis(0), "0 ms");
}

#[test]
fn test_format_duration_millis_less_than_second() {
    assert_eq!(format_duration_millis(1), "1 ms");
    assert_eq!(format_duration_millis(100), "100 ms");
    assert_eq!(format_duration_millis(500), "500 ms");
    assert_eq!(format_duration_millis(999), "999 ms");
}

#[test]
fn test_format_duration_millis_seconds_with_fraction() {
    assert_eq!(format_duration_millis(1000), "1s");
    assert_eq!(format_duration_millis(1100), "1.1s");
    assert_eq!(format_duration_millis(1500), "1.5s");
    assert_eq!(format_duration_millis(1949), "1.9s");
    assert_eq!(format_duration_millis(1950), "2s");
    assert_eq!(format_duration_millis(1999), "2s");
    assert_eq!(format_duration_millis(2000), "2s");
}

#[test]
fn test_format_duration_millis_minutes_and_seconds() {
    assert_eq!(format_duration_millis(60000), "1m");
    assert_eq!(format_duration_millis(61000), "1m 1s");
    assert_eq!(format_duration_millis(65000), "1m 5s");
    assert_eq!(format_duration_millis(59950), "1m");
    assert_eq!(format_duration_millis(119000), "1m 59s");
    assert_eq!(format_duration_millis(119500), "2m");
    assert_eq!(format_duration_millis(120000), "2m");
}

#[test]
fn test_format_duration_millis_hours_minutes_seconds() {
    assert_eq!(format_duration_millis(3600000), "1h");
    assert_eq!(format_duration_millis(3660000), "1h 1m");
    assert_eq!(format_duration_millis(3661000), "1h 1m 1s");
    assert_eq!(format_duration_millis(3665000), "1h 1m 5s");
    assert_eq!(format_duration_millis(7200000), "2h");
    assert_eq!(format_duration_millis(7260000), "2h 1m");
    assert_eq!(format_duration_millis(7261000), "2h 1m 1s");
}

#[test]
fn test_format_duration_millis_hours_and_seconds_only() {
    assert_eq!(format_duration_millis(3601000), "1h 1s");
}

#[test]
fn test_format_duration_millis_large_values() {
    // 24 hours
    assert_eq!(format_duration_millis(86400000), "24h");
    // 25 hours 30 minutes 45 seconds
    assert_eq!(format_duration_millis(91845000), "25h 30m 45s");
}

#[test]
fn test_format_duration_nanos_negative() {
    assert_eq!(format_duration_nanos(-100), "0 ns");
    assert_eq!(format_duration_nanos(-1), "0 ns");
}

#[test]
fn test_format_duration_nanos_zero() {
    assert_eq!(format_duration_nanos(0), "0 ns");
}

#[test]
fn test_format_duration_nanos_less_than_microsecond() {
    assert_eq!(format_duration_nanos(1), "1 ns");
    assert_eq!(format_duration_nanos(100), "100 ns");
    assert_eq!(format_duration_nanos(500), "500 ns");
    assert_eq!(format_duration_nanos(999), "999 ns");
}

#[test]
fn test_format_duration_nanos_microseconds() {
    assert_eq!(format_duration_nanos(1000), "1 μs");
    assert_eq!(format_duration_nanos(1100), "1.1 μs");
    assert_eq!(format_duration_nanos(1500), "1.5 μs");
    assert_eq!(format_duration_nanos(1949), "1.9 μs");
    assert_eq!(format_duration_nanos(1950), "2 μs");
    assert_eq!(format_duration_nanos(1999), "2 μs");
    assert_eq!(format_duration_nanos(2000), "2 μs");
    assert_eq!(format_duration_nanos(999949), "999.9 μs");
    assert_eq!(format_duration_nanos(999950), "1 ms");
    assert_eq!(format_duration_nanos(999999), "1 ms");
}

#[test]
fn test_format_duration_nanos_milliseconds() {
    assert_eq!(format_duration_nanos(1_000_000), "1 ms");
    assert_eq!(format_duration_nanos(1_100_000), "1.1 ms");
    assert_eq!(format_duration_nanos(1_500_000), "1.5 ms");
    assert_eq!(format_duration_nanos(1_949_000), "1.9 ms");
    assert_eq!(format_duration_nanos(1_950_000), "2 ms");
    assert_eq!(format_duration_nanos(1_999_000), "2 ms");
    assert_eq!(format_duration_nanos(2_000_000), "2 ms");
    assert_eq!(format_duration_nanos(999_949_999), "999.9 ms");
    assert_eq!(format_duration_nanos(999_950_000), "1s");
    assert_eq!(format_duration_nanos(999_999_999), "1s");
}

#[test]
fn test_format_duration_nanos_seconds_and_above() {
    // 1 second
    assert_eq!(format_duration_nanos(1_000_000_000), "1s");
    // 1.5 seconds
    assert_eq!(format_duration_nanos(1_500_000_000), "1.5s");
    // 1.949999999 seconds rounds directly to 1 decimal place without double rounding
    assert_eq!(format_duration_nanos(1_949_999_999), "1.9s");
    // 1.95 seconds rounds to 2 seconds
    assert_eq!(format_duration_nanos(1_950_000_000), "2s");
    // 1.999 seconds rounds to 2 seconds
    assert_eq!(format_duration_nanos(1_999_000_000), "2s");
    // 59.949999999 seconds stays below the minute boundary
    assert_eq!(format_duration_nanos(59_949_999_999), "59.9s");
    // 59.95 seconds rounds to 1 minute
    assert_eq!(format_duration_nanos(59_950_000_000), "1m");
    // 1 minute
    assert_eq!(format_duration_nanos(60_000_000_000), "1m");
    // Durations of at least 1 minute round to whole seconds
    assert_eq!(format_duration_nanos(60_499_999_999), "1m");
    assert_eq!(format_duration_nanos(60_500_000_000), "1m 1s");
    // 1 hour
    assert_eq!(format_duration_nanos(3_600_000_000_000), "1h");
}

#[test]
fn test_format_duration_nanos_extreme_positive_value() {
    assert_eq!(
        format_duration_nanos(i128::MAX),
        "47261439850130342147690917h 41m 56s",
    );
}

#[test]
fn test_format_speed_normal_values() {
    assert_eq!(format_speed(0.0, "/s"), "0.00/s");
    assert_eq!(format_speed(1.0, "/s"), "1.00/s");
    assert_eq!(format_speed(123.456, "/s"), "123.46/s");
    assert_eq!(format_speed(123.454, "/s"), "123.45/s");
    assert_eq!(format_speed(0.001, "/m"), "0.00/m");
    assert_eq!(format_speed(999.999, "/m"), "1000.00/m");
}

#[test]
fn test_format_speed_special_values() {
    assert_eq!(format_speed(f64::NAN, "/s"), "N/A");
    assert_eq!(format_speed(f64::INFINITY, "/s"), "N/A");
    assert_eq!(format_speed(f64::NEG_INFINITY, "/s"), "N/A");
}

#[test]
fn test_format_speed_negative_values() {
    assert_eq!(format_speed(-1.0, "/s"), "-1.00/s");
    assert_eq!(format_speed(-123.456, "/s"), "-123.46/s");
}

#[test]
fn test_format_speed_different_units() {
    assert_eq!(format_speed(100.0, "/s"), "100.00/s");
    assert_eq!(format_speed(100.0, "/m"), "100.00/m");
    assert_eq!(format_speed(100.0, "/h"), "100.00/h");
    assert_eq!(format_speed(100.0, " items/sec"), "100.00 items/sec");
}

#[test]
fn test_format_speed_rounding() {
    assert_eq!(format_speed(1.234, "/s"), "1.23/s");
    assert_eq!(format_speed(1.235, "/s"), "1.24/s");
    assert_eq!(format_speed(1.995, "/s"), "2.00/s");
    assert_eq!(format_speed(1.999, "/s"), "2.00/s");
}
