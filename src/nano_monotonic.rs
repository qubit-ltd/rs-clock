/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! High-precision monotonic clock implementation.
//!
//! This module provides [`NanoMonotonicClock`], a clock implementation that
//! provides nanosecond-precision monotonic time measurements.
//!
//! # Author
//!
//! Haixing Hu

use crate::{Clock, NanoClock};
use chrono::Utc;
use std::time::{Duration, Instant};

/// A clock implementation that provides nanosecond-precision monotonic time.
///
/// This clock combines the monotonic guarantees of
/// [`MonotonicClock`](crate::MonotonicClock) with the nanosecond precision
/// of [`NanoClock`]. It uses `std::time::Instant` as its time source and
/// stores the base time with nanosecond precision.
///
/// # Use Cases
///
/// - High-precision performance testing
/// - Microbenchmarking
/// - Scenarios requiring nanosecond-level time measurements
///
/// # Thread Safety
///
/// This type is completely thread-safe as all fields are immutable after
/// creation.
///
/// # Examples
///
/// ```
/// use qubit_clock::{NanoClock, NanoMonotonicClock};
///
/// let clock = NanoMonotonicClock::new();
/// let start = clock.nanos();
///
/// // Perform some operation
/// for _ in 0..1000 {
///     // Some work
/// }
///
/// let elapsed = clock.nanos() - start;
/// println!("Elapsed: {} ns", elapsed);
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct NanoMonotonicClock {
    /// The base instant when this clock was created.
    instant_base: Instant,
    /// The system time (seconds part) when this clock was created.
    system_time_base_seconds: i64,
    /// The system time (nanoseconds part) when this clock was created.
    system_time_base_nanos: u32,
}

impl NanoMonotonicClock {
    /// Creates a new `NanoMonotonicClock`.
    ///
    /// The clock records the current instant and system time (with nanosecond
    /// precision) as its base point. All subsequent time queries will be
    /// calculated relative to this base point.
    ///
    /// # Returns
    ///
    /// A new `NanoMonotonicClock` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::NanoMonotonicClock;
    ///
    /// let clock = NanoMonotonicClock::new();
    /// ```
    ///
    #[inline]
    pub fn new() -> Self {
        let now = Utc::now();
        NanoMonotonicClock {
            instant_base: Instant::now(),
            system_time_base_seconds: now.timestamp(),
            system_time_base_nanos: now.timestamp_subsec_nanos(),
        }
    }

    /// Returns the elapsed monotonic duration since this clock was created.
    ///
    /// This value is based purely on `Instant` and is not affected by system
    /// time adjustments.
    ///
    /// # Returns
    ///
    /// The elapsed monotonic duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::NanoMonotonicClock;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let clock = NanoMonotonicClock::new();
    /// thread::sleep(Duration::from_millis(10));
    /// assert!(clock.elapsed() >= Duration::from_millis(10));
    /// ```
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.instant_base.elapsed()
    }

    /// Returns the elapsed monotonic time in nanoseconds since creation.
    ///
    /// Unlike [`NanoClock::nanos`](crate::NanoClock::nanos), this value does
    /// not include a wall-clock epoch anchor and is intended for interval
    /// measurement.
    ///
    /// # Returns
    ///
    /// The elapsed monotonic nanoseconds, saturated at `i128::MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::NanoMonotonicClock;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let clock = NanoMonotonicClock::new();
    /// thread::sleep(Duration::from_millis(10));
    /// assert!(clock.monotonic_nanos() >= 10_000_000);
    /// ```
    #[inline]
    pub fn monotonic_nanos(&self) -> i128 {
        let elapsed_nanos = self.elapsed().as_nanos();
        i128::try_from(elapsed_nanos).unwrap_or(i128::MAX)
    }
}

impl Default for NanoMonotonicClock {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for NanoMonotonicClock {
    #[inline]
    fn millis(&self) -> i64 {
        let elapsed_millis = self.elapsed().as_millis();
        let elapsed_millis = i64::try_from(elapsed_millis).unwrap_or(i64::MAX);
        let base_millis =
            self.system_time_base_seconds * 1000 + (self.system_time_base_nanos / 1_000_000) as i64;
        base_millis.saturating_add(elapsed_millis)
    }
}

impl NanoClock for NanoMonotonicClock {
    #[inline]
    fn nanos(&self) -> i128 {
        let elapsed_nanos = self.monotonic_nanos();
        let base_nanos = (self.system_time_base_seconds as i128) * 1_000_000_000
            + (self.system_time_base_nanos as i128);
        base_nanos.saturating_add(elapsed_nanos)
    }
}
