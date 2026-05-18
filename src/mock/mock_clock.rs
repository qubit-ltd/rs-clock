/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Clock view backed by a shared mock timeline.

use std::time::Duration as StdDuration;

use chrono::{
    DateTime,
    Duration,
    Utc,
};
use parking_lot::{
    Mutex,
    MutexGuard,
};
use std::sync::Arc;

use crate::{
    Clock,
    ControllableClock,
    MockTimeError,
    MockTimeline,
    NanoClock,
};

/// Number of nanoseconds in one millisecond.
const NANOS_PER_MILLISECOND: i128 = 1_000_000;
/// Number of nanoseconds in one second.
const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// Clock implementation whose readings are derived from a shared mock timeline.
#[derive(Debug, Clone)]
pub struct MockClock {
    timeline: MockTimeline,
    anchor: Arc<Mutex<MockClockAnchor>>,
}

/// Wall-clock mapping state for a mock clock.
#[derive(Debug)]
struct MockClockAnchor {
    initial_wall_origin_nanos: i128,
    wall_origin_nanos: i128,
}

impl MockClock {
    /// Creates a mock clock anchored at the current system time.
    ///
    /// # Returns
    /// A mock clock with a fresh zero-elapsed timeline.
    #[must_use]
    pub fn new() -> Self {
        Self::at(Utc::now())
    }

    /// Creates a mock clock anchored at a specific UTC time.
    ///
    /// # Parameters
    /// - `start`: UTC time returned while the associated timeline is at its
    ///   current elapsed value.
    ///
    /// # Returns
    /// A mock clock with a fresh timeline.
    #[must_use]
    pub fn at(start: DateTime<Utc>) -> Self {
        Self::with_timeline(start, MockTimeline::new())
    }

    /// Creates a mock clock backed by an existing timeline.
    ///
    /// # Parameters
    /// - `start`: UTC time returned for the timeline's current elapsed value.
    /// - `timeline`: Shared mock timeline used by this clock.
    ///
    /// # Returns
    /// A mock clock view over the provided timeline.
    #[must_use]
    pub fn with_timeline(start: DateTime<Utc>, timeline: MockTimeline) -> Self {
        let wall_origin_nanos =
            datetime_to_nanos(start).saturating_sub(timeline_elapsed_i128(&timeline));
        Self {
            timeline,
            anchor: Arc::new(Mutex::new(MockClockAnchor {
                initial_wall_origin_nanos: wall_origin_nanos,
                wall_origin_nanos,
            })),
        }
    }

    /// Returns the timeline backing this clock.
    ///
    /// # Returns
    /// The shared mock timeline.
    #[inline]
    pub fn timeline(&self) -> MockTimeline {
        self.timeline.clone()
    }

    /// Advances the shared mock timeline.
    ///
    /// # Parameters
    /// - `duration`: Duration to add to the shared timeline.
    #[inline]
    pub fn advance(&self, duration: StdDuration) {
        self.timeline.advance(duration);
    }

    /// Resets the shared timeline and this clock's wall-time anchor.
    ///
    /// # Returns
    /// `Ok(())` when reset succeeds.
    ///
    /// # Errors
    /// Returns [`MockTimeError::ActiveWaiters`] when timeline waiters are active.
    pub fn try_reset(&self) -> Result<(), MockTimeError> {
        self.timeline.reset()?;
        self.reset_wall_anchor();
        Ok(())
    }

    /// Resets only the wall-time anchor.
    ///
    /// This is used by [`crate::MockTime`] after it has reset the shared
    /// timeline.
    pub(crate) fn reset_wall_anchor(&self) {
        let mut anchor = self.lock_anchor();
        anchor.wall_origin_nanos = anchor.initial_wall_origin_nanos;
    }

    /// Calculates the current UTC timestamp in nanoseconds.
    ///
    /// # Returns
    /// Current timestamp as Unix nanoseconds.
    fn current_nanos(&self) -> i128 {
        let anchor = self.lock_anchor();
        anchor
            .wall_origin_nanos
            .saturating_add(timeline_elapsed_i128(&self.timeline))
    }

