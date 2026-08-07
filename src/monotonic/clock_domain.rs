// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines unique monotonic clock domains.

use std::fmt::Display;
use std::fmt::Formatter;
use std::num::NonZeroU64;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Next unallocated clock domain identifier; zero marks exhaustion.
static NEXT_CLOCK_DOMAIN: AtomicU64 = AtomicU64::new(1);

/// Returns the allocator state following `identifier`, or `None` when zero
/// already marks exhaustion.
///
/// The maximum identifier transitions to zero so it remains allocatable once
/// without wrapping into a reused nonzero identifier.
///
/// # Parameters
///
/// * `identifier` - The current allocator state.
///
/// # Returns
///
/// The next allocator state, or `None` when `identifier` is already zero.
#[inline(always)]
pub(crate) fn next_identifier_state(identifier: u64) -> Option<u64> {
    NonZeroU64::new(identifier)
        .map(|identifier| identifier.get().wrapping_add(1))
}

/// Allocates an identifier from `next` without wrapping into a reused value.
///
/// The maximum `u64` value is returned once and atomically changes `next` to
/// the terminal zero state. Calls made after that transition panic.
///
/// # Parameters
///
/// * `next` - Atomic allocator state to advance.
///
/// # Returns
///
/// A process-unique nonzero clock-domain identifier.
///
/// # Panics
///
/// Panics when the allocator has already reached its terminal zero state.
#[must_use = "the allocated domain identifier must initialize a clock domain"]
#[inline]
fn allocate_clock_domain_identifier(next: &AtomicU64) -> u64 {
    next.fetch_update(
        Ordering::Relaxed,
        Ordering::Relaxed,
        next_identifier_state,
    )
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
#[must_use = "clock domains should be retained to identify monotonic timelines"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockDomain(
    /// Process-unique nonzero domain identifier.
    u64,
);

impl ClockDomain {
    /// Allocates a domain that is not reused within this process.
    ///
    /// Every representable nonzero identifier, including the final one, can be
    /// allocated. After the final identifier is returned, later calls panic.
    ///
    /// # Returns
    ///
    /// A newly allocated process-unique clock domain.
    ///
    /// # Panics
    ///
    /// Panics if all representable nonzero domain identifiers have been
    /// allocated rather than wrapping and reusing a prior identity.
    #[allow(clippy::new_without_default)]
    #[inline(always)]
    pub fn new() -> Self {
        Self(allocate_clock_domain_identifier(&NEXT_CLOCK_DOMAIN))
    }
}

impl Display for ClockDomain {
    /// Formats this domain for diagnostics.
    ///
    /// # Parameters
    ///
    /// * `formatter` - Destination formatter.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the identifier is formatted.
    ///
    /// # Errors
    ///
    /// Returns [`std::fmt::Error`] when the formatter rejects the output.
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let Self(identifier) = self;
        identifier.fmt(formatter)
    }
}
