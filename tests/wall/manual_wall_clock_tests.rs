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
use std::sync::Barrier;
use std::sync::atomic::{
    AtomicBool,
    Ordering,
};
use std::thread;
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

#[test]
fn test_manual_wall_clock_now_panics_when_system_time_overflows() {
    let monotonic_clock = Arc::new(ManualMonotonicClock::new());
    let wall_clock =
        ManualWallClock::from_clock(UNIX_EPOCH, Arc::clone(&monotonic_clock));
    monotonic_clock
        .advance(Duration::MAX)
        .expect("maximum duration should fit the manual monotonic clock");

    let panic = std::panic::catch_unwind(|| wall_clock.now());

    assert!(
        panic.is_err(),
        "wall-clock reading should panic after SystemTime overflow",
    );
}

#[test]
fn test_manual_wall_clock_concurrent_reanchor_never_mixes_snapshots() {
    const ITERATIONS: u64 = 10_000;
    const READERS: usize = 4;
    const WALL_STRIDE_SECONDS: u64 = 100;
    let monotonic_clock = Arc::new(ManualMonotonicClock::new());
    let wall_clock = Arc::new(ManualWallClock::from_clock(
        UNIX_EPOCH,
        Arc::clone(&monotonic_clock),
    ));
    let barrier = Arc::new(Barrier::new(READERS + 1));
    let mixed_snapshot_observed = Arc::new(AtomicBool::new(false));
    let readers: Vec<_> = (0..READERS)
        .map(|_| {
            let wall_clock = Arc::clone(&wall_clock);
            let barrier = Arc::clone(&barrier);
            let mixed_snapshot_observed = Arc::clone(&mixed_snapshot_observed);
            thread::spawn(move || {
                for iteration in 0..ITERATIONS {
                    barrier.wait();
                    let observed = wall_clock.now();
                    let old_mapping = if iteration == 0 {
                        UNIX_EPOCH
                    } else {
                        UNIX_EPOCH
                            + Duration::from_secs(
                                iteration * WALL_STRIDE_SECONDS,
                            )
                            + Duration::from_nanos(1)
                    };
                    let new_mapping = UNIX_EPOCH
                        + Duration::from_secs(
                            (iteration + 1) * WALL_STRIDE_SECONDS,
                        );
                    let new_mapping_after_advance =
                        new_mapping + Duration::from_nanos(1);
                    if observed != old_mapping
                        && observed != new_mapping
                        && observed != new_mapping_after_advance
                    {
                        mixed_snapshot_observed.store(true, Ordering::SeqCst);
                    }
                    barrier.wait();
                }
            })
        })
        .collect();

    for iteration in 0..ITERATIONS {
        barrier.wait();
        wall_clock.reanchor(
            UNIX_EPOCH
                + Duration::from_secs((iteration + 1) * WALL_STRIDE_SECONDS),
        );
        monotonic_clock
            .advance(Duration::from_nanos(1))
            .expect("small concurrent advance should succeed");
        barrier.wait();
    }
    for reader in readers {
        reader.join().expect("wall-clock reader should finish");
    }
    assert!(
        !mixed_snapshot_observed.load(Ordering::SeqCst),
        "wall reading mixed an old anchor with a new monotonic sample",
    );
}