    /// Locks the wall-clock anchor.
    ///
    /// # Returns
    /// A guard for anchor state.
    fn lock_anchor(&self) -> MutexGuard<'_, MockClockAnchor> {
        self.anchor.lock()
    }
}

impl Default for MockClock {
    /// Creates a mock clock anchored at the current system time.
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    /// Returns the current mock UTC timestamp in milliseconds.
    fn millis(&self) -> i64 {
        millis_from_nanos(self.current_nanos())
    }

    /// Returns the current mock UTC time with nanosecond precision.
    fn time(&self) -> DateTime<Utc> {
        datetime_from_nanos(self.current_nanos())
    }
}

impl NanoClock for MockClock {
    /// Returns the current mock UTC timestamp in nanoseconds.
    fn nanos(&self) -> i128 {
        self.current_nanos()
    }
}

impl ControllableClock for MockClock {
    /// Reanchors this clock so the current timeline instant reads as `instant`.
    fn set_time(&self, instant: DateTime<Utc>) {
        let mut anchor = self.lock_anchor();
        anchor.wall_origin_nanos =
            datetime_to_nanos(instant).saturating_sub(timeline_elapsed_i128(&self.timeline));
    }

    /// Advances the shared mock timeline by a non-negative duration.
    ///
    /// # Panics
    /// Panics if `duration` is negative or cannot be represented as
    /// [`std::time::Duration`].
    fn add_duration(&self, duration: Duration) {
        let duration = duration
            .to_std()
            .expect("mock time can only be advanced by a non-negative duration");
        self.timeline.advance(duration);
    }

    /// Resets the shared timeline and this clock's wall-time anchor.
    ///
    /// # Panics
    /// Panics when active timeline waiters prevent reset.
    fn reset(&self) {
        self.try_reset()
            .expect("mock clock reset should not run with active waiters");
    }
}

/// Converts a UTC date-time to Unix nanoseconds.
///
/// # Parameters
/// - `instant`: UTC timestamp to convert.
///
/// # Returns
/// Unix timestamp in nanoseconds.
fn datetime_to_nanos(instant: DateTime<Utc>) -> i128 {
    (instant.timestamp() as i128)
        .saturating_mul(NANOS_PER_SECOND)
        .saturating_add(instant.timestamp_subsec_nanos() as i128)
}

/// Converts Unix nanoseconds to a UTC date-time, clamping out-of-range values.
///
/// # Parameters
/// - `nanos`: Unix timestamp in nanoseconds.
///
/// # Returns
/// UTC date-time represented by the timestamp, clamped to chrono bounds.
fn datetime_from_nanos(nanos: i128) -> DateTime<Utc> {
    let seconds = nanos.div_euclid(NANOS_PER_SECOND);
    let sub_nanos = nanos.rem_euclid(NANOS_PER_SECOND) as u32;
    let seconds = match i64::try_from(seconds) {
        Ok(seconds) => seconds,
        Err(_) if nanos < 0 => return DateTime::<Utc>::MIN_UTC,
        Err(_) => return DateTime::<Utc>::MAX_UTC,
    };
    DateTime::from_timestamp(seconds, sub_nanos).unwrap_or({
        if nanos < 0 {
            DateTime::<Utc>::MIN_UTC
        } else {
            DateTime::<Utc>::MAX_UTC
        }
    })
}

/// Converts Unix nanoseconds to timestamp milliseconds using Euclidean division.
///
/// # Parameters
/// - `nanos`: Unix timestamp in nanoseconds.
///
/// # Returns
/// Timestamp milliseconds, saturated to `i64` bounds.
fn millis_from_nanos(nanos: i128) -> i64 {
    let millis = nanos.div_euclid(NANOS_PER_MILLISECOND);
    match i64::try_from(millis) {
        Ok(value) => value,
        Err(_) if millis < 0 => i64::MIN,
        Err(_) => i64::MAX,
    }
}

/// Reads timeline elapsed nanoseconds as an `i128`, saturating on overflow.
///
/// # Parameters
/// - `timeline`: Timeline to inspect.
///
/// # Returns
/// Elapsed nanoseconds as `i128`.
fn timeline_elapsed_i128(timeline: &MockTimeline) -> i128 {
    i128::try_from(timeline.elapsed_nanos()).unwrap_or(i128::MAX)
}
