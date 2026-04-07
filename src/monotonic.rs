/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Monotonic clock implementation.
//!
//! This module provides [`MonotonicClock`], a clock implementation that
//! guarantees monotonically increasing time values, unaffected by system time
//! adjustments.
//!
//! # Author
//!
//! Haixing Hu

use crate::Clock;
use chrono::Utc;
use std::time::Instant;

/// A clock implementation that provides monotonically increasing time.
///
/// This clock uses `std::time::Instant` as its time source, which guarantees
/// that time always moves forward and is not affected by system time
/// adjustments (e.g., NTP synchronization, manual changes).
///
/// The clock records a base point when created, and all subsequent time
/// queries are calculated relative to this base point.
///
/// # Use Cases
///
/// - Performance monitoring
/// - Timeout control
/// - Measuring time intervals
/// - Any scenario requiring stable, monotonic time
///
/// # Note
///
/// This clock is designed for measuring time intervals, not for getting the
/// "current time" for display purposes. For timezone support, you can wrap it
/// with [`Zoned`](crate::Zoned), but this is generally not recommended as
/// timezone information is not meaningful for interval measurements.
///
/// # Thread Safety
///
/// This type is completely thread-safe as all fields are immutable after
/// creation.
///
/// # Examples
///
/// ```
/// use qubit_clock::{Clock, MonotonicClock};
/// use std::thread;
/// use std::time::Duration;
///
/// let clock = MonotonicClock::new();
/// let start = clock.millis();
///
/// thread::sleep(Duration::from_millis(100));
///
/// let elapsed = clock.millis() - start;
/// assert!(elapsed >= 100);
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct MonotonicClock {
    /// The base instant when this clock was created.
    instant_base: Instant,
    /// The system time (in milliseconds) when this clock was created.
    system_time_base_millis: i64,
}

impl MonotonicClock {
    /// Creates a new `MonotonicClock`.
    ///
    /// The clock records the current instant and system time as its base
    /// point. All subsequent time queries will be calculated relative to this
    /// base point.
    ///
    /// # Returns
    ///
    /// A new `MonotonicClock` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::MonotonicClock;
    ///
    /// let clock = MonotonicClock::new();
    /// ```
    ///
    pub fn new() -> Self {
        MonotonicClock {
            instant_base: Instant::now(),
            system_time_base_millis: Utc::now().timestamp_millis(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    fn millis(&self) -> i64 {
        let elapsed = self.instant_base.elapsed();
        let elapsed_millis = elapsed.as_millis() as i64;
        self.system_time_base_millis + elapsed_millis
    }
}
