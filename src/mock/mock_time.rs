// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Facade for a complete mock time runtime.

use std::time::Duration as StdDuration;

use chrono::{
    DateTime,
    Utc,
};

use crate::sleep::MockSleeper;
use crate::{
    ControllableClock,
    MockClock,
    MockTimeError,
    MockTimeline,
};

/// A complete mock time runtime sharing one timeline across clocks and
/// sleepers.
///
/// `MockTime` is the recommended entry point for tests that need both "what
/// time is it?" and "how long should this operation sleep?" to use the same
/// deterministic time source. It constructs one [`MockTimeline`], then creates
/// a [`MockClock`] and [`MockSleeper`](crate::sleep::MockSleeper) over that
/// timeline.
///
/// Advancing the runtime through [`advance`](Self::advance) advances every
/// component derived from it. This avoids a common testing bug where a mock
/// clock reports one logical time while a mock sleeper or monitor waits on a
/// different logical time source.
///
/// Wall-clock anchoring is separate from elapsed mock time. Calling
/// [`set_time`](Self::set_time) changes the UTC value read at the current
/// timeline instant, but it does not advance or rewind the timeline. Calling
/// [`reset`](Self::reset) restores both the elapsed timeline and the clock
/// anchor, and fails if any timeline waiter is active.
///
/// # Example
///
/// ```
/// use std::time::Duration;
///
/// use qubit_clock::{Clock, MockTime};
///
/// let mock = MockTime::unix_epoch();
/// let clock = mock.clock();
///
/// assert_eq!(0, clock.millis());
///
/// mock.advance(Duration::from_millis(250));
///
/// assert_eq!(250, clock.millis());
/// assert_eq!(Duration::from_millis(250), mock.elapsed());
/// ```
#[derive(Debug, Clone)]
pub struct MockTime {
    timeline: MockTimeline,
    clock: MockClock,
    sleeper: MockSleeper,
}

impl MockTime {
    /// Creates a mock runtime anchored at a UTC time.
    ///
    /// # Parameters
    /// - `start`: UTC reading returned before the timeline advances.
    ///
    /// # Returns
    /// A mock runtime with a clock and sleeper sharing one timeline.
    #[must_use]
    pub fn at(start: DateTime<Utc>) -> Self {
        let timeline = MockTimeline::new();
        let clock = MockClock::with_timeline(start, timeline.clone());
        let sleeper = MockSleeper::with_timeline(timeline.clone());
        Self {
            timeline,
            clock,
            sleeper,
        }
    }

    /// Creates a mock runtime anchored at the Unix epoch.
    ///
    /// # Returns
    /// A mock runtime starting at `1970-01-01T00:00:00Z`.
    #[must_use]
    pub fn unix_epoch() -> Self {
        Self::at(DateTime::<Utc>::UNIX_EPOCH)
    }

    /// Returns the shared timeline.
    ///
    /// # Returns
    /// The timeline that drives all components in this runtime.
    #[inline]
    pub fn timeline(&self) -> MockTimeline {
        self.timeline.clone()
    }

    /// Returns a clock view over the shared timeline.
    ///
    /// # Returns
    /// Cloneable mock clock.
    #[inline]
    pub fn clock(&self) -> MockClock {
        self.clock.clone()
    }

    /// Returns a sleeper view over the shared timeline.
    ///
    /// # Returns
    /// Cloneable mock sleeper.
    #[inline]
    pub fn sleeper(&self) -> MockSleeper {
        self.sleeper.clone()
    }

    /// Returns elapsed time on the shared timeline.
    ///
    /// # Returns
    /// Elapsed mock time.
    #[inline]
    pub fn elapsed(&self) -> StdDuration {
        self.timeline.elapsed()
    }

    /// Advances the shared timeline.
    ///
    /// # Parameters
    /// - `duration`: Duration to add to mock elapsed time.
    #[inline]
    pub fn advance(&self, duration: StdDuration) {
        self.timeline.advance(duration);
    }

    /// Reanchors the runtime clock at the current timeline instant.
    ///
    /// # Parameters
    /// - `instant`: New UTC time returned at the current timeline instant.
    #[inline]
    pub fn set_time(&self, instant: DateTime<Utc>) {
        self.clock.set_time(instant);
    }

    /// Resets the timeline and clock anchor to the runtime's initial state.
    ///
    /// # Returns
    /// `Ok(())` when reset succeeds.
    ///
    /// # Errors
    /// Returns [`MockTimeError::ActiveWaiters`] when timeline waiters are
    /// active.
    pub fn reset(&self) -> Result<(), MockTimeError> {
        self.timeline.reset()?;
        self.clock.reset_wall_anchor();
        Ok(())
    }
}
