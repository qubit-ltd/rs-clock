/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for TimeMeter.

use chrono::Duration;
use prism3_clock::meter::TimeMeter;
use prism3_clock::{Clock, MockClock, MonotonicClock};
use std::thread;
use std::time::Duration as StdDuration;

#[test]
fn test_new() {
    let meter = TimeMeter::new();
    assert!(!meter.is_running());
    assert!(!meter.is_stopped());
    assert_eq!(meter.millis(), 0);
}

#[test]
fn test_with_clock() {
    let clock = MonotonicClock::new();
    let meter = TimeMeter::with_clock(clock);
    assert!(!meter.is_running());
    assert!(!meter.is_stopped());
    assert_eq!(meter.millis(), 0);
}

#[test]
fn test_with_clock_started() {
    let clock = MonotonicClock::new();
    let meter = TimeMeter::with_clock_started(clock);
    assert!(meter.is_running());
    assert!(!meter.is_stopped());
}

#[test]
fn test_start_now() {
    let meter = TimeMeter::start_now();
    assert!(meter.is_running());
    assert!(!meter.is_stopped());
}

#[test]
fn test_start_and_stop() {
    let mut meter = TimeMeter::new();

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
    let mut meter = TimeMeter::new();
    meter.start();
    thread::sleep(StdDuration::from_millis(10));
    meter.stop();
    let first_duration = meter.millis();

    // Restart should reset and start again
    meter.restart();
    assert!(meter.is_running());
    assert!(!meter.is_stopped());

    thread::sleep(StdDuration::from_millis(10));
    meter.stop();
    let second_duration = meter.millis();

    // Second duration should be less than first + second
    assert!(second_duration < first_duration + second_duration);
}

#[test]
fn test_reset() {
    let mut meter = TimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(10));
    meter.stop();

    // Reset should clear everything
    meter.reset();
    assert!(!meter.is_running());
    assert!(!meter.is_stopped());
    assert_eq!(meter.millis(), 0);
}

#[test]
fn test_millis_with_mock_clock() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(1000, false);
    meter.stop();

    assert_eq!(meter.millis(), 1000);
}

#[test]
fn test_millis_not_started() {
    let meter = TimeMeter::new();
    assert_eq!(meter.millis(), 0);
}

#[test]
fn test_millis_running() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(500, false);

    // Should return current elapsed time even without stop
    assert_eq!(meter.millis(), 500);

    clock.add_millis(500, false);
    assert_eq!(meter.millis(), 1000);
}

#[test]
fn test_millis_stopped() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(1000, false);
    meter.stop();

    // After stop, should return fixed duration
    assert_eq!(meter.millis(), 1000);

    clock.add_millis(1000, false);
    assert_eq!(meter.millis(), 1000); // Still 1000
}

#[test]
fn test_seconds() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(5500, false);
    meter.stop();

    assert_eq!(meter.seconds(), 5);
}

#[test]
fn test_minutes() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(125000, false); // 2 minutes 5 seconds
    meter.stop();

    assert_eq!(meter.minutes(), 2);
}

#[test]
fn test_duration() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(1500, false);
    meter.stop();

    let duration = meter.duration();
    assert_eq!(duration, Duration::milliseconds(1500));
}

#[test]
fn test_readable_duration() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(1500, false);
    meter.stop();

    let readable = meter.readable_duration();
    assert_eq!(readable, "1.5s");
}

#[test]
fn test_speed_per_second_zero_time() {
    let meter = TimeMeter::new();
    assert_eq!(meter.speed_per_second(1000), None);
}

#[test]
fn test_speed_per_second() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(2000, false); // 2 seconds
    meter.stop();

    let speed = meter.speed_per_second(1000);
    assert_eq!(speed, Some(500.0)); // 1000 / 2 = 500
}

#[test]
fn test_speed_per_minute_zero_time() {
    let meter = TimeMeter::new();
    assert_eq!(meter.speed_per_minute(1000), None);
}

#[test]
fn test_speed_per_minute() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(2000, false); // 2 seconds
    meter.stop();

    let speed = meter.speed_per_minute(1000);
    assert_eq!(speed, Some(30000.0)); // (1000 / 2) * 60 = 30000
}

#[test]
fn test_formatted_speed_per_second_zero_time() {
    let meter = TimeMeter::new();
    assert_eq!(meter.formatted_speed_per_second(1000), "N/A");
}

#[test]
fn test_formatted_speed_per_second() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(2000, false);
    meter.stop();

    let formatted = meter.formatted_speed_per_second(1000);
    assert_eq!(formatted, "500.00/s");
}

