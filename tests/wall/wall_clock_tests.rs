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

fn wall_time<C: WallClock>(clock: C) -> std::time::SystemTime {
    clock.now()
}

#[test]
fn test_wall_clock_reference_delegates_to_concrete_and_trait_object() {
    let clock = FixedWallClock::new(UNIX_EPOCH);
    let trait_object: &dyn WallClock = &clock;

    assert_eq!(UNIX_EPOCH, wall_time(clock));
    assert_eq!(UNIX_EPOCH, wall_time(trait_object));
}
