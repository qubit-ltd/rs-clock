/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Controllable clock trait for testing.
//!
//! This module defines the [`ControllableClock`] trait, which extends
//! [`Clock`] to provide methods for controlling the clock's time. This is
//! primarily useful for testing scenarios.
//!
//! # Author
//!
//! Haixing Hu

use crate::Clock;
use chrono::{DateTime, Duration, Utc};

/// A trait representing a clock that can be controlled.
///
/// This trait extends [`Clock`] to provide methods for manually setting and
/// advancing the clock's time. It's primarily designed for testing scenarios
/// where you need precise control over time.
///
/// # Warning
///
/// This trait should only be used in testing code, not in production code.
///
/// # Examples
///
/// ```
/// use prism3_clock::{Clock, ControllableClock, MockClock};
/// use chrono::{DateTime, Duration, Utc};
///
/// let clock = MockClock::new();
///
/// // Set to a specific time
/// let fixed_time = DateTime::parse_from_rfc3339(
///     "2024-01-01T00:00:00Z"
/// ).unwrap().with_timezone(&Utc);
/// clock.set_time(fixed_time);
///
/// assert_eq!(clock.time(), fixed_time);
///
/// // Advance by 1 hour
/// clock.add_duration(Duration::hours(1));
/// assert_eq!(
///     clock.time(),
///     fixed_time + Duration::hours(1)
/// );
/// ```
///
/// # Author
///
/// Haixing Hu
pub trait ControllableClock: Clock {
    /// Sets the clock to a specific time.
    ///
    /// # Arguments
    ///
    /// * `instant` - The time to set the clock to (UTC).
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::{ControllableClock, MockClock};
    /// use chrono::{DateTime, Utc};
    ///
    /// let clock = MockClock::new();
    /// let time = DateTime::parse_from_rfc3339(
    ///     "2024-01-01T00:00:00Z"
    /// ).unwrap().with_timezone(&Utc);
    ///
    /// clock.set_time(time);
    /// assert_eq!(clock.time(), time);
    /// ```
    fn set_time(&self, instant: DateTime<Utc>);

    /// Advances the clock by the specified duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration to advance the clock by.
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::{Clock, ControllableClock, MockClock};
    /// use chrono::Duration;
    ///
    /// let clock = MockClock::new();
    /// let before = clock.time();
    ///
    /// clock.add_duration(Duration::hours(1));
    ///
    /// let after = clock.time();
    /// assert_eq!(after - before, Duration::hours(1));
    /// ```
    fn add_duration(&self, duration: Duration);

    /// Resets the clock to its initial state.
    ///
    /// The exact behavior of this method depends on the implementation. For
    /// [`MockClock`](crate::MockClock), it resets to the time when the clock
    /// was created.
    ///
    /// # Examples
    ///
    /// ```
    /// use prism3_clock::{Clock, ControllableClock, MockClock};
    /// use chrono::Duration;
    ///
    /// let clock = MockClock::new();
    /// let initial = clock.time();
    ///
    /// clock.add_duration(Duration::hours(1));
    /// clock.reset();
    ///
    /// // After reset, time should be close to initial time
    /// let diff = (clock.time() - initial).num_milliseconds().abs();
    /// assert!(diff < 100); // Allow small difference
    /// ```
    fn reset(&self);
}
