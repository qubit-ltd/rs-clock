// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{StdWallClock, WallClock};
use std::time::SystemTime;

#[test]
fn test_std_wall_clock_returns_system_time() {
    let _: SystemTime = StdWallClock::new().now();
}

#[test]
fn test_std_wall_clock_is_zero_sized() {
    assert_eq!(0, size_of::<StdWallClock>());
}
