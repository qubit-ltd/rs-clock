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

use std::time::Duration;

/// A monotonic instant measured from a [`crate::MockTimeline`] origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MockInstant {
    nanos_since_origin: u128,
}

impl MockInstant {
    /// Creates an instant from nanoseconds since the mock timeline origin.
    ///
    /// # Parameters
    /// - `nanos_since_origin`: Monotonic nanoseconds from the timeline origin.
    ///
    /// # Returns
    /// A mock instant representing that offset.
    pub(crate) const fn from_nanos_since_origin(nanos_since_origin: u128) -> Self {
        Self { nanos_since_origin }
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
            nanos_since_origin: self.nanos_since_origin.saturating_add(duration.as_nanos()),
        }
    }
}
