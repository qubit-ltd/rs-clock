/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::time::Duration;

use crate::timer::{
    TimerInstant,
    TimerResult,
};

static NEXT_TIMER_DOMAIN_ID: AtomicU64 = AtomicU64::new(0);

/// Defines a monotonic timer domain.
///
/// A timer domain owns a monotonic time axis. [`TimerInstant`] values are only
/// meaningful inside the domain that created them, and implementations reject
/// foreign instants instead of comparing unrelated elapsed durations.
pub trait TimerDomain: Send + Sync {
    /// Returns the timer domain ID owned by this timer domain.
    ///
    /// # Returns
    ///
    /// The opaque numeric ID of the monotonic axis used by this timer domain.
    /// Clones of the same concrete timer share this ID.
    fn id(&self) -> u64;

    /// Returns the current instant in this timer domain.
    ///
    /// # Returns
    ///
    /// A [`TimerInstant`] representing the elapsed time since this timer domain
    /// was created.
    fn now(&self) -> TimerInstant;

    /// Creates a deadline after the specified relative duration.
    ///
    /// The duration is measured from the instant returned by [`now`](Self::now)
    /// at the time of the call. Overflow saturates to [`Duration::MAX`] via
    /// [`TimerInstant::saturating_add`].
    ///
    /// # Arguments
    ///
    /// * `duration` - The offset from the current instant.
    ///
    /// # Returns
    ///
    /// A [`TimerInstant`] at or after the current instant.
    fn deadline_after(&self, duration: Duration) -> TimerInstant {
        self.now().saturating_add(duration)
    }

    /// Returns the remaining duration until a deadline in this timer domain.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant, which must belong to this timer domain.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(duration))` when the deadline is still in the future.
    /// * `Ok(None)` when the deadline has already been reached or passed.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` was created by
    /// a different timer domain.
    fn duration_until(&self, deadline: TimerInstant) -> TimerResult<Option<Duration>> {
        deadline.ensure_domain_id(self.id())?;
        deadline.checked_duration_since(self.now())
    }
}

/// Creates a fresh timer domain ID.
///
/// # Returns
///
/// A process-local numeric ID that has not previously been allocated by this
/// crate instance.
pub(crate) fn next_timer_domain_id() -> u64 {
    NEXT_TIMER_DOMAIN_ID.fetch_add(1, Ordering::Relaxed)
}
