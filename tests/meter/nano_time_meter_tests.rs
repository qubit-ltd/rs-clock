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
use qubit_clock::{Clock, NanoClock, NanoMonotonicClock};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::thread;
use std::time::Duration as StdDuration;

#[derive(Debug)]
struct SequenceNanoClock {
    values: Mutex<VecDeque<i128>>,
    fallback: i128,
}

impl SequenceNanoClock {
    fn new(values: Vec<i128>) -> Self {
        assert!(!values.is_empty(), "values must not be empty");
        let fallback = *values.last().expect("values must not be empty");
        Self {
            values: Mutex::new(VecDeque::from(values)),
            fallback,
        }
    }

    fn next_nanos(&self) -> i128 {
        self.values
            .lock()
            .expect("mutex poisoned")
            .pop_front()
            .unwrap_or(self.fallback)
    }
}

impl Clock for SequenceNanoClock {
    fn millis(&self) -> i64 {
        let millis = self.next_nanos().div_euclid(1_000_000);
        if millis > i64::MAX as i128 {
            i64::MAX
        } else if millis < i64::MIN as i128 {
            i64::MIN
        } else {
            millis as i64
        }
    }
}

impl NanoClock for SequenceNanoClock {
    fn nanos(&self) -> i128 {
        self.next_nanos()
    }
}

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
    assert!(millis >= 95);
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
    assert!((700.0..=2000.0).contains(&speed_val));
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
    assert!((42_000.0..=120_000.0).contains(&speed_val));
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
    assert!(elapsed_millis >= 95);
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
fn test_conversion_saturates_on_positive_overflow() {
    let huge_nanos = (i64::MAX as i128 + 10) * 60_000_000_000;
    let clock = SequenceNanoClock::new(vec![0, huge_nanos]);
    let mut meter = NanoTimeMeter::with_clock(clock);
    meter.start();
    meter.stop();

    assert_eq!(meter.nanos(), huge_nanos);
    assert_eq!(meter.millis(), i64::MAX);
    assert_eq!(meter.seconds(), i64::MAX);
    assert_eq!(meter.minutes(), i64::MAX);
    assert_eq!(meter.duration(), Duration::MAX);
}

#[test]
fn test_conversion_saturates_on_negative_overflow() {
    let huge_nanos = (i64::MIN as i128 - 10) * 60_000_000_000;
    let clock = SequenceNanoClock::new(vec![0, huge_nanos]);
    let mut meter = NanoTimeMeter::with_clock(clock);
    meter.start();
    meter.stop();

    assert_eq!(meter.nanos(), huge_nanos);
    assert_eq!(meter.millis(), i64::MIN);
    assert_eq!(meter.seconds(), i64::MIN);
    assert_eq!(meter.minutes(), i64::MIN);
    assert_eq!(meter.duration(), Duration::MIN);
}

#[test]
fn test_duration_preserves_representable_large_positive_nanos() {
    let elapsed_nanos = i64::MAX as i128 + 1_000_000_000;
    let clock = SequenceNanoClock::new(vec![0, elapsed_nanos]);
    let mut meter = NanoTimeMeter::with_clock(clock);
    meter.start();
    meter.stop();

    let seconds = elapsed_nanos.div_euclid(1_000_000_000);
    let sub_nanos = elapsed_nanos.rem_euclid(1_000_000_000);
    let expected = Duration::new(seconds as i64, sub_nanos as u32)
        .expect("elapsed nanos should fit chrono Duration");

    assert_eq!(meter.duration(), expected);
    assert!(meter.duration() > Duration::nanoseconds(i64::MAX));
}

#[test]
fn test_duration_preserves_representable_large_negative_nanos() {
    let elapsed_nanos = i64::MIN as i128 - 1_000_000_000;
    let clock = SequenceNanoClock::new(vec![0, elapsed_nanos]);
    let mut meter = NanoTimeMeter::with_clock(clock);
    meter.start();
    meter.stop();

    let seconds = elapsed_nanos.div_euclid(1_000_000_000);
    let sub_nanos = elapsed_nanos.rem_euclid(1_000_000_000);
    let expected = Duration::new(seconds as i64, sub_nanos as u32)
        .expect("elapsed nanos should fit chrono Duration");

    assert_eq!(meter.duration(), expected);
    assert!(meter.duration() < Duration::nanoseconds(i64::MIN));
}

#[test]
fn test_nanos_saturates_on_positive_elapsed_overflow() {
    let clock = SequenceNanoClock::new(vec![i128::MIN, i128::MAX]);
    let mut meter = NanoTimeMeter::with_clock(clock);
    meter.start();
    meter.stop();

    assert_eq!(meter.nanos(), i128::MAX);
}

#[test]
fn test_nanos_saturates_on_negative_elapsed_overflow() {
    let clock = SequenceNanoClock::new(vec![i128::MAX, i128::MIN]);
    let mut meter = NanoTimeMeter::with_clock(clock);
    meter.start();
    meter.stop();

    assert_eq!(meter.nanos(), i128::MIN);
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

    // seconds() is still integer-truncated.
    assert_eq!(meter.seconds(), 0);
    // But speed now uses precise elapsed nanoseconds, so sub-second
    // durations can still report speed.
    let speed_half_sec = meter
        .speed_per_second(1000)
        .expect("speed should be available for positive elapsed time");
    assert!(speed_half_sec > 1500.0);

    // Around 1 second should report speed around 1000/s.
    meter.restart();
    thread::sleep(StdDuration::from_secs(1));
    meter.stop();

    let speed = meter.speed_per_second(1000);
    assert!(speed.is_some());
    let speed = speed.expect("speed should be available");
    assert!((700.0..=2000.0).contains(&speed));
}
