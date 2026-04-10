/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Nanosecond-precision time meter implementation.
//!
//! This module provides [`NanoTimeMeter`], a high-precision time
//! measurement tool with nanosecond precision. For millisecond precision,
//! use [`TimeMeter`](super::TimeMeter).
//!
//! # Author
//!
//! Haixing Hu

use crate::meter::format::{format_duration_nanos, format_speed};
use crate::{NanoClock, NanoMonotonicClock};
use chrono::Duration;

/// A time meter for measuring elapsed time with nanosecond precision.
///
/// This is the high-precision version of [`TimeMeter`](super::TimeMeter),
/// specifically designed for scenarios requiring nanosecond-level
/// precision, such as microbenchmarking and high-frequency operation
/// performance analysis.
///
/// # Differences from TimeMeter
///
/// - **Precision**: NanoTimeMeter provides nanosecond precision,
///   TimeMeter provides millisecond precision
/// - **Performance**: NanoTimeMeter has slightly higher computational
///   overhead, but offers higher precision
/// - **Use Cases**: NanoTimeMeter is suitable for microbenchmarking,
///   TimeMeter is suitable for general business monitoring
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
/// use qubit_clock::meter::NanoTimeMeter;
///
/// // Basic usage
/// let mut meter = NanoTimeMeter::new();
/// meter.start();
/// // Perform high-frequency operations
/// meter.stop();
/// println!("Elapsed: {} nanos", meter.nanos());
/// println!("Elapsed: {}", meter.readable_duration());
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct NanoTimeMeter<C: NanoClock> {
    /// The clock used by this meter.
    clock: C,
    /// Start timestamp in nanoseconds. `None` means not started.
    /// Uses i128 to avoid overflow.
    start_time: Option<i128>,
    /// End timestamp in nanoseconds. `None` means not stopped.
    /// Uses i128 to avoid overflow.
    end_time: Option<i128>,
}

impl<C: NanoClock> NanoTimeMeter<C> {
    /// Creates a new nano time meter with the specified clock.
    ///
    /// # Arguments
    ///
    /// * `clock` - The clock to use for time measurement
    ///
    /// # Returns
    ///
    /// A new `NanoTimeMeter` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{NanoMonotonicClock, meter::NanoTimeMeter};
    ///
    /// let clock = NanoMonotonicClock::new();
    /// let meter = NanoTimeMeter::with_clock(clock);
    /// ```
    #[inline]
    pub fn with_clock(clock: C) -> Self {
        NanoTimeMeter {
            clock,
            start_time: None,
            end_time: None,
        }
    }

