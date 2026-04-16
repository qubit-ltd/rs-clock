/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! High-precision clock trait providing nanosecond accuracy.
//!
//! This module defines the [`NanoClock`] trait, which extends [`Clock`] to
//! provide nanosecond-precision time measurements.
//!
//! # Author
//!
//! Haixing Hu

use crate::Clock;
use chrono::{DateTime, Utc};

/// A trait representing a clock with nanosecond precision.
///
/// This trait extends [`Clock`] to provide high-precision time measurements
/// at the nanosecond level. It's useful for performance testing,
/// microbenchmarking, and scenarios requiring very precise time measurements.
///
/// # Note
///
/// The nanosecond timestamp is stored as an `i128` to avoid overflow issues.
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
pub trait NanoClock: Clock {
    /// Returns the current time as a Unix timestamp in nanoseconds (UTC).
    ///
    /// The timestamp represents the number of nanoseconds since the Unix
    /// epoch (1970-01-01 00:00:00 UTC).
    ///
    /// # Returns
    ///
    /// The current time as nanoseconds since the Unix epoch.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{NanoClock, NanoMonotonicClock};
    ///
    /// let clock = NanoMonotonicClock::new();
    /// let nanos = clock.nanos();
    /// assert!(nanos > 0);
    /// ```
    fn nanos(&self) -> i128;

    /// Returns the current time as a `DateTime<Utc>` with nanosecond
    /// precision.
    ///
    /// This method has a default implementation that constructs a
    /// `DateTime<Utc>` from the result of [`nanos()`](NanoClock::nanos).
    ///
    /// # Returns
    ///
    /// The current time as a `DateTime<Utc>` object with nanosecond
    /// precision.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{NanoClock, NanoMonotonicClock};
    ///
    /// let clock = NanoMonotonicClock::new();
    /// let time = clock.time_precise();
    /// println!("Current time (precise): {}", time);
    /// ```
    #[inline]
    fn time_precise(&self) -> DateTime<Utc> {
        let nanos = self.nanos();
        let secs = nanos.div_euclid(1_000_000_000);
        let nsecs = nanos.rem_euclid(1_000_000_000) as u32;
        let secs = match i64::try_from(secs) {
            Ok(value) => value,
            Err(_) => return DateTime::<Utc>::UNIX_EPOCH,
        };
        DateTime::from_timestamp(secs, nsecs).unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
    }
}
