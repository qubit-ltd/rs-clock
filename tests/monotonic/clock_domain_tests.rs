// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ClockDomain,
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
};
use std::collections::HashSet;
use std::time::Duration;

use super::clock_domain::next_identifier_state;

#[test]
fn test_clock_domain_new_creates_distinct_domains() {
    let first = ClockDomain::new();
    let second = ClockDomain::new();

    assert_ne!(first, second);
    assert_ne!("0", first.to_string());
    assert_ne!("0", second.to_string());
}

#[test]
fn test_clock_domain_identifier_state_reaches_terminal_zero() {
    assert_eq!(Some(2), next_identifier_state(1));
    assert_eq!(Some(0), next_identifier_state(u64::MAX));
    assert_eq!(None, next_identifier_state(0));
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
