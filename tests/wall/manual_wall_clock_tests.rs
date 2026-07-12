// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    ManualWallClock,
    MonotonicClock,
    WallClock,
};
use std::sync::Arc;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

#[test]
fn test_manual_wall_clock_starts_at_wall_anchor() {
    let monotonic_clock = Arc::new(ManualMonotonicClock::new());
    let start = UNIX_EPOCH + Duration::from_secs(100);
    let wall_clock =
        ManualWallClock::from_clock(start, Arc::clone(&monotonic_clock));

    assert_eq!(start, wall_clock.now());
}

#[test]
fn test_manual_wall_clock_follows_monotonic_advance() {
    let monotonic_clock = Arc::new(ManualMonotonicClock::new());
    let wall_clock =
        ManualWallClock::from_clock(UNIX_EPOCH, Arc::clone(&monotonic_clock));

    monotonic_clock
        .advance(Duration::from_secs(600))
        .expect("short advance should succeed");

    assert_eq!(UNIX_EPOCH + Duration::from_secs(600), wall_clock.now(),);
}

#[test]
fn test_manual_wall_clock_reanchor_changes_only_wall_mapping() {
    let monotonic_clock = Arc::new(ManualMonotonicClock::new());
    let wall_clock =
        ManualWallClock::from_clock(UNIX_EPOCH, Arc::clone(&monotonic_clock));
    monotonic_clock
        .advance(Duration::from_secs(10))
        .expect("short advance should succeed");
    let monotonic_before = monotonic_clock.now();

    let new_wall_time = UNIX_EPOCH + Duration::from_secs(1_000);
    wall_clock.reanchor(new_wall_time);

    assert_eq!(new_wall_time, wall_clock.now());
    assert_eq!(monotonic_before, monotonic_clock.now());

    monotonic_clock
        .advance(Duration::from_secs(5))
        .expect("short advance should succeed");
    assert_eq!(UNIX_EPOCH + Duration::from_secs(1_005), wall_clock.now(),);
}
