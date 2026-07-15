// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines a manually re-anchorable wall-clock projection.

use crate::{ManualMonotonicClock, MonotonicClock, MonotonicInstant, WallClock};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;

/// A wall clock projected from a shared [`ManualMonotonicClock`].
///
/// Advancing the monotonic clock advances this wall clock by the same
/// duration. Calling [`reanchor()`](Self::reanchor) changes only the wall-time
/// mapping and never changes the underlying monotonic clock.
#[derive(Debug)]
pub struct ManualWallClock {
    clock: Arc<ManualMonotonicClock>,
    anchor: Mutex<(SystemTime, MonotonicInstant)>,
}

impl ManualWallClock {
    /// Creates a wall clock whose current reading is `wall_time`.
    ///
    /// Future readings advance according to the explicitly shared `clock`.
    #[must_use]
    pub fn from_clock(wall_time: SystemTime, clock: Arc<ManualMonotonicClock>) -> Self {
        let monotonic_anchor = clock.now();
        Self {
            clock,
            anchor: Mutex::new((wall_time, monotonic_anchor)),
        }
    }

    /// Reassigns the current monotonic instant to `wall_time`.
    ///
    /// This operation may move wall time forward or backward. It does not
    /// advance the monotonic clock and does not wake monotonic sleepers. The
    /// anchor mutex remains held while the monotonic clock is sampled, so
    /// concurrent calls to [`now()`](WallClock::now) observe either the old or
    /// the new mapping without combining both snapshots.
    pub fn reanchor(&self, wall_time: SystemTime) {
        let mut anchor = self.lock_anchor();
        let monotonic_anchor = self.clock.now();
        *anchor = (wall_time, monotonic_anchor);
    }

    /// Locks the wall and monotonic anchor pair, recovering after poisoning.
    fn lock_anchor(&self) -> MutexGuard<'_, (SystemTime, MonotonicInstant)> {
        self.anchor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl WallClock for ManualWallClock {
    /// Returns wall time derived from the anchor and current monotonic time.
    ///
    /// # Panics
    ///
    /// Panics if the manually advanced duration cannot be represented by
    /// [`SystemTime`]. Normal application and test durations are representable.
    fn now(&self) -> SystemTime {
        let anchor = self.lock_anchor();
        let (wall_anchor, monotonic_anchor) = *anchor;
        let monotonic_now = self.clock.now();
        drop(anchor);
        let elapsed = monotonic_now
            .duration_since(monotonic_anchor)
            .expect("manual wall clock must retain its monotonic clock domain");
        wall_anchor
            .checked_add(elapsed)
            .expect("manual wall time exceeded SystemTime range")
    }
}
