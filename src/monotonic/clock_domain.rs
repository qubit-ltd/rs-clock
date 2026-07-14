// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines unique monotonic clock domains.

use std::fmt::{
    Display,
    Formatter,
};
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

/// Next unallocated clock domain identifier; zero is reserved.
static NEXT_CLOCK_DOMAIN: AtomicU64 = AtomicU64::new(1);

/// Identifies one monotonic clock timeline within this process.
///
/// A domain is allocated by new() and is carried by every monotonic instant
/// produced by its clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockDomain(u64);

impl ClockDomain {
    /// Allocates a domain that is not reused within this process.
    ///
    /// # Panics
    ///
    /// Panics if all representable nonzero domain identifiers have been
    /// allocated rather than wrapping and reusing a prior identity.
    #[must_use]
    pub fn new() -> Self {
        Self(
            NEXT_CLOCK_DOMAIN
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
                .expect("monotonic clock domain identifiers exhausted"),
        )
    }
}

impl Default for ClockDomain {
    /// Allocates a new clock domain.
    ///
    /// This is equivalent to calling new and never returns a sentinel domain.
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ClockDomain {
    /// Formats this domain for diagnostics.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
