/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

static NEXT_TIMER_DOMAIN_ID: AtomicU64 = AtomicU64::new(1);

/// Identifies the monotonic time axis owned by a timer.
///
/// Timer domain IDs are opaque values. They are exposed so callers can diagnose
/// domain mismatches and verify that cloned timers still refer to the same
/// logical time axis.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TimerDomainId(NonZeroU64);

impl TimerDomainId {
    /// Returns the numeric identifier.
    ///
    /// # Returns
    ///
    /// The opaque non-zero ID assigned to this timer domain.
    pub fn get(self) -> u64 {
        self.0.get()
    }

    /// Creates a fresh timer domain identifier.
    ///
    /// Each call allocates a new ID from a process-wide counter.
    ///
    /// # Returns
    ///
    /// A previously unused [`TimerDomainId`].
    pub(crate) fn new_unique() -> Self {
        let value = NEXT_TIMER_DOMAIN_ID.fetch_add(1, Ordering::Relaxed);
        let value = NonZeroU64::new(value).expect("timer domain counter should never wrap to zero");
        Self(value)
    }
}

impl fmt::Display for TimerDomainId {
    /// Formats the numeric domain identifier.
    ///
    /// # Arguments
    ///
    /// * `formatter` - The destination formatter.
    ///
    /// # Returns
    ///
    /// The result of formatting the underlying numeric ID.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
