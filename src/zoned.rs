/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Timezone wrapper for clocks.
//!
//! This module provides [`Zoned`], a generic wrapper that adds timezone
//! support to any clock implementation.
//!
//! # Author
//!
//! Haixing Hu

use crate::{Clock, ZonedClock};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::ops::Deref;

/// A wrapper that adds timezone support to any clock.
///
/// `Zoned<C>` wraps any clock implementing [`Clock`] and adds timezone
/// functionality by implementing [`ZonedClock`]. It can convert UTC time to
/// local time in the specified timezone.
///
/// # Deref Behavior
///
/// This type implements [`Deref`] to allow direct access to the wrapped
/// clock's methods. This is particularly useful when wrapping controllable
/// clocks like [`MockClock`](crate::MockClock).
///
/// # Examples
///
/// ```
/// use qubit_clock::{Clock, ZonedClock, SystemClock, Zoned};
/// use chrono_tz::Asia::Shanghai;
///
/// // Wrap a SystemClock with Shanghai timezone
/// let clock = Zoned::new(SystemClock::new(), Shanghai);
/// let local = clock.local_time();
/// println!("Local time in Shanghai: {}", local);
/// ```
///
/// ## Using with MockClock
///
/// ```
/// use qubit_clock::{
///     Clock, ZonedClock, ControllableClock, MockClock, Zoned
/// };
/// use chrono::{DateTime, Utc};
/// use chrono_tz::Asia::Shanghai;
///
/// let mock = MockClock::new();
/// let clock = Zoned::new(mock, Shanghai);
///
/// // Can use ZonedClock methods
/// let local = clock.local_time();
///
/// // Can also use ControllableClock methods via Deref
/// let time = DateTime::parse_from_rfc3339(
///     "2024-01-01T00:00:00Z"
/// ).unwrap().with_timezone(&Utc);
/// clock.set_time(time);
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct Zoned<C: Clock> {
    /// The wrapped clock.
    clock: C,
    /// The timezone for this clock.
    timezone: Tz,
}

impl<C: Clock> Zoned<C> {
    /// Creates a new `Zoned` clock wrapping the given clock with the
    /// specified timezone.
    ///
    /// # Arguments
    ///
    /// * `clock` - The clock to wrap.
    /// * `timezone` - The timezone to use for local time conversions.
    ///
    /// # Returns
    ///
    /// A new `Zoned<C>` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{SystemClock, Zoned};
    /// use chrono_tz::Asia::Shanghai;
    ///
    /// let clock = Zoned::new(SystemClock::new(), Shanghai);
    /// ```
    ///
    pub fn new(clock: C, timezone: Tz) -> Self {
        Zoned { clock, timezone }
    }

    /// Returns a reference to the inner clock.
    ///
    /// # Returns
    ///
    /// A reference to the wrapped clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{Clock, SystemClock, Zoned};
    /// use chrono_tz::Asia::Shanghai;
    ///
    /// let clock = Zoned::new(SystemClock::new(), Shanghai);
    /// let inner = clock.inner();
    /// let millis = inner.millis();
    /// ```
    ///
    pub fn inner(&self) -> &C {
        &self.clock
    }

    /// Consumes the `Zoned` wrapper and returns the inner clock.
    ///
    /// # Returns
    ///
    /// The wrapped clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{SystemClock, Zoned};
    /// use chrono_tz::Asia::Shanghai;
    ///
    /// let clock = Zoned::new(SystemClock::new(), Shanghai);
    /// let inner = clock.into_inner();
    /// ```
    ///
    pub fn into_inner(self) -> C {
        self.clock
    }
}

impl<C: Clock> Clock for Zoned<C> {
    fn millis(&self) -> i64 {
        self.clock.millis()
    }

    fn time(&self) -> DateTime<Utc> {
        self.clock.time()
    }
}

impl<C: Clock> ZonedClock for Zoned<C> {
    fn timezone(&self) -> Tz {
        self.timezone
    }

    fn local_time(&self) -> DateTime<Tz> {
        self.timezone.from_utc_datetime(&self.time().naive_utc())
    }
}

impl<C: Clock> Deref for Zoned<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.clock
    }
}
