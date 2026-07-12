// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Allocates process-wide monotonic clock domain identifiers.

use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

/// Next unallocated clock domain identifier; zero is reserved.
static NEXT_CLOCK_DOMAIN_ID: AtomicU64 = AtomicU64::new(1);

/// Allocates a domain identifier that is never reused within this process.
///
/// # Panics
///
/// Panics before the reserved `u64::MAX` sentinel could be reused rather than
/// wrapping and reusing an existing domain identifier.
pub fn allocate_clock_domain_id() -> u64 {
    NEXT_CLOCK_DOMAIN_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .expect("monotonic clock domain identifiers exhausted")
}
