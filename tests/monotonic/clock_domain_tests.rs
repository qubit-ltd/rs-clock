// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ClockDomain,
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
};
use std::collections::HashSet;
use std::time::Duration;

/// Production source containing the clock domain's public API declaration.
const CLOCK_DOMAIN_SOURCE: &str =
    include_str!("../../src/monotonic/clock_domain.rs");

#[test]
fn test_clock_domain_new_creates_distinct_domains() {
    let first = ClockDomain::new();
    let second = ClockDomain::new();

    assert_ne!(first, second);
    assert_ne!("0", first.to_string());
    assert_ne!("0", second.to_string());
}

#[test]
fn test_clock_domain_does_not_implement_default() {
    assert!(
        !CLOCK_DOMAIN_SOURCE.contains("impl Default for ClockDomain"),
        "clock domains must be allocated explicitly",
    );
}

#[test]
fn test_clock_domain_allocator_exhausts_after_maximum_identifier() {
    assert!(
        CLOCK_DOMAIN_SOURCE.contains("u64::MAX => Some(0)"),
        "the maximum identifier must be returned while marking exhaustion",
    );
    assert!(
        CLOCK_DOMAIN_SOURCE.contains("0 => None"),
        "the exhausted allocator state must reject later allocations",
    );
    assert!(
        CLOCK_DOMAIN_SOURCE.contains(
            ".expect(\"monotonic clock domain identifiers exhausted\")",
        ),
        "a rejected allocation must panic with the exhaustion diagnostic",
    );
}

#[test]
fn test_clock_domain_supports_hash_collections() {
    let first = ClockDomain::new();
    let second = ClockDomain::new();
    let domains = HashSet::from([first, second]);

    assert_eq!(2, domains.len());
}

#[test]
fn test_monotonic_instant_new_preserves_clock_domain() {
    let domain = ClockDomain::new();
    let instant = MonotonicInstant::new(domain, Duration::from_secs(3));

    assert_eq!(domain, instant.domain());
    assert_eq!(Duration::from_secs(3), instant.elapsed_since_origin());
}

#[test]
fn test_manual_monotonic_clock_creates_distinct_domains() {
    let first = ManualMonotonicClock::new().now().domain();
    let second = ManualMonotonicClock::new().now().domain();

    assert_ne!(first, second);
}
