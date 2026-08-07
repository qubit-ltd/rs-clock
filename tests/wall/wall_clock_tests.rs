// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::time::UNIX_EPOCH;

use qubit_clock::FixedWallClock;
use qubit_clock::WallClock;

#[test]
fn test_wall_clock_supports_trait_object() {
    let clock: Arc<dyn WallClock> = Arc::new(FixedWallClock::new(UNIX_EPOCH));
    assert_eq!(UNIX_EPOCH, clock.now());
}

#[test]
fn test_wall_clock_arc_delegates_to_shared_object() {
    let clock = Arc::new(FixedWallClock::new(UNIX_EPOCH));
    assert_eq!(UNIX_EPOCH, WallClock::now(&clock));
}

#[test]
fn test_wall_clock_box_delegates_to_inner_object() {
    let clock: Box<dyn WallClock> = Box::new(FixedWallClock::new(UNIX_EPOCH));
    assert_eq!(UNIX_EPOCH, clock.now());
}
