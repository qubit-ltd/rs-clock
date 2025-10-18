/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
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
use std::time::Instant;

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
/// use prism3_clock::{NanoClock, NanoMonotonicClock};
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
    /// use prism3_clock::NanoMonotonicClock;
    ///
    /// let clock = NanoMonotonicClock::new();
    /// ```
    ///
    pub fn new() -> Self {
        let now = Utc::now();
        NanoMonotonicClock {
            instant_base: Instant::now(),
            system_time_base_seconds: now.timestamp(),
            system_time_base_nanos: now.timestamp_subsec_nanos(),
        }
    }
}

impl Default for NanoMonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for NanoMonotonicClock {
    fn millis(&self) -> i64 {
        let elapsed = self.instant_base.elapsed();
        let elapsed_millis = elapsed.as_millis() as i64;
        let base_millis =
            self.system_time_base_seconds * 1000 + (self.system_time_base_nanos / 1_000_000) as i64;
        base_millis + elapsed_millis
    }
}

impl NanoClock for NanoMonotonicClock {
    fn nanos(&self) -> i128 {
        let elapsed = self.instant_base.elapsed();
        let elapsed_nanos = elapsed.as_nanos() as i128;
        let base_nanos = (self.system_time_base_seconds as i128) * 1_000_000_000
            + (self.system_time_base_nanos as i128);
        base_nanos + elapsed_nanos
    }
}
