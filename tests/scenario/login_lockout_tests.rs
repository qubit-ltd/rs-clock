// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::ManualWallClock;
use qubit_clock::WallClock;

const LOCK_DURATION: Duration = Duration::from_secs(10 * 60);

#[test]
fn test_login_unlocks_after_ten_minutes_of_manual_time() {
    let monotonic_clock = Arc::new(ManualMonotonicClock::new());
    let wall_clock =
        ManualWallClock::from_clock(UNIX_EPOCH, Arc::clone(&monotonic_clock));
    let locked_until = lock_after_five_failures(&wall_clock, 5);

    assert!(is_locked(&wall_clock, locked_until));
    monotonic_clock
        .advance(Duration::from_secs(599))
        .expect("manual advance should succeed");
    assert!(is_locked(&wall_clock, locked_until));

    monotonic_clock
        .advance(Duration::from_secs(1))
        .expect("manual advance should succeed");
    assert!(!is_locked(&wall_clock, locked_until));
}

/// Calculates the lock deadline after five consecutive failures.
fn lock_after_five_failures(
    clock: &dyn WallClock,
    failure_count: usize,
) -> Option<SystemTime> {
    (failure_count >= 5).then(|| {
        clock
            .now()
            .checked_add(LOCK_DURATION)
            .expect("short lock duration should be representable")
    })
}

/// Returns whether the current wall time is before the lock deadline.
fn is_locked(clock: &dyn WallClock, locked_until: Option<SystemTime>) -> bool {
    locked_until.is_some_and(|deadline| clock.now() < deadline)
}
