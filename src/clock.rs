/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! The base clock trait providing UTC time.
//!
//! This module defines the [`Clock`] trait, which is the foundation of the
//! clock abstraction. All clock implementations must implement this trait.
//!
//! # Author
//!
//! Haixing Hu

use chrono::{DateTime, Utc};

/// Clamps an out-of-range millisecond timestamp to chrono's nearest boundary.
fn clamp_out_of_range_millis(millis: i64) -> DateTime<Utc> {
    if millis < 0 {
        DateTime::<Utc>::MIN_UTC
    } else {
        DateTime::<Utc>::MAX_UTC
    }
}

/// A trait representing a clock that provides UTC time.
///
/// This is the base trait for all clock implementations. It provides methods
/// to get the current time as a Unix timestamp (milliseconds) or as a
/// `DateTime<Utc>` object.
///
/// All methods return **UTC time** only. For timezone support, see
/// [`ZonedClock`](crate::ZonedClock).
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to ensure thread safety.
///
/// # Examples
///
/// ```
/// use qubit_clock::{Clock, SystemClock};
///
/// let clock = SystemClock::new();
/// let timestamp = clock.millis();
/// let time = clock.time();
/// println!("Current time: {}", time);
/// ```
///
/// # Author
///
/// Haixing Hu
pub trait Clock: Send + Sync {
    /// Returns the current time as a Unix timestamp in milliseconds (UTC).
    ///
    /// The timestamp represents the number of milliseconds since the Unix
    /// epoch (1970-01-01 00:00:00 UTC).
    ///
    /// # Returns
    ///
    /// The current time as milliseconds since the Unix epoch.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{Clock, SystemClock};
    ///
    /// let clock = SystemClock::new();
    /// let millis = clock.millis();
    /// assert!(millis > 0);
    /// ```
    fn millis(&self) -> i64;

    /// Returns the current time as a `DateTime<Utc>`.
    ///
    /// This method has a default implementation that constructs a
    /// `DateTime<Utc>` from the result of [`millis()`](Clock::millis).
    /// If the millisecond timestamp is outside chrono's representable range,
    /// the result is clamped to the nearest representable UTC datetime.
    ///
    /// # Returns
    ///
    /// The current time as a `DateTime<Utc>` object.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{Clock, SystemClock};
    ///
    /// let clock = SystemClock::new();
    /// let time = clock.time();
    /// println!("Current time: {}", time);
    /// ```
    #[inline]
    fn time(&self) -> DateTime<Utc> {
        let millis = self.millis();
        DateTime::from_timestamp_millis(millis).unwrap_or_else(|| clamp_out_of_range_millis(millis))
    }
}