    /// Creates a new nano time meter with the specified clock and starts
    /// it immediately.
    ///
    /// # Arguments
    ///
    /// * `clock` - The clock to use for time measurement
    ///
    /// # Returns
    ///
    /// A new `NanoTimeMeter` instance that has already been started
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{NanoMonotonicClock, meter::NanoTimeMeter};
    ///
    /// let clock = NanoMonotonicClock::new();
    /// let meter = NanoTimeMeter::with_clock_started(clock);
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::new();
    /// meter.start();
    /// ```
    #[inline]
    pub fn start(&mut self) {
        self.start_time = Some(self.clock.nanos());
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// ```
    #[inline]
    pub fn stop(&mut self) {
        self.end_time = Some(self.clock.nanos());
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// meter.reset();
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// meter.restart();
    /// // Do more work
    /// ```
    #[inline]
    pub fn restart(&mut self) {
        self.reset();
        self.start();
    }

    /// Returns the elapsed duration in nanoseconds.
    ///
    /// If the meter has been stopped (by calling `stop()`), returns the
    /// time interval from start to stop. If the meter has not been
    /// stopped, returns the time interval from start to the current
    /// moment.
    ///
    /// If the meter has not been started (by calling `start()`), returns
    /// 0.
    ///
    /// To avoid overflow issues, this method uses safe arithmetic
    /// operations.
    ///
    /// # Returns
    ///
    /// The elapsed duration in nanoseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// assert!(meter.nanos() > 0);
    /// ```
    #[inline]
    pub fn nanos(&self) -> i128 {
        let start = match self.start_time {
            Some(t) => t,
            None => return 0,
        };
        let end = self.end_time.unwrap_or_else(|| self.clock.nanos());
        end - start
    }

    /// Returns the elapsed duration in microseconds.
    ///
    /// This method is based on the result of `nanos()`, converting
    /// nanoseconds to microseconds.
    ///
    /// # Returns
    ///
    /// The elapsed duration in microseconds
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// assert!(meter.micros() >= 0);
    /// ```
    #[inline]
    pub fn micros(&self) -> i128 {
        self.nanos() / 1_000
    }

    /// Returns the elapsed duration in milliseconds.
    ///
    /// This method is based on the result of `nanos()`, converting
    /// nanoseconds to milliseconds.
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
    /// use qubit_clock::meter::NanoTimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// thread::sleep(Duration::from_millis(100));
    /// assert!(meter.millis() >= 100);
    /// ```
    #[inline]
    pub fn millis(&self) -> i64 {
        (self.nanos() / 1_000_000) as i64
    }

    /// Returns the elapsed duration in seconds.
    ///
    /// This method is based on the result of `nanos()`, converting
    /// nanoseconds to seconds (rounded down).
    ///
    /// # Returns
    ///
    /// The elapsed duration in seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::meter::NanoTimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// thread::sleep(Duration::from_secs(1));
    /// assert!(meter.seconds() >= 1);
    /// ```
    #[inline]
    pub fn seconds(&self) -> i64 {
        (self.nanos() / 1_000_000_000) as i64
    }

    /// Returns the elapsed duration in minutes.
    ///
    /// This method is based on the result of `nanos()`, converting
    /// nanoseconds to minutes (rounded down).
    ///
    /// # Returns
    ///
    /// The elapsed duration in minutes
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::new();
    /// meter.start();
    /// // Simulate some time
    /// meter.stop();
    /// ```
    #[inline]
    pub fn minutes(&self) -> i64 {
        (self.nanos() / 60_000_000_000) as i64
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
    /// The returned `Duration` object has nanosecond precision.
    ///
    /// # Returns
    ///
    /// The elapsed duration as a `Duration` object (nanosecond precision)
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// let duration = meter.duration();
    /// ```
    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::nanoseconds(self.nanos() as i64)
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// println!("Elapsed: {}", meter.readable_duration());
    /// ```
    #[inline]
    pub fn readable_duration(&self) -> String {
        format_duration_nanos(self.nanos())
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
    /// use qubit_clock::meter::NanoTimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// thread::sleep(Duration::from_secs(1));
    /// meter.stop();
    /// if let Some(speed) = meter.speed_per_second(1000) {
    ///     println!("Speed: {:.2} items/s", speed);
    /// }
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// thread::sleep(Duration::from_secs(1));
    /// meter.stop();
    /// if let Some(speed) = meter.speed_per_minute(1000) {
    ///     println!("Speed: {:.2} items/m", speed);
    /// }
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// println!("Speed: {}", meter.formatted_speed_per_second(1000));
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// // Do some work
    /// meter.stop();
    /// println!("Speed: {}", meter.formatted_speed_per_minute(1000));
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::new();
    /// assert!(!meter.is_running());
    /// meter.start();
    /// assert!(meter.is_running());
    /// meter.stop();
    /// assert!(!meter.is_running());
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::start_now();
    /// assert!(!meter.is_stopped());
    /// meter.stop();
    /// assert!(meter.is_stopped());
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let meter = NanoTimeMeter::new();
    /// let clock = meter.clock();
    /// ```
    #[inline]
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
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let mut meter = NanoTimeMeter::new();
    /// let clock = meter.clock_mut();
    /// ```
    #[inline]
    pub fn clock_mut(&mut self) -> &mut C {
        &mut self.clock
    }
}

impl NanoTimeMeter<NanoMonotonicClock> {
    /// Creates a new nano time meter using the default
    /// `NanoMonotonicClock`.
    ///
    /// The default clock uses `NanoMonotonicClock`, which is based on
    /// `Instant` and is not affected by system time adjustments, making
    /// it more suitable for high-precision time measurement.
    ///
    /// # Returns
    ///
    /// A new `NanoTimeMeter` instance
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let meter = NanoTimeMeter::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self::with_clock(NanoMonotonicClock::new())
    }

    /// Creates a new nano time meter using the default
    /// `NanoMonotonicClock` and starts it immediately.
    ///
    /// # Returns
    ///
    /// A new `NanoTimeMeter` instance that has already been started
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::meter::NanoTimeMeter;
    ///
    /// let meter = NanoTimeMeter::start_now();
    /// ```
    #[inline]
    pub fn start_now() -> Self {
        Self::with_clock_started(NanoMonotonicClock::new())
    }
}

impl Default for NanoTimeMeter<NanoMonotonicClock> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
