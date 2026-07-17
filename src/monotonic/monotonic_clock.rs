// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the monotonic clock capability.
//!
//! The [`MonotonicClock`] trait is the injectable source of
//! [`MonotonicInstant`] values used for timeouts, deadlines, and elapsed-time
//! measurements. Wall time belongs on [`WallClock`](crate::WallClock) instead.

use crate::{
    MonotonicInstant,
    Timer,
};
use std::sync::Arc;

/// Provides the current instant in a stable, non-decreasing clock domain.
///
/// Implementations return [`MonotonicInstant`] values that never move backward
/// within one domain. Prefer this trait for timeouts, deadlines, and measuring
/// elapsed durations. Use [`WallClock`](crate::WallClock) for civil timestamps
/// that must align with an external calendar clock, which may jump forward or
/// backward after a system adjustment.
///
/// The crate ships several implementations:
///
/// - [`StdMonotonicClock`](crate::StdMonotonicClock) — production clock backed
///   by [`std::time::Instant`]
/// - [`ManualMonotonicClock`](crate::ManualMonotonicClock) — explicitly
///   advanced clock for deterministic tests
/// - `TokioMonotonicClock` — Tokio time-driver clock (requires the `tokio`
///   feature)
///
/// `Arc<T>` and `Box<T>` implement this trait when `T: MonotonicClock +
/// ?Sized`, so a shared or owned trait object needs no extra adapter.
///
/// # Examples
///
/// Sample a standard monotonic clock and form a deadline from its instant:
///
/// ```
/// use qubit_clock::{MonotonicClock, StdMonotonicClock};
/// use std::time::Duration;
///
/// let clock = StdMonotonicClock::new();
/// let start = clock.now();
/// let deadline = start
///     .checked_add(Duration::from_millis(10))
///     .expect("duration should fit");
/// assert!(deadline.elapsed_since_origin() > start.elapsed_since_origin());
/// ```
///
/// Drive logical time with a manual clock in tests without waiting for real
/// time:
///
/// ```
/// use qubit_clock::{ManualMonotonicClock, MonotonicClock};
/// use std::time::Duration;
///
/// let clock = ManualMonotonicClock::new();
/// assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
///
/// clock
///     .advance(Duration::from_secs(2))
///     .expect("manual time should advance");
/// assert_eq!(Duration::from_secs(2), clock.now().elapsed_since_origin());
/// ```
///
/// Share one clock through a trait object:
///
/// ```
/// use qubit_clock::{ManualMonotonicClock, MonotonicClock};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// let clock: Arc<dyn MonotonicClock> = Arc::new(ManualMonotonicClock::new());
/// let first = clock.now();
/// let second = clock.now();
/// assert_eq!(first.domain(), second.domain());
/// assert_eq!(Duration::ZERO, second.elapsed_since_origin());
/// ```
pub trait MonotonicClock: Send + Sync {
    /// Returns the current instant in this clock's domain.
    ///
    /// Successive calls on the same clock never return an earlier instant.
    /// Instants from different clock instances belong to different
    /// [`ClockDomain`](crate::ClockDomain)s and must not be mixed.
    ///
    /// # Returns
    ///
    /// The current domain-scoped monotonic instant.
    fn now(&self) -> MonotonicInstant;

    /// Creates a timer in this clock's exact monotonic domain.
    ///
    /// The call borrows rather than consumes the clock. The returned timer
    /// retains an independent same-domain handle, so callers do not need to
    /// clone an `Arc` before invoking this method and may continue using or
    /// drop the original clock afterward.
    ///
    /// # Returns
    ///
    /// A shared timer whose [`Timer::clock`] reports this clock's domain.
    #[must_use = "the timer should be retained to register deadlines"]
    fn new_timer(&self) -> Arc<dyn Timer>;
}

impl<T> MonotonicClock for std::sync::Arc<T>
where
    T: MonotonicClock + ?Sized,
{
    /// Delegates the current instant to the shared clock object.
    ///
    /// # Returns
    ///
    /// The current instant returned by the wrapped clock.
    #[inline(always)]
    fn now(&self) -> MonotonicInstant {
        self.as_ref().now()
    }

    /// Delegates timer creation without consuming the shared clock pointer.
    ///
    /// # Returns
    ///
    /// A timer in the wrapped clock's exact monotonic domain.
    #[inline(always)]
    fn new_timer(&self) -> Arc<dyn Timer> {
        self.as_ref().new_timer()
    }
}

impl<T> MonotonicClock for Box<T>
where
    T: MonotonicClock + ?Sized,
{
    /// Delegates the current instant to the boxed clock object.
    ///
    /// # Returns
    ///
    /// The current instant returned by the wrapped clock.
    #[inline(always)]
    fn now(&self) -> MonotonicInstant {
        self.as_ref().now()
    }

    /// Delegates timer creation without consuming the boxed clock.
    ///
    /// # Returns
    ///
    /// A timer in the wrapped clock's exact monotonic domain.
    #[inline(always)]
    fn new_timer(&self) -> Arc<dyn Timer> {
        self.as_ref().new_timer()
    }
}