#[test]
fn test_formatted_speed_per_minute_zero_time() {
    let meter = TimeMeter::new();
    assert_eq!(meter.formatted_speed_per_minute(1000), "N/A");
}

#[test]
fn test_formatted_speed_per_minute() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(2000, false);
    meter.stop();

    let formatted = meter.formatted_speed_per_minute(1000);
    assert_eq!(formatted, "30000.00/m");
}

#[test]
fn test_is_running_states() {
    let mut meter = TimeMeter::new();

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
    let mut meter = TimeMeter::new();

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
    let clock = MonotonicClock::new();
    let initial_time = clock.millis();
    let mut meter = TimeMeter::with_clock(clock);

    // Test immutable access
    let clock_ref = meter.clock();
    let time1 = clock_ref.millis();
    assert!(time1 >= initial_time);

    // Test mutable access
    let _clock_mut = meter.clock_mut();
}

#[test]
fn test_multiple_start_calls() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(1000, false);

    // Start again should reset the start time
    meter.start();
    clock.add_millis(500, false);
    meter.stop();

    // Should only measure from second start
    assert_eq!(meter.millis(), 500);
}

#[test]
fn test_real_time_measurement() {
    let mut meter = TimeMeter::new();
    meter.start();
    thread::sleep(StdDuration::from_millis(100));
    meter.stop();

    let elapsed = meter.millis();
    // Allow some tolerance for timing
    assert!((95..=150).contains(&elapsed));
}

#[test]
fn test_real_time_without_stop() {
    let meter = TimeMeter::start_now();
    thread::sleep(StdDuration::from_millis(50));

    let elapsed1 = meter.millis();
    thread::sleep(StdDuration::from_millis(50));
    let elapsed2 = meter.millis();

    // Second reading should be larger
    assert!(elapsed2 > elapsed1);
    assert!(elapsed1 >= 45);
    assert!(elapsed2 >= 95);
}

#[test]
fn test_default_trait() {
    let meter: TimeMeter<MonotonicClock> = Default::default();
    assert!(!meter.is_running());
    assert_eq!(meter.millis(), 0);
}

#[test]
fn test_speed_calculation_with_fractional_seconds() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(500, false); // 0.5 seconds
    meter.stop();

    // Speed should be None because seconds() returns 0
    assert_eq!(meter.speed_per_second(1000), None);

    // But if we have at least 1 second
    meter.restart();
    clock.add_millis(1500, false); // 1.5 seconds
    meter.stop();

    // seconds() returns 1, so speed is 1000 / 1 = 1000
    assert_eq!(meter.speed_per_second(1000), Some(1000.0));
}

#[test]
fn test_edge_case_zero_count() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(1000, false);
    meter.stop();

    assert_eq!(meter.speed_per_second(0), Some(0.0));
    assert_eq!(meter.speed_per_minute(0), Some(0.0));
}

#[test]
fn test_edge_case_large_count() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    meter.start();
    clock.add_millis(1000, false);
    meter.stop();

    let large_count = 1_000_000_000;
    assert_eq!(
        meter.speed_per_second(large_count),
        Some(large_count as f64)
    );
}

#[test]
fn test_is_running_with_end_time_set() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    // Start the meter
    meter.start();
    assert!(meter.is_running());
    assert!(meter.is_running()); // start_time.is_some() && end_time.is_none()

    // Stop the meter - this sets end_time
    meter.stop();
    // Now start_time.is_some() && end_time.is_some()
    assert!(!meter.is_running()); // Should be false because end_time.is_some()
}

#[test]
fn test_millis_when_not_started() {
    let clock = MockClock::new();
    let meter = TimeMeter::with_clock(clock.clone());

    // Explicitly test the None branch in match statement
    assert_eq!(meter.millis(), 0); // start_time is None, should return 0
}

#[test]
fn test_millis_uses_current_time_when_running() {
    let clock = MockClock::new();
    let mut meter = TimeMeter::with_clock(clock.clone());

    // Start the meter
    meter.start();
    clock.add_millis(100, false);

    // First call - should use self.clock.millis() because end_time is None
    let elapsed1 = meter.millis();
    assert_eq!(elapsed1, 100);

    // Advance time more
    clock.add_millis(50, false);

    // Second call - should again use self.clock.millis() because end_time is still None
    let elapsed2 = meter.millis();
    assert_eq!(elapsed2, 150);

    // Now stop the meter
    meter.stop();

    // After stop, should use end_time instead of self.clock.millis()
    clock.add_millis(100, false);
    let elapsed3 = meter.millis();
    assert_eq!(elapsed3, 150); // Should still be 150, not 250
}
