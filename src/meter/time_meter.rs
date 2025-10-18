/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Millisecond-precision time meter implementation.
//!
//! This module provides [`TimeMeter`], a simple yet powerful time
//! measurement tool with millisecond precision. For nanosecond precision,
//! use [`NanoTimeMeter`](super::NanoTimeMeter).
//!
//! # Author
//!
//! Haixing Hu

use crate::meter::format::{format_duration_millis, format_speed};
use crate::{Clock, MonotonicClock};
use chrono::Duration;

/// A time meter for measuring elapsed time with millisecond precision.
///
/// This meter provides a simple and powerful tool for time measurement
/// with the following features:
///
/// - **Flexible clock source**: Supports any clock implementing `Clock`
///   trait via generic parameter
/// - **High precision**: Uses `MonotonicClock` by default, based on
///   `Instant`
/// - **Easy to use**: Provides simple start/stop interface
/// - **Multiple output formats**: Supports milliseconds, seconds, minutes,
///   and human-readable format
/// - **Speed calculation**: Provides per-second and per-minute speed
///   calculation
/// - **Test-friendly**: Supports injecting `MockClock` for unit testing
///
/// # Design Philosophy
///
/// `TimeMeter` uses dependency injection pattern through the `Clock`
/// trait. This design brings the following benefits:
///
/// 1. **Production reliability**: Uses `MonotonicClock` by default,
///    ensuring time measurement is not affected by system time adjustments
/// 2. **Test controllability**: Can inject `MockClock` for deterministic
///    time testing
/// 3. **Extensibility**: Can implement custom `Clock` to meet special
///    requirements
/// 4. **Compatibility**: Fully compatible with standard Java time API
///
/// # Default Clock Selection
///
/// If no clock is specified, this meter uses `MonotonicClock` as the
/// default clock instead of system clock. This is because:
///
/// - `MonotonicClock` is based on `Instant`, providing monotonically
///   increasing time
/// - Not affected by system time adjustments (e.g., NTP sync, manual
///   settings)
/// - More suitable for performance measurement and benchmarking scenarios
/// - Provides more stable and reliable results in most use cases
///
/// # Thread Safety
///
/// This type is not thread-safe. If you need to use it in a
/// multi-threaded environment, you should create separate instances for
/// each thread or use external synchronization mechanisms.
///
/// # Examples
///
/// ```
/// use prism3_clock::meter::TimeMeter;
/// use std::thread;
/// use std::time::Duration as StdDuration;
///
/// // Basic usage
/// let mut meter = TimeMeter::new();
/// meter.start();
/// thread::sleep(StdDuration::from_millis(100));
/// meter.stop();
/// println!("Elapsed: {}", meter.readable_duration());
///
/// // Auto-start
/// let mut meter = TimeMeter::start_now();
/// thread::sleep(StdDuration::from_millis(50));
/// meter.stop();
///
/// // Real-time monitoring (without calling stop)
/// let mut meter = TimeMeter::start_now();
/// for _ in 0..5 {
///     thread::sleep(StdDuration::from_millis(10));
///     println!("Running: {}", meter.readable_duration());
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct TimeMeter<C: Clock> {
    /// The clock used by this meter.
    clock: C,
    /// Start timestamp in milliseconds. `None` means not started.
    start_time: Option<i64>,
    /// End timestamp in milliseconds. `None` means not stopped.
    end_time: Option<i64>,
}

impl<C: Clock> TimeMeter<C> {
    /// Creates a new time meter with the specified clock.
    ///
    /// # Arguments
    ///
    /// * `clock` - The clock to use for time measurement
    ///
    /// # Returns
    ///
    /// A new `TimeMeter` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::{MonotonicClock, meter::TimeMeter};
    ///
    /// let clock = MonotonicClock::new();
    /// let meter = TimeMeter::with_clock(clock);
    /// ```
    pub fn with_clock(clock: C) -> Self {
        TimeMeter {
            clock,
            start_time: None,
            end_time: None,
        }
    }

    /// Creates a new time meter with the specified clock and starts it
    /// immediately.
    ///
    /// # Arguments
    ///
    /// * `clock` - The clock to use for time measurement
    ///
    /// # Returns
    ///
    /// A new `TimeMeter` instance that has already been started
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::{MonotonicClock, meter::TimeMeter};
    ///
    /// let clock = MonotonicClock::new();
    /// let meter = TimeMeter::with_clock_started(clock);
    /// ```
    pub fn with_clock_started(clock: C) -> Self {
        let mut meter = Self::with_clock(clock);
        meter.start();
        meter
    }

