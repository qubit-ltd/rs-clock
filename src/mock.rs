/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Mock clock implementation for testing.
//!
//! This module provides [`MockClock`], a controllable clock implementation
//! designed for testing scenarios where precise control over time is needed.
//!
//! # Author
//!
//! Haixing Hu

use crate::{Clock, ControllableClock, MonotonicClock};
use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;
use std::sync::Arc;

/// A controllable clock implementation for testing.
///
/// `MockClock` allows you to manually control the passage of time, making it
/// ideal for testing time-dependent code. It uses [`MonotonicClock`] as its
/// internal time base to ensure stability during tests.
///
/// # Features
///
/// - Set the clock to a specific time
/// - Advance the clock by a duration
/// - Automatically advance time on each call
/// - Reset to initial state
///
/// # Thread Safety
///
/// This type is thread-safe, using `Arc<Mutex<>>` internally to protect its
/// mutable state.
///
/// # Examples
///
/// ```
/// use qubit_clock::{Clock, ControllableClock, MockClock};
/// use chrono::{DateTime, Duration, Utc};
///
/// let clock = MockClock::new();
///
/// // Set to a specific time
/// let fixed_time = DateTime::parse_from_rfc3339(
///     "2024-01-01T00:00:00Z"
/// ).unwrap().with_timezone(&Utc);
/// clock.set_time(fixed_time);
/// assert_eq!(clock.time(), fixed_time);
///
/// // Advance by 1 hour
/// clock.add_duration(Duration::hours(1));
/// assert_eq!(clock.time(), fixed_time + Duration::hours(1));
///
/// // Reset to initial state
/// clock.reset();
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct MockClock {
    inner: Arc<Mutex<MockClockInner>>,
}

#[derive(Debug)]
struct MockClockInner {
    /// The monotonic clock used as the time base.
    monotonic_clock: MonotonicClock,
    /// The time when this clock was created (milliseconds since epoch).
    create_time: i64,
    /// The epoch time to use as the base (milliseconds since epoch).
    epoch: i64,
    /// Additional milliseconds to add to the current time.
    millis_to_add: i64,
    /// Milliseconds to add on each call to `millis()`.
    millis_to_add_each_time: i64,
    /// Whether to automatically add `millis_to_add_each_time` on each call.
    add_every_time: bool,
}

impl MockClock {
    /// Creates a new `MockClock`.
    ///
    /// The clock is initialized with the current system time and uses a
    /// [`MonotonicClock`] as its internal time base.
    ///
    /// # Returns
    ///
    /// A new `MockClock` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::MockClock;
    ///
    /// let clock = MockClock::new();
    /// ```
    ///
    pub fn new() -> Self {
        let monotonic_clock = MonotonicClock::new();
        let create_time = monotonic_clock.millis();
        MockClock {
            inner: Arc::new(Mutex::new(MockClockInner {
                monotonic_clock,
                create_time,
                epoch: create_time,
                millis_to_add: 0,
                millis_to_add_each_time: 0,
                add_every_time: false,
            })),
        }
    }

    /// Adds a fixed amount of milliseconds to the clock.
    ///
    /// # Arguments
    ///
    /// * `millis` - The number of milliseconds to add.
    /// * `add_every_time` - If `true`, the specified milliseconds will be
    ///   added on every call to [`millis()`](Clock::millis). If `false`, the
    ///   milliseconds are added only once.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{Clock, MockClock};
    ///
    /// let clock = MockClock::new();
    /// let before = clock.millis();
    ///
    /// // Add 1000ms once
    /// clock.add_millis(1000, false);
    /// assert_eq!(clock.millis(), before + 1000);
    ///
    /// // Add 100ms on every call
    /// clock.add_millis(100, true);
    /// let t1 = clock.millis();
    /// let t2 = clock.millis();
    /// assert_eq!(t2 - t1, 100);
    /// ```
    ///
    pub fn add_millis(&self, millis: i64, add_every_time: bool) {
        if add_every_time {
            self.set_auto_advance_millis(millis);
        } else {
            self.advance_millis(millis);
        }
    }

    /// Advances the clock by a fixed amount once.
    ///
    /// This method updates the offset used by [`millis()`](Clock::millis) and
    /// [`time()`](Clock::time) without enabling auto-advance.
    ///
    /// # Arguments
    ///
    /// * `millis` - The milliseconds to add once.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{Clock, MockClock};
    ///
    /// let clock = MockClock::new();
    /// let before = clock.millis();
    /// clock.advance_millis(1000);
    /// assert_eq!(clock.millis(), before + 1000);
    /// ```
    pub fn advance_millis(&self, millis: i64) {
        let mut inner = self.inner.lock();
        inner.millis_to_add += millis;
    }

    /// Enables auto-advance on each read operation.
    ///
    /// After calling this method, each call to [`millis()`](Clock::millis) or
    /// [`time()`](Clock::time) will advance the clock by `millis`.
    ///
    /// # Arguments
    ///
    /// * `millis` - The milliseconds to advance on each read.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{Clock, MockClock};
    ///
    /// let clock = MockClock::new();
    /// clock.set_auto_advance_millis(100);
    /// let t1 = clock.millis();
    /// let t2 = clock.millis();
    /// assert_eq!(t2 - t1, 100);
    /// ```
    pub fn set_auto_advance_millis(&self, millis: i64) {
        let mut inner = self.inner.lock();
        inner.millis_to_add_each_time = millis;
        inner.add_every_time = true;
    }

    /// Disables auto-advance behavior.
    ///
    /// This method clears the per-read advance setting. Subsequent read
    /// operations will no longer mutate the clock state.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{Clock, MockClock};
    ///
    /// let clock = MockClock::new();
    /// clock.set_auto_advance_millis(100);
    /// let _ = clock.millis();
    /// clock.clear_auto_advance();
    /// let t1 = clock.millis();
    /// let t2 = clock.millis();
    /// assert!((t2 - t1).abs() < 10);
    /// ```
    pub fn clear_auto_advance(&self) {
        let mut inner = self.inner.lock();
        inner.millis_to_add_each_time = 0;
        inner.add_every_time = false;
    }
}

impl Default for MockClock {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn millis(&self) -> i64 {
        let mut inner = self.inner.lock();
        let elapsed = inner.monotonic_clock.millis() - inner.create_time;
        let result = inner.epoch + elapsed + inner.millis_to_add;

        if inner.add_every_time {
            inner.millis_to_add += inner.millis_to_add_each_time;
        }

        result
    }
}

impl ControllableClock for MockClock {
    fn set_time(&self, instant: DateTime<Utc>) {
        let mut inner = self.inner.lock();
        let current_monotonic = inner.monotonic_clock.millis();
        let elapsed = current_monotonic - inner.create_time;
        inner.epoch = instant.timestamp_millis() - elapsed;
        inner.millis_to_add = 0;
        inner.millis_to_add_each_time = 0;
        inner.add_every_time = false;
    }

    #[inline]
    fn add_duration(&self, duration: Duration) {
        let millis = duration.num_milliseconds();
        self.advance_millis(millis);
    }

    fn reset(&self) {
        let mut inner = self.inner.lock();
        inner.epoch = inner.create_time;
        inner.millis_to_add = 0;
        inner.millis_to_add_each_time = 0;
        inner.add_every_time = false;
    }
}
