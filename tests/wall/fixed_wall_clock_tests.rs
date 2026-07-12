// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    FixedWallClock,
    WallClock,
};
use std::time::{
    Duration,
    UNIX_EPOCH,
};

#[test]
fn test_fixed_wall_clock_always_returns_fixed_time() {
    let fixed = UNIX_EPOCH + Duration::from_secs(123);
    let clock = FixedWallClock::new(fixed);

    assert_eq!(fixed, clock.now());
    assert_eq!(fixed, clock.now());
}

#[test]
fn test_fixed_wall_clock_exposes_fixed_time() {
    let fixed = UNIX_EPOCH + Duration::from_secs(456);
    let clock = FixedWallClock::new(fixed);
    assert_eq!(fixed, clock.fixed_time());
}