    /// Starts this meter.
    ///
    /// Records the current time as the start timestamp. If the meter has
    /// already been started, this operation will restart timing.
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::new();
    /// meter.start();
    /// ```
    pub fn start(&mut self) {
        self.start_time = Some(self.clock.millis());
        self.end_time = None;
    }

    /// Stops this meter.
    ///
    /// Records the current time as the end timestamp. After calling this
    /// method, `duration()` will return a fixed time interval until
    /// `start()` or `reset()` is called again.
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// ```
    pub fn stop(&mut self) {
        self.end_time = Some(self.clock.millis());
    }

    /// Resets this meter.
    ///
    /// Clears the start and end timestamps, restoring the meter to its
    /// initial state. After reset, you need to call `start()` again to
    /// begin a new time measurement.
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// meter.reset();
    /// ```
    pub fn reset(&mut self) {
        self.start_time = None;
        self.end_time = None;
    }

    /// Resets and immediately starts this meter.
    ///
    /// This is equivalent to calling `reset()` followed by `start()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// // Do some work
    /// meter.restart();
    /// // Do more work
    /// ```
    pub fn restart(&mut self) {
        self.reset();
        self.start();
    }

    /// Returns the elapsed duration in milliseconds.
    ///
    /// If the meter has been stopped (by calling `stop()`), returns the
    /// time interval from start to stop. If the meter has not been
    /// stopped, returns the time interval from start to the current
    /// moment.
    ///
    /// If the meter has not been started (by calling `start()`), returns
    /// 0.
    ///
    /// # Returns
    ///
    /// The elapsed duration in milliseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// thread::sleep(Duration::from_millis(100));
    /// assert!(meter.millis() >= 100);
    /// ```
    pub fn millis(&self) -> i64 {
        let start = match self.start_time {
            Some(t) => t,
            None => return 0,
        };
        let end = self.end_time.unwrap_or_else(|| self.clock.millis());
        end - start
    }

    /// Returns the elapsed duration in seconds.
    ///
    /// This method is based on the result of `millis()`, converting
    /// milliseconds to seconds (rounded down).
    ///
    /// # Returns
    ///
    /// The elapsed duration in seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// thread::sleep(Duration::from_secs(2));
    /// assert!(meter.seconds() >= 2);
    /// ```
    pub fn seconds(&self) -> i64 {
        self.millis() / 1000
    }

    /// Returns the elapsed duration in minutes.
    ///
    /// This method is based on the result of `millis()`, converting
    /// milliseconds to minutes (rounded down).
    ///
    /// # Returns
    ///
    /// The elapsed duration in minutes
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::new();
    /// meter.start();
    /// // Simulate 2 minutes
    /// meter.stop();
    /// // In real usage, this would be >= 2 after 2 minutes
    /// ```
    pub fn minutes(&self) -> i64 {
        self.millis() / 60000
    }

    /// Returns the elapsed duration as a `Duration` object.
    ///
    /// If the meter has been stopped (by calling `stop()`), returns the
    /// time interval from start to stop. If the meter has not been
    /// stopped, returns the time interval from start to the current
    /// moment.
    ///
    /// If the meter has not been started (by calling `start()`), returns
    /// a zero duration.
    ///
    /// # Returns
    ///
    /// The elapsed duration as a `Duration` object (millisecond precision)
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// let duration = meter.duration();
    /// ```
    pub fn duration(&self) -> Duration {
        Duration::milliseconds(self.millis())
    }

    /// Returns a human-readable string representation of the elapsed
    /// duration.
    ///
    /// Formats the duration into an easy-to-read string, such as
    /// "1h 23m 45s" or "2.5s".
    ///
    /// # Returns
    ///
    /// A human-readable string representation of the duration
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// println!("Elapsed: {}", meter.readable_duration());
    /// ```
    pub fn readable_duration(&self) -> String {
        format_duration_millis(self.millis())
    }

    /// Calculates the per-second speed for a given count.
    ///
    /// Computes the average count processed per second during the elapsed
    /// time. Useful for performance monitoring and speed analysis.
    ///
    /// # Arguments
    ///
    /// * `count` - The count value to calculate speed for
    ///
    /// # Returns
    ///
    /// The per-second speed, or `None` if the elapsed time is zero
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// thread::sleep(Duration::from_secs(1));
    /// meter.stop();
    /// if let Some(speed) = meter.speed_per_second(1000) {
    ///     println!("Speed: {:.2} items/s", speed);
    /// }
    /// ```
    pub fn speed_per_second(&self, count: usize) -> Option<f64> {
        let seconds = self.seconds();
        if seconds == 0 {
            None
        } else {
            Some(count as f64 / seconds as f64)
        }
    }

