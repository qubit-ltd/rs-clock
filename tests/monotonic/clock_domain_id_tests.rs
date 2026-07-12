// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    allocate_clock_domain_id,
};

#[test]
fn test_clock_domain_id_generator_assigns_nonzero_unique_ids() {
    let first = ManualMonotonicClock::new().now().domain_id();
    let second = ManualMonotonicClock::new().now().domain_id();

    assert_ne!(0, first);
    assert_ne!(0, second);
    assert_ne!(first, second);
}

#[test]
fn test_allocate_clock_domain_id_is_public_and_unique() {
    let first = allocate_clock_domain_id();
    let second = allocate_clock_domain_id();

    assert_ne!(0, first);
    assert_ne!(first, second);
}
