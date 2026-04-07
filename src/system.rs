/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! System clock implementation.
//!
//! This module provides [`SystemClock`], a clock implementation that uses the
//! system's wall clock time. The time is subject to system time adjustments
//! (e.g., NTP synchronization, manual changes).
//!
//! # Author
//!
//! Haixing Hu

use crate::Clock;
use chrono::{DateTime, Utc};

/// A clock implementation that uses the system's wall clock time.
///
/// This is a zero-sized type (ZST) with no runtime overhead. It directly
/// queries the system for the current time whenever [`millis()`](Clock::millis)
/// or [`time()`](Clock::time) is called.
///
/// # Note
///
/// The time returned by this clock is subject to system time adjustments,
/// such as NTP synchronization or manual changes. For monotonic time
/// measurements, use [`MonotonicClock`](crate::MonotonicClock) instead.
///
/// # Thread Safety
///
/// This type is completely thread-safe as it has no mutable state.
///
/// # Examples
///
/// ```
/// use qubit_clock::{Clock, SystemClock};
///
/// let clock = SystemClock::new();
/// let timestamp = clock.millis();
/// let time = clock.time();
/// println!("Current system time: {}", time);
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    /// Creates a new `SystemClock`.
    ///
    /// # Returns
    ///
    /// A new `SystemClock` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_clock::SystemClock;
    ///
    /// let clock = SystemClock::new();
    /// ```
    ///
    pub fn new() -> Self {
        SystemClock
    }
}

impl Clock for SystemClock {
    fn millis(&self) -> i64 {
        Utc::now().timestamp_millis()
    }

    fn time(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
