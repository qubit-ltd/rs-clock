// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Sleeper backed by a shared mock timeline.

use std::time::Duration;

use crate::sleep::Sleeper;
#[cfg(feature = "tokio")]
use crate::sleep::{
    AsyncSleepFuture,
    AsyncSleeper,
};
use crate::{
    MockTimeline,
    MockWaiterKind,
};

/// Relative sleeper driven by a [`crate::MockTimeline`].
///
/// `MockSleeper` is the test implementation of [`Sleeper`]. It does not wait
/// for real wall-clock time. Instead, each sleep captures a deadline on the
/// backing [`crate::MockTimeline`] and completes only after that timeline has
/// advanced to the deadline.
///
/// Cloned sleepers share the same timeline. A sleeper created through
/// [`with_timeline`](Self::with_timeline) can therefore be paired with a
/// [`crate::MockClock`] or future monitor implementation that uses the same
/// timeline. For most tests, [`crate::MockTime`] is the simpler way to create
/// that shared runtime.
///
/// Blocking sleeps register themselves as [`crate::MockWaiterKind::Sleep`] so
/// tests can use
/// [`MockTimeline::wait_for_blocked_waiters`](crate::MockTimeline::wait_for_blocked_waiters)
/// before advancing time. With the `tokio` feature enabled, async sleeps use
/// the same timeline and waiter registration model.
#[derive(Clone, Debug)]
pub struct MockSleeper {
    timeline: MockTimeline,
}

impl MockSleeper {
    /// Creates a mock sleeper with a fresh timeline.
    ///
    /// # Returns
    /// A sleeper backed by a new zero-elapsed timeline.
    #[must_use]
    pub fn new() -> Self {
        Self::with_timeline(MockTimeline::new())
    }

    /// Creates a mock sleeper backed by an existing timeline.
    ///
    /// # Parameters
    /// - `timeline`: Shared mock timeline to wait on.
    ///
    /// # Returns
    /// A sleeper view over the provided timeline.
    #[must_use]
    pub fn with_timeline(timeline: MockTimeline) -> Self {
        Self { timeline }
    }

    /// Returns the timeline backing this sleeper.
    ///
    /// # Returns
    /// The shared mock timeline.
    #[inline]
    pub fn timeline(&self) -> MockTimeline {
        self.timeline.clone()
    }
}

impl Default for MockSleeper {
    /// Creates a mock sleeper with a fresh timeline.
    fn default() -> Self {
        Self::new()
    }
}

impl Sleeper for MockSleeper {
    /// Blocks until the shared mock timeline advances by `duration`.
    fn sleep_for(&self, duration: Duration) {
        let deadline = self.timeline.now().saturating_add(duration);
        self.timeline
            .wait_until_with_kind(deadline, MockWaiterKind::Sleep)
            .expect(
                "mock sleeper deadlines should belong to the sleeper timeline",
            );
    }
}

#[cfg(feature = "tokio")]
impl AsyncSleeper for MockSleeper {
    /// Returns a future that resolves when the shared mock timeline reaches the
    /// sleep deadline.
    fn sleep_for_async<'a>(
        &'a self,
        duration: Duration,
    ) -> AsyncSleepFuture<'a> {
        let deadline = self.timeline.now().saturating_add(duration);
        self.timeline
            .wait_until_async_with_kind(deadline, MockWaiterKind::Sleep)
            .expect(
                "mock sleeper deadlines should belong to the sleeper timeline",
            )
    }
}
