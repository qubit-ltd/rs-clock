// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the Tokio monotonic clock implementation.

use crate::{
    ClockDomain,
    MonotonicClock,
    MonotonicInstant,
};
use tokio::time::Instant;

/// A monotonic clock backed by Tokio's time driver.
///
/// It follows Tokio pause and advance semantics. The type intentionally does
/// not implement [`Clone`]; shared identity uses `Arc<TokioMonotonicClock>`.
///
/// When Tokio time is paused or explicitly advanced, create this clock after
/// entering the runtime and read it only from that runtime. A paired
/// [`TokioAsyncSleeper`](crate::TokioAsyncSleeper) must also be polled by the
/// same runtime time driver. Moving tasks between worker threads of one runtime
/// is supported; moving the clock or sleeper between independent runtimes is
/// not. Driver identity is a caller contract because Tokio does not expose an
/// identity that this crate can validate.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioMonotonicClock {
    /// Domain carried by instants sampled from this clock.
    domain: ClockDomain,
    /// Native Tokio instant mapped to elapsed duration zero.
    origin: Instant,
}

impl TokioMonotonicClock {
    /// Creates a new Tokio clock domain at the current Tokio instant.
    ///
    /// Calling this method does not itself require a Tokio runtime. When using
    /// paused or explicitly advanced Tokio time, call it after entering the
    /// runtime whose time driver will read this clock and poll its paired
    /// sleeper.
    ///
    /// # Returns
    ///
    /// A Tokio monotonic clock with a newly allocated domain.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            domain: ClockDomain::new(),
            origin: Instant::now(),
        }
    }

    /// Returns the Tokio origin used by the paired async sleeper.
    ///
    /// # Returns
    ///
    /// The Tokio instant mapped to elapsed duration zero.
    #[inline(always)]
    pub(crate) const fn origin(&self) -> Instant {
        self.origin
    }

    /// Returns this concrete clock's domain without sampling Tokio time.
    ///
    /// # Returns
    ///
    /// This clock's process-unique domain.
    #[inline(always)]
    pub(crate) const fn domain(&self) -> ClockDomain {
        self.domain
    }
}

impl Default for TokioMonotonicClock {
    /// Creates a new independent Tokio monotonic clock domain.
    ///
    /// # Returns
    ///
    /// A Tokio monotonic clock with a newly allocated domain.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for TokioMonotonicClock {
    /// Returns the current instant in this clock's domain.
    ///
    /// # Returns
    ///
    /// The current elapsed duration represented in this clock's domain.
    #[inline]
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.origin.elapsed())
    }
}
