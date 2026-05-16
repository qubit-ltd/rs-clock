/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::Duration;

use crate::timer::{
    TimerDomainId,
    TimerError,
};

/// A monotonic instant inside one timer domain.
///
/// The stored duration is relative to the creation point of the timer domain
/// that produced the instant. Values from different domains are intentionally
/// incompatible, even if their elapsed durations happen to be equal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimerInstant {
    domain: TimerDomainId,
    elapsed: Duration,
}

impl TimerInstant {
    /// Creates an instant in the given timer domain.
    pub(crate) fn new(domain: TimerDomainId, elapsed: Duration) -> Self {
        Self { domain, elapsed }
    }

    /// Returns the timer domain that owns this instant.
    pub fn domain(self) -> TimerDomainId {
        self.domain
    }

    /// Returns the elapsed duration since the owning timer domain was created.
    pub fn elapsed_since_timer_start(self) -> Duration {
        self.elapsed
    }

    /// Returns a new instant after adding the duration.
    ///
    /// Returns `None` if the duration overflows [`Duration`].
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        self.elapsed
            .checked_add(duration)
            .map(|elapsed| Self::new(self.domain, elapsed))
    }

    /// Returns a new instant after adding the duration, saturating on overflow.
    pub fn saturating_add(self, duration: Duration) -> Self {
        Self::new(self.domain, self.elapsed.saturating_add(duration))
    }

    /// Returns the duration elapsed since an earlier instant in the same domain.
    ///
    /// Returns `Ok(Some(duration))` when `self` is not earlier than `earlier`,
    /// `Ok(None)` when `earlier` is later than `self`, and `Err` when the two
    /// instants belong to different timer domains.
    pub fn checked_duration_since(self, earlier: Self) -> Result<Option<Duration>, TimerError> {
        self.ensure_domain(earlier.domain)?;
        Ok(self.elapsed.checked_sub(earlier.elapsed))
    }

    /// Returns the duration elapsed since an earlier instant in the same domain.
    ///
    /// The result saturates at zero when `earlier` is later than `self`. Returns
    /// `Err` when the two instants belong to different timer domains.
    pub fn saturating_duration_since(self, earlier: Self) -> Result<Duration, TimerError> {
        self.ensure_domain(earlier.domain)?;
        Ok(self.elapsed.saturating_sub(earlier.elapsed))
    }

    /// Verifies that this instant belongs to the expected timer domain.
    pub(crate) fn ensure_domain(self, expected: TimerDomainId) -> Result<(), TimerError> {
        if self.domain == expected {
            Ok(())
        } else {
            Err(TimerError::timer_domain_mismatch(expected, self.domain))
        }
    }
}