    /// Calculates the per-minute speed for a given count.
    ///
    /// Computes the average count processed per minute during the elapsed
    /// time. Useful for performance monitoring and speed analysis.
    ///
    /// # Arguments
    ///
    /// * `count` - The count value to calculate speed for
    ///
    /// # Returns
    ///
    /// The per-minute speed, or `None` if the elapsed time is zero
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// thread::sleep(Duration::from_secs(1));
    /// meter.stop();
    /// if let Some(speed) = meter.speed_per_minute(1000) {
    ///     println!("Speed: {:.2} items/m", speed);
    /// }
    /// ```
    pub fn speed_per_minute(&self, count: usize) -> Option<f64> {
        let seconds = self.seconds();
        if seconds == 0 {
            None
        } else {
            Some((count as f64 / seconds as f64) * 60.0)
        }
    }

    /// Returns a formatted string of the per-second speed for a given
    /// count.
    ///
    /// # Arguments
    ///
    /// * `count` - The count value to calculate speed for
    ///
    /// # Returns
    ///
    /// A string in the format "{speed}/s", or "N/A" if the elapsed time
    /// is zero
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// println!("Speed: {}", meter.formatted_speed_per_second(1000));
    /// ```
    pub fn formatted_speed_per_second(&self, count: usize) -> String {
        match self.speed_per_second(count) {
            Some(speed) => format_speed(speed, "/s"),
            None => "N/A".to_string(),
        }
    }

    /// Returns a formatted string of the per-minute speed for a given
    /// count.
    ///
    /// # Arguments
    ///
    /// * `count` - The count value to calculate speed for
    ///
    /// # Returns
    ///
    /// A string in the format "{speed}/m", or "N/A" if the elapsed time
    /// is zero
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// println!("Speed: {}", meter.formatted_speed_per_minute(1000));
    /// ```
    pub fn formatted_speed_per_minute(&self, count: usize) -> String {
        match self.speed_per_minute(count) {
            Some(speed) => format_speed(speed, "/m"),
            None => "N/A".to_string(),
        }
    }

    /// Checks if the meter is currently running.
    ///
    /// # Returns
    ///
    /// `true` if the meter has been started but not stopped, `false`
    /// otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::new();
    /// assert!(!meter.is_running());
    /// meter.start();
    /// assert!(meter.is_running());
    /// meter.stop();
    /// assert!(!meter.is_running());
    /// ```
    pub fn is_running(&self) -> bool {
        self.start_time.is_some() && self.end_time.is_none()
    }

    /// Checks if the meter has been stopped.
    ///
    /// # Returns
    ///
    /// `true` if the meter has been stopped, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::start_now();
    /// assert!(!meter.is_stopped());
    /// meter.stop();
    /// assert!(meter.is_stopped());
    /// ```
    pub fn is_stopped(&self) -> bool {
        self.end_time.is_some()
    }

    /// Returns a reference to the clock used by this meter.
    ///
    /// # Returns
    ///
    /// A reference to the clock
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let meter = TimeMeter::new();
    /// let clock = meter.clock();
    /// ```
    pub fn clock(&self) -> &C {
        &self.clock
    }

    /// Returns a mutable reference to the clock used by this meter.
    ///
    /// # Returns
    ///
    /// A mutable reference to the clock
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let mut meter = TimeMeter::new();
    /// let clock = meter.clock_mut();
    /// ```
    pub fn clock_mut(&mut self) -> &mut C {
        &mut self.clock
    }
}

impl TimeMeter<MonotonicClock> {
    /// Creates a new time meter using the default `MonotonicClock`.
    ///
    /// The default clock uses `MonotonicClock`, which is based on
    /// `Instant` and is not affected by system time adjustments, making
    /// it more suitable for time measurement.
    ///
    /// # Returns
    ///
    /// A new `TimeMeter` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let meter = TimeMeter::new();
    /// ```
    pub fn new() -> Self {
        Self::with_clock(MonotonicClock::new())
    }

    /// Creates a new time meter using the default `MonotonicClock` and
    /// starts it immediately.
    ///
    /// # Returns
    ///
    /// A new `TimeMeter` instance that has already been started
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::meter::TimeMeter;
    ///
    /// let meter = TimeMeter::start_now();
    /// ```
    pub fn start_now() -> Self {
        Self::with_clock_started(MonotonicClock::new())
    }
}

impl Default for TimeMeter<MonotonicClock> {
    fn default() -> Self {
        Self::new()
    }
}
