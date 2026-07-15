// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
};
use std::time::Duration;

#[test]
fn test_manual_monotonic_state_accumulates_multiple_advances() {
    let clock = ManualMonotonicClock::new();
    clock
        .advance(Duration::from_secs(2))
        .expect("first advance should succeed");
    clock
        .advance(Duration::from_secs(3))
        .expect("second advance should succeed");

    assert_eq!(Duration::from_secs(5), clock.now().elapsed_since_origin());
}
