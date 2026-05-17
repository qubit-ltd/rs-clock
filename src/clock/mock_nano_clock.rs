/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Nanosecond-precision mock clock implementation for testing.
//!
//! This module provides [`MockNanoClock`], a controllable clock implementation
//! for tests that need deterministic nanosecond timestamps.
//!

use crate::{Clock, ControllableClock, MockClockProgression, NanoClock, NanoMonotonicClock};
use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, Mutex, MutexGuard};

const NANOS_PER_MILLISECOND: i128 = 1_000_000;
const NANOS_PER_SECOND: i128 = 1_000_000_000;

#[inline]
fn datetime_to_nanos(instant: DateTime<Utc>) -> i128 {
    (instant.timestamp() as i128)
        .saturating_mul(NANOS_PER_SECOND)
        .saturating_add(instant.timestamp_subsec_nanos() as i128)
}

#[inline]
fn duration_to_nanos(duration: Duration) -> i128 {
    duration.num_nanoseconds().map(i128::from).unwrap_or({
        if duration < Duration::zero() {
            i128::MIN
        } else {
            i128::MAX
        }
    })
}

#[inline]
fn millis_from_nanos(nanos: i128) -> i64 {
    let millis = nanos.div_euclid(NANOS_PER_MILLISECOND);
    match i64::try_from(millis) {
        Ok(value) => value,
        Err(_) if millis < 0 => i64::MIN,
        Err(_) => i64::MAX,
    }
}

/// A nanosecond-precision controllable clock implementation for testing.
///
/// `MockNanoClock` is the high-precision counterpart of
/// [`MockClock`](crate::MockClock). It implements [`Clock`], [`NanoClock`],
/// and [`ControllableClock`]. Readings are frozen after construction by
/// default. [`set_time()`](ControllableClock::set_time) reanchors the logical
/// time without changing the current progression mode or auto-advance settings.
///
/// # Features
///
/// - Align the logical current time to a specific time
/// - Advance the clock by a chrono duration or raw nanoseconds
/// - Automatically advance nanoseconds on each call
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
/// use chrono::{DateTime, Duration, Utc};
/// use qubit_clock::{ControllableClock, MockNanoClock, NanoClock};
///
/// let clock = MockNanoClock::new();
/// let fixed_time = DateTime::parse_from_rfc3339(
///     "2024-01-01T00:00:00.000000123Z"
/// ).unwrap().with_timezone(&Utc);
///
/// clock.set_time(fixed_time);
/// assert_eq!(clock.time_precise(), fixed_time);
///
/// clock.advance_nanos(1);
/// assert_eq!(
///     clock.time_precise(),
///     fixed_time + Duration::nanoseconds(1)
/// );
/// ```
#[derive(Debug, Clone)]
pub struct MockNanoClock {
    inner: Arc<Mutex<MockNanoClockInner>>,
}

#[derive(Debug)]
struct MockNanoClockInner {
    /// The frozen nanosecond timestamp captured when this clock was created.
    initial_nanos: i128,
    /// The progression mode captured when this clock was created.
    initial_progression: MockClockProgression,
    /// The epoch nanosecond timestamp to use as the base.
    epoch_nanos: i128,
    /// The monotonic clock used when monotonic progression is enabled.
    monotonic_clock: NanoMonotonicClock,
    /// The monotonic reading corresponding to `epoch_nanos`.
    monotonic_base_nanos: i128,
    /// The current progression mode.
    progression: MockClockProgression,
    /// Additional nanoseconds to add to the current time.
    nanos_to_add: i128,
    /// Nanoseconds to add after each read.
    nanos_to_add_each_time: i128,
    /// Whether to automatically add `nanos_to_add_each_time` on each call.
    add_every_time: bool,
}

impl MockNanoClock {
    #[inline]
    fn lock_inner(&self) -> MutexGuard<'_, MockNanoClockInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[inline]
    fn current_nanos(inner: &MockNanoClockInner) -> i128 {
        let elapsed = if inner.progression.is_monotonic() {
            inner
                .monotonic_clock
                .monotonic_nanos()
                .saturating_sub(inner.monotonic_base_nanos)
        } else {
            0
        };
        inner
            .epoch_nanos
            .saturating_add(elapsed)
            .saturating_add(inner.nanos_to_add)
    }

    #[inline]
    fn rebase_at_current(inner: &mut MockNanoClockInner) {
        let current = Self::current_nanos(inner);
        inner.epoch_nanos = current;
        inner.nanos_to_add = 0;
        inner.monotonic_base_nanos = inner.monotonic_clock.monotonic_nanos();
    }

