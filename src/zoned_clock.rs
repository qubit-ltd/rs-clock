/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Clock trait with timezone support.
//!
//! This module defines the [`ZonedClock`] trait, which extends [`Clock`] to
//! provide timezone support and local time access.
//!
//! # Author
//!
//! Haixing Hu

use crate::Clock;
use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;

/// A trait representing a clock with timezone support.
///
/// This trait extends [`Clock`] to provide timezone information and methods
/// to get the current local time in the clock's timezone.
///
/// # Examples
///
/// ```
/// use qubit_clock::{Clock, ZonedClock, SystemClock, Zoned};
/// use chrono_tz::Asia::Shanghai;
///
/// let clock = Zoned::new(SystemClock::new(), Shanghai);
/// let local = clock.local_time();
/// println!("Local time in Shanghai: {}", local);
/// ```
///
/// # Author
///
/// Haixing Hu
pub trait ZonedClock: Clock {
    /// Returns the timezone of this clock.
    ///
    /// # Returns
    ///
    /// The timezone associated with this clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{ZonedClock, SystemClock, Zoned};
    /// use chrono_tz::Asia::Shanghai;
    ///
    /// let clock = Zoned::new(SystemClock::new(), Shanghai);
    /// assert_eq!(clock.timezone(), Shanghai);
    /// ```
    fn timezone(&self) -> Tz;

    /// Returns the current local time in this clock's timezone.
    ///
    /// This method has a default implementation that converts the UTC time
    /// from [`time()`](Clock::time) to the local time using the clock's
    /// timezone.
    ///
    /// # Returns
    ///
    /// The current local time as a `DateTime<Tz>` object.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{ZonedClock, SystemClock, Zoned};
    /// use chrono_tz::Asia::Shanghai;
    ///
    /// let clock = Zoned::new(SystemClock::new(), Shanghai);
    /// let local = clock.local_time();
    /// println!("Local time: {}", local);
    /// ```
    fn local_time(&self) -> DateTime<Tz> {
        self.timezone().from_utc_datetime(&self.time().naive_utc())
    }
}
