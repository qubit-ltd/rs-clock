/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Mock clock implementation for testing.
//!
//! This module provides [`MockClock`], a controllable clock implementation
//! designed for testing scenarios where precise control over time is needed.
//!

use crate::{
    Clock,
    ControllableClock,
    MockClockProgression,
    MonotonicClock,
    SystemClock,
};
use chrono::{
    DateTime,
    Duration,
    Utc,
};
use std::sync::{
    Arc,
    Mutex,
    MutexGuard,
};

/// A controllable clock implementation for testing.
///
/// `MockClock` allows you to adjust logical time, making it useful for testing
/// time-dependent code. Readings are frozen after construction by default.
/// [`set_time()`](ControllableClock::set_time) reanchors the logical time
/// without changing the current progression mode or auto-advance settings.
///
/// # Features
///
/// - Align the logical current time to a specific time
/// - Advance the clock by a duration
/// - Automatically advance time on each call
/// - Switch between frozen and monotonic progression
/// - Reset to the initial creation state
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
#[derive(Debug, Clone)]
pub struct MockClock {
    inner: Arc<Mutex<MockClockInner>>,
}

#[derive(Debug)]
struct MockClockInner {
    /// The frozen time captured when this clock was created.
    initial_time: i64,
    /// The progression mode captured when this clock was created.
    initial_progression: MockClockProgression,
    /// The epoch time to use as the base (milliseconds since epoch).
    epoch: i64,
    /// The monotonic clock used when monotonic progression is enabled.
    monotonic_clock: MonotonicClock,
    /// The monotonic reading corresponding to `epoch`.
    monotonic_base_millis: i64,
    /// The current progression mode.
    progression: MockClockProgression,
    /// Additional milliseconds to add to the current time.
    millis_to_add: i64,
    /// Milliseconds to add on each call to `millis()`.
    millis_to_add_each_time: i64,
    /// Whether to automatically add `millis_to_add_each_time` on each call.
    add_every_time: bool,
}

impl MockClock {
    #[inline]
    fn lock_inner(&self) -> MutexGuard<'_, MockClockInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[inline]
    fn current_millis(inner: &MockClockInner) -> i64 {
        let elapsed = if inner.progression.is_monotonic() {
            inner
                .monotonic_clock
                .millis()
                .saturating_sub(inner.monotonic_base_millis)
        } else {
            0
        };
        inner
            .epoch
            .saturating_add(elapsed)
            .saturating_add(inner.millis_to_add)
    }

    #[inline]
    fn rebase_at_current(inner: &mut MockClockInner) {
        let current = Self::current_millis(inner);
        inner.epoch = current;
        inner.millis_to_add = 0;
        inner.monotonic_base_millis = inner.monotonic_clock.millis();
    }

