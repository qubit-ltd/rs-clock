/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Instant value on a mock timeline.

use std::cmp::Ordering;
use std::time::Duration;

/// A monotonic instant measured from a [`crate::MockTimeline`] origin.
///
/// `MockInstant` is the deadline value used by [`crate::MockTimeline`]. It is
/// intentionally monotonic and relative: it stores nanoseconds since a timeline
/// origin, not a UTC timestamp. It also carries the id of the timeline that
/// produced it through [`crate::MockTimeline::now`].
///
/// Instants can be ordered and can be advanced with
/// [`saturating_add`](Self::saturating_add), which is useful for computing
/// relative deadlines. Ordering is defined only for instants from the same
/// timeline. Passing an instant from one timeline to another timeline's wait
/// API is rejected with [`crate::MockTimeError::MismatchedTimeline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MockInstant {
    timeline_id: u64,
    nanos_since_origin: u128,
}

impl MockInstant {
    /// Creates an instant from nanoseconds since the mock timeline origin.
    ///
    /// # Parameters
    /// - `timeline_id`: Timeline that produced the instant.
    /// - `nanos_since_origin`: Monotonic nanoseconds from the timeline origin.
    ///
    /// # Returns
    /// A mock instant representing that offset.
    pub(crate) const fn from_nanos_since_origin(
        timeline_id: u64,
        nanos_since_origin: u128,
    ) -> Self {
        Self {
            timeline_id,
            nanos_since_origin,
        }
    }

    /// Returns the id of the timeline that produced this instant.
    ///
    /// # Returns
    /// The source timeline id.
    #[inline]
    pub const fn timeline_id(&self) -> u64 {
        self.timeline_id
    }

    /// Returns the instant offset from the timeline origin in nanoseconds.
    ///
    /// # Returns
    /// Nanoseconds since the mock timeline origin.
    #[inline]
    pub const fn nanos_since_origin(&self) -> u128 {
        self.nanos_since_origin
    }

    /// Adds a relative duration, saturating at `u128::MAX`.
    ///
    /// # Parameters
    /// - `duration`: Relative duration to add.
    ///
    /// # Returns
    /// The advanced mock instant.
    #[inline]
    pub fn saturating_add(self, duration: Duration) -> Self {
        Self {
            timeline_id: self.timeline_id,
            nanos_since_origin: self.nanos_since_origin.saturating_add(duration.as_nanos()),
        }
    }
}

impl PartialOrd for MockInstant {
    /// Compares two instants only when they belong to the same timeline.
    ///
    /// # Returns
    /// `Some(ordering)` for instants from the same timeline, or `None` for
    /// instants from different timelines.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.timeline_id == other.timeline_id {
            Some(self.nanos_since_origin.cmp(&other.nanos_since_origin))
        } else {
            None
        }
    }
}