    /// Creates a new `MockNanoClock`.
    ///
    /// The clock is initialized with the current system time at nanosecond
    /// precision and remains frozen at that instant until adjusted by the
    /// control methods or switched to monotonic progression.
    ///
    /// # Returns
    ///
    /// A new `MockNanoClock` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::MockNanoClock;
    ///
    /// let clock = MockNanoClock::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self::with_progression(MockClockProgression::Frozen)
    }

    /// Creates a new `MockNanoClock` with the specified progression mode.
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
    /// use qubit_clock::{MockClockProgression, MockNanoClock};
    ///
    /// let clock = MockNanoClock::with_progression(MockClockProgression::Monotonic);
    /// assert_eq!(clock.progression(), MockClockProgression::Monotonic);
    /// ```
    #[inline]
    pub fn with_progression(progression: MockClockProgression) -> Self {
        let initial_nanos = datetime_to_nanos(Utc::now());
        let monotonic_clock = NanoMonotonicClock::new();
        let monotonic_base_nanos = monotonic_clock.monotonic_nanos();
        Self {
            inner: Arc::new(Mutex::new(MockNanoClockInner {
                initial_nanos,
                initial_progression: progression,
                epoch_nanos: initial_nanos,
                monotonic_clock,
                monotonic_base_nanos,
                progression,
                nanos_to_add: 0,
                nanos_to_add_each_time: 0,
                add_every_time: false,
            })),
        }
    }

    /// Returns the current progression mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{MockClockProgression, MockNanoClock};
    ///
    /// let clock = MockNanoClock::new();
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
    /// use qubit_clock::{MockClockProgression, MockNanoClock};
    ///
    /// let clock = MockNanoClock::new();
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
    /// use qubit_clock::MockNanoClock;
    ///
    /// let clock = MockNanoClock::new();
    /// assert!(!clock.monotonic_progression_enabled());
    /// ```
    pub fn monotonic_progression_enabled(&self) -> bool {
        self.progression().is_monotonic()
    }

    /// Enables or disables monotonic progression.
    ///
    /// This is a boolean convenience wrapper around
    /// [`set_progression()`](MockNanoClock::set_progression).
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to use monotonic progression, `false` to freeze.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::MockNanoClock;
    ///
    /// let clock = MockNanoClock::new();
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

    /// Adds a fixed amount of nanoseconds to the clock.
    ///
    /// # Arguments
    ///
    /// * `nanos` - The number of nanoseconds to add.
    /// * `add_every_time` - If `true`, the specified nanoseconds will be
    ///   added after every call to [`nanos()`](NanoClock::nanos). If `false`,
    ///   the nanoseconds are added only once.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{MockNanoClock, NanoClock};
    ///
    /// let clock = MockNanoClock::new();
    /// let before = clock.nanos();
    ///
    /// clock.add_nanos(1_000, false);
    /// assert_eq!(clock.nanos(), before + 1_000);
    ///
    /// clock.add_nanos(100, true);
    /// let t1 = clock.nanos();
    /// let t2 = clock.nanos();
    /// assert_eq!(t2 - t1, 100);
    /// ```
    pub fn add_nanos(&self, nanos: i128, add_every_time: bool) {
        if add_every_time {
            self.set_auto_advance_nanos(nanos);
        } else {
            self.advance_nanos(nanos);
        }
    }

    /// Advances the clock by a fixed nanosecond amount once.
    ///
    /// This method updates the offset used by [`nanos()`](NanoClock::nanos)
    /// without enabling auto-advance. If the accumulated offset exceeds the
    /// `i128` range, it saturates at the nearest boundary.
    ///
    /// # Arguments
    ///
    /// * `nanos` - The nanoseconds to add once.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{MockNanoClock, NanoClock};
    ///
    /// let clock = MockNanoClock::new();
    /// let before = clock.nanos();
    /// clock.advance_nanos(1_000);
    /// assert_eq!(clock.nanos(), before + 1_000);
    /// ```
    pub fn advance_nanos(&self, nanos: i128) {
        let mut inner = self.lock_inner();
        inner.nanos_to_add = inner.nanos_to_add.saturating_add(nanos);
    }

    /// Enables auto-advance after each read operation.
    ///
    /// After calling this method, each call to [`nanos()`](NanoClock::nanos),
    /// [`millis()`](Clock::millis), or [`time_precise()`](NanoClock::time_precise)
    /// returns the current logical time and advances the next read by `nanos`.
    ///
    /// # Arguments
    ///
    /// * `nanos` - The nanoseconds to advance on each read.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::{MockNanoClock, NanoClock};
    ///
    /// let clock = MockNanoClock::new();
    /// clock.set_auto_advance_nanos(100);
    /// let t1 = clock.nanos();
    /// let t2 = clock.nanos();
    /// assert_eq!(t2 - t1, 100);
    /// ```
    pub fn set_auto_advance_nanos(&self, nanos: i128) {
        let mut inner = self.lock_inner();
        inner.nanos_to_add_each_time = nanos;
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
    /// use qubit_clock::{MockNanoClock, NanoClock};
    ///
    /// let clock = MockNanoClock::new();
    /// clock.set_auto_advance_nanos(100);
    /// let _ = clock.nanos();
    /// clock.clear_auto_advance();
    /// let t1 = clock.nanos();
    /// let t2 = clock.nanos();
    /// assert_eq!(t2, t1);
    /// ```
    pub fn clear_auto_advance(&self) {
        let mut inner = self.lock_inner();
        inner.nanos_to_add_each_time = 0;
        inner.add_every_time = false;
    }
}

impl Default for MockNanoClock {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockNanoClock {
    #[inline]
    fn millis(&self) -> i64 {
        millis_from_nanos(self.nanos())
    }
}

impl NanoClock for MockNanoClock {
    fn nanos(&self) -> i128 {
        let mut inner = self.lock_inner();
        let result = Self::current_nanos(&inner);

        if inner.add_every_time {
            inner.nanos_to_add = inner
                .nanos_to_add
                .saturating_add(inner.nanos_to_add_each_time);
        }

        result
    }
}

impl ControllableClock for MockNanoClock {
    fn set_time(&self, instant: DateTime<Utc>) {
        let mut inner = self.lock_inner();
        inner.epoch_nanos = datetime_to_nanos(instant);
        inner.monotonic_base_nanos = inner.monotonic_clock.monotonic_nanos();
        inner.nanos_to_add = 0;
    }

    #[inline]
    fn add_duration(&self, duration: Duration) {
        self.advance_nanos(duration_to_nanos(duration));
    }

    fn reset(&self) {
        let mut inner = self.lock_inner();
        inner.epoch_nanos = inner.initial_nanos;
        inner.progression = inner.initial_progression;
        inner.monotonic_base_nanos = inner.monotonic_clock.monotonic_nanos();
        inner.nanos_to_add = 0;
        inner.nanos_to_add_each_time = 0;
        inner.add_every_time = false;
    }
}
