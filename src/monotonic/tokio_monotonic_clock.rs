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
#[derive(Debug)]
pub struct TokioMonotonicClock {
    domain: ClockDomain,
    origin: Instant,
}

impl TokioMonotonicClock {
    /// Creates a new Tokio clock domain at the current Tokio instant.
    ///
    /// The paired async sleeper requires a Tokio runtime with time enabled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domain: ClockDomain::new(),
            origin: Instant::now(),
        }
    }

    /// Returns the Tokio origin used by the paired async sleeper.
    pub(crate) const fn origin(&self) -> Instant {
        self.origin
    }
}

impl Default for TokioMonotonicClock {
    /// Creates a new independent Tokio monotonic clock domain.
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for TokioMonotonicClock {
    /// Returns the current instant in this clock's domain.
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.origin.elapsed())
    }
}