    /// Creates a new `MockClock`.
    ///
    /// The clock is initialized with the current system time and remains
    /// frozen at that instant until adjusted by the control methods or switched
    /// to monotonic progression.
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
        Self::with_progression(MockClockProgression::Frozen)
    }

    /// Creates a new `MockClock` with the specified progression mode.
    ///
    /// The clock starts at the current system time. In
    /// [`Frozen`](MockClockProgression::Frozen) mode, readings stay fixed until
    /// explicitly advanced. In [`Monotonic`](MockClockProgression::Monotonic)
    /// mode, readings progress naturally from the initial system time.
    ///
    /// # Arguments
    ///
    /// * `progression` - The initial progression mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{MockClock, MockClockProgression};
    ///
    /// let clock = MockClock::with_progression(MockClockProgression::Monotonic);
    /// assert_eq!(clock.progression(), MockClockProgression::Monotonic);
    /// ```
    ///
    pub fn with_progression(progression: MockClockProgression) -> Self {
        let initial_time = SystemClock::new().millis();
        let monotonic_clock = MonotonicClock::new();
        let monotonic_base_millis = monotonic_clock.millis();
        MockClock {
            inner: Arc::new(Mutex::new(MockClockInner {
                initial_time,
                initial_progression: progression,
                epoch: initial_time,
                monotonic_clock,
                monotonic_base_millis,
                progression,
                millis_to_add: 0,
                millis_to_add_each_time: 0,
                add_every_time: false,
            })),
        }
    }

    /// Returns the current progression mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{MockClock, MockClockProgression};
    ///
    /// let clock = MockClock::new();
    /// assert_eq!(clock.progression(), MockClockProgression::Frozen);
    /// ```
    pub fn progression(&self) -> MockClockProgression {
        self.lock_inner().progression
    }

    /// Switches the clock progression mode without changing the current reading.
    ///
    /// The current logical reading is first folded into the clock's base state,
    /// so changing between frozen and monotonic modes does not cause an
    /// immediate time jump.
    ///
    /// # Arguments
    ///
    /// * `progression` - The new progression mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{MockClock, MockClockProgression};
    ///
    /// let clock = MockClock::new();
    /// clock.set_progression(MockClockProgression::Monotonic);
    /// assert_eq!(clock.progression(), MockClockProgression::Monotonic);
    /// ```
    pub fn set_progression(&self, progression: MockClockProgression) {
        let mut inner = self.lock_inner();
        Self::rebase_at_current(&mut inner);
        inner.progression = progression;
    }

    /// Returns whether monotonic progression is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::MockClock;
    ///
    /// let clock = MockClock::new();
    /// assert!(!clock.monotonic_progression_enabled());
    /// ```
    pub fn monotonic_progression_enabled(&self) -> bool {
        self.progression().is_monotonic()
    }

    /// Enables or disables monotonic progression.
    ///
    /// This is a boolean convenience wrapper around
    /// [`set_progression()`](MockClock::set_progression).
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to use monotonic progression, `false` to freeze.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::MockClock;
    ///
    /// let clock = MockClock::new();
    /// clock.set_monotonic_progression_enabled(true);
    /// assert!(clock.monotonic_progression_enabled());
    /// ```
    pub fn set_monotonic_progression_enabled(&self, enabled: bool) {
        let progression = if enabled {
            MockClockProgression::Monotonic
        } else {
            MockClockProgression::Frozen
        };
        self.set_progression(progression);
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
    /// [`time()`](Clock::time) without enabling auto-advance. If the
    /// accumulated offset exceeds the `i64` range, it saturates at the nearest
    /// boundary.
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
        let mut inner = self.lock_inner();
        inner.millis_to_add = inner.millis_to_add.saturating_add(millis);
    }

    /// Enables auto-advance on each read operation.
    ///
    /// After calling this method, each call to [`millis()`](Clock::millis) or
    /// [`time()`](Clock::time) returns the current logical time and advances
    /// the next read by `millis`.
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
        let mut inner = self.lock_inner();
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
    /// assert_eq!(t2, t1);
    /// ```
    pub fn clear_auto_advance(&self) {
        let mut inner = self.lock_inner();
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
        let mut inner = self.lock_inner();
        let result = Self::current_millis(&inner);

        if inner.add_every_time {
            inner.millis_to_add = inner
                .millis_to_add
                .saturating_add(inner.millis_to_add_each_time);
        }

        result
    }
}

impl ControllableClock for MockClock {
    fn set_time(&self, instant: DateTime<Utc>) {
        let mut inner = self.lock_inner();
        inner.epoch = instant.timestamp_millis();
        inner.monotonic_base_millis = inner.monotonic_clock.millis();
        inner.millis_to_add = 0;
    }

    #[inline]
    fn add_duration(&self, duration: Duration) {
        let millis = duration.num_milliseconds();
        self.advance_millis(millis);
    }

    fn reset(&self) {
        let mut inner = self.lock_inner();
        inner.epoch = inner.initial_time;
        inner.progression = inner.initial_progression;
        inner.monotonic_base_millis = inner.monotonic_clock.millis();
        inner.millis_to_add = 0;
        inner.millis_to_add_each_time = 0;
        inner.add_every_time = false;
    }
}
