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
use std::num::NonZeroU64;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

/// Next unallocated clock domain identifier; zero marks exhaustion.
static NEXT_CLOCK_DOMAIN: AtomicU64 = AtomicU64::new(1);

/// Allocates an identifier from `next` without wrapping into a reused value.
///
/// The maximum `u64` value is returned once and atomically changes `next` to
/// the terminal zero state. Calls made after that transition panic.
fn allocate_clock_domain_identifier(next: &AtomicU64) -> u64 {
    next.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        NonZeroU64::new(value).map(|value| value.get().wrapping_add(1))
    })
    .expect("monotonic clock domain identifiers exhausted")
}

/// Identifies one monotonic clock timeline within this process.
///
/// A domain is allocated by new() and is carried by every monotonic instant
/// produced by its clock.
///
/// `ClockDomain` intentionally requires explicit allocation:
///
/// ```compile_fail
/// use qubit_clock::ClockDomain;
///
/// let domain = ClockDomain::default();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockDomain(u64);

impl ClockDomain {
    /// Allocates a domain that is not reused within this process.
    ///
    /// Every representable nonzero identifier, including the final one, can be
    /// allocated. After the final identifier is returned, later calls panic.
    ///
    /// # Panics
    ///
    /// Panics if all representable nonzero domain identifiers have been
    /// allocated rather than wrapping and reusing a prior identity.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(allocate_clock_domain_identifier(&NEXT_CLOCK_DOMAIN))
    }
}

impl Display for ClockDomain {
    /// Formats this domain for diagnostics.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
