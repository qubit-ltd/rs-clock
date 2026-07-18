// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    BlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    TimeError,
    Timer,
    WallClock,
};
use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

#[test]
fn test_manual_monotonic_clock_shared_helpers_use_same_timeline() {
    let clock = ManualMonotonicClock::new_shared();
    let wall_clock = clock.new_wall_clock(UNIX_EPOCH);
    let timer = clock.new_timer();
    let blocking_sleeper = BlockingSleeper::new(Arc::clone(&timer));

    assert_eq!(UNIX_EPOCH, wall_clock.now());
    assert_eq!(clock.now(), timer.clock().now());
    assert_eq!(clock.now(), blocking_sleeper.timer().clock().now());

    clock
        .advance(Duration::from_secs(4))
        .expect("short manual advance should succeed");

    assert_eq!(UNIX_EPOCH + Duration::from_secs(4), wall_clock.now());
    assert_eq!(clock.now(), timer.clock().now());
    assert_eq!(clock.now(), blocking_sleeper.timer().clock().now());
}

#[test]
fn test_manual_monotonic_clock_starts_at_zero() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();

    assert_eq!(Duration::ZERO, start.elapsed_since_origin());
}

#[test]
fn test_manual_monotonic_clock_advance_moves_forward() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();

    clock
        .advance(Duration::from_secs(10))
        .expect("short advance should succeed");

    assert_eq!(
        Duration::from_secs(10),
        clock
            .now()
            .duration_since(start)
            .expect("instants should share one domain"),
    );
}

#[test]
fn test_manual_monotonic_clock_advance_to_rejects_backward_target() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();
    clock
        .advance(Duration::from_secs(10))
        .expect("short advance should succeed");

    assert_eq!(Err(TimeError::CannotMoveBackward), clock.advance_to(start),);
}

#[test]
fn test_manual_monotonic_clock_advance_to_rejects_foreign_domain() {
    let clock = ManualMonotonicClock::new();
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();

    assert_eq!(
        Err(TimeError::ClockDomainMismatch {
            expected,
            actual: foreign.domain(),
        }),
        clock.advance_to(foreign),
    );
}

#[test]
fn test_manual_monotonic_clock_instances_have_distinct_domains() {
    let first = ManualMonotonicClock::new();
    let second = ManualMonotonicClock::new();

    assert_ne!(first.now().domain(), second.now().domain());
}

#[test]
fn test_manual_monotonic_clock_default_starts_at_zero() {
    let clock = ManualMonotonicClock::default();
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
}

#[test]
fn test_manual_monotonic_clock_debug_includes_domain() {
    let clock = ManualMonotonicClock::new();
    assert!(format!("{clock:?}").contains("domain"));
}

#[test]
fn test_manual_monotonic_clock_zero_advance_is_noop() {
    let clock = ManualMonotonicClock::new();
    let before = clock.now();
    clock
        .advance(Duration::ZERO)
        .expect("zero advance should succeed");
    assert_eq!(before, clock.now());
}

#[test]
fn test_manual_monotonic_clock_reports_advance_overflow() {
    let clock = ManualMonotonicClock::new();
    clock
        .advance(Duration::MAX)
        .expect("maximum duration should fit from zero");
    assert_eq!(
        Err(TimeError::InstantOverflow),
        clock.advance(Duration::from_nanos(1)),
    );
}

#[test]
fn test_manual_monotonic_clock_advance_to_current_is_noop() {
    let clock = ManualMonotonicClock::new();
    let current = clock.now();
    clock
        .advance_to(current)
        .expect("advancing to current instant should succeed");
    assert_eq!(current, clock.now());
}

#[tokio::test]
async fn test_manual_monotonic_clock_waits_and_advances_to_next_deadline_async()
{
    let clock = ManualMonotonicClock::new_shared();
    let driver_clock = Arc::clone(&clock);
    let driver = tokio::spawn(async move {
        driver_clock.advance_to_next_deadline_async().await
    });
    tokio::task::yield_now().await;
    assert!(!driver.is_finished());

    let timer = clock.new_timer();
    let timer_future = timer
        .after(Duration::from_secs(5))
        .expect("manual deadline should register");
    let reached = driver.await.expect("manual-time driver should finish");

    assert_eq!(Duration::from_secs(5), reached.elapsed_since_origin());
    timer_future.await;
}

#[tokio::test]
async fn test_manual_monotonic_clock_drives_mixed_waiters_in_deadline_order() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let blocking_sleeper = Arc::new(BlockingSleeper::new(Arc::clone(&timer)));
    let async_wait = timer
        .after(Duration::from_secs(2))
        .expect("timer deadline should register");
    let worker_sleeper = Arc::clone(&blocking_sleeper);
    let worker = thread::spawn(move || {
        worker_sleeper
            .sleep_for(Duration::from_secs(5))
            .expect("blocking wait should complete");
    });
    assert!(clock.wait_for_waiters(2, Duration::from_secs(1)));

    assert_eq!(2, clock.pending_waiters());
    assert_eq!(
        Duration::from_secs(2),
        clock
            .next_deadline()
            .expect("mixed waiters should have a deadline")
            .elapsed_since_origin(),
    );
    assert_eq!(
        Duration::from_secs(2),
        clock
            .advance_to_next_deadline()
            .expect("async deadline should exist")
            .elapsed_since_origin(),
    );
    async_wait.await;

    assert_eq!(1, clock.pending_waiters());
    assert_eq!(
        Duration::from_secs(5),
        clock
            .advance_to_next_deadline()
            .expect("blocking deadline should exist")
            .elapsed_since_origin(),
    );
    worker.join().expect("blocking waiter should finish");
    assert_eq!(None, clock.advance_to_next_deadline());
}

#[test]
fn test_manual_monotonic_clock_concurrent_advances_are_not_lost() {
    const THREADS: usize = 8;
    const ADVANCES_PER_THREAD: usize = 100;
    let clock = Arc::new(ManualMonotonicClock::new());
    let barrier = Arc::new(std::sync::Barrier::new(THREADS));
    let workers: Vec<_> = (0..THREADS)
        .map(|_| {
            let clock = Arc::clone(&clock);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..ADVANCES_PER_THREAD {
                    clock
                        .advance(Duration::from_nanos(1))
                        .expect("concurrent advance should succeed");
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("advance worker should finish");
    }

    assert_eq!(
        Duration::from_nanos((THREADS * ADVANCES_PER_THREAD) as u64),
        clock.now().elapsed_since_origin(),
    );
}

#[test]
fn test_manual_monotonic_clock_wait_for_waiters_times_out() {
    let clock = ManualMonotonicClock::new();
    assert!(!clock.wait_for_waiters(1, Duration::from_millis(1)));
}

/// Verifies that an already satisfied waiter count needs no real-time wait.
#[test]
fn test_manual_monotonic_clock_wait_for_waiters_is_already_satisfied() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let pending_sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");

    assert_eq!(1, clock.pending_waiters());
    assert!(clock.wait_for_waiters(1, Duration::ZERO));

    drop(pending_sleep);
    assert_eq!(0, clock.pending_waiters());
}

/// Verifies that an already satisfied waiter count takes precedence over an
/// unrepresentable real-time guard.
#[test]
fn test_manual_monotonic_clock_wait_for_waiters_prefers_satisfied_count() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let pending_sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");

    assert_eq!(1, clock.pending_waiters());
    assert!(clock.wait_for_waiters(1, Duration::MAX));

    drop(pending_sleep);
    assert_eq!(0, clock.pending_waiters());
}

/// Verifies that deadline coordination waits for a later registration after
/// the previous blocking waiter becomes due.
#[test]
fn test_manual_monotonic_clock_wait_for_next_deadline_tracks_retries() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let worker = thread::spawn(move || {
        sleeper
            .sleep_for(Duration::from_secs(1))
            .expect("first manual wait should complete");
        sleeper
            .sleep_for(Duration::from_secs(2))
            .expect("second manual wait should complete");
    });

    let first_deadline = clock
        .wait_for_next_deadline(Duration::from_secs(1))
        .expect("first deadline should be registered");
    assert_eq!(
        Duration::from_secs(1),
        first_deadline
            .duration_since(clock.now())
            .expect("first deadline should share the manual domain"),
    );
    clock
        .advance_to(first_deadline)
        .expect("manual time should reach the first deadline");

    let second_deadline = clock
        .wait_for_next_deadline(Duration::from_secs(1))
        .expect("second deadline should be registered");
    assert_eq!(
        Duration::from_secs(2),
        second_deadline
            .duration_since(clock.now())
            .expect("second deadline should share the manual domain"),
    );
    clock
        .advance_to(second_deadline)
        .expect("manual time should reach the second deadline");

    worker.join().expect("retry worker should finish");
}

/// Verifies that deadline coordination uses its real-time timeout as a guard.
#[test]
fn test_manual_monotonic_clock_wait_for_next_deadline_times_out() {
    let clock = ManualMonotonicClock::new();
    assert_eq!(None, clock.wait_for_next_deadline(Duration::ZERO));
    assert_eq!(None, clock.wait_for_next_deadline(Duration::from_millis(1)),);
    assert_eq!(None, clock.wait_for_next_deadline(Duration::MAX));
}

/// Verifies that an existing deadline takes precedence over an
/// unrepresentable real-time guard.
#[test]
fn test_manual_monotonic_clock_wait_for_next_deadline_prefers_existing() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let pending_sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");
    let expected_deadline = clock
        .next_deadline()
        .expect("pending sleep should register a deadline");

    assert_eq!(
        Some(expected_deadline),
        clock.wait_for_next_deadline(Duration::MAX),
    );

    drop(pending_sleep);
}

#[test]
fn test_manual_monotonic_clock_rejects_unrepresentable_guard_timeout() {
    let clock = ManualMonotonicClock::new();
    assert!(!clock.wait_for_waiters(1, Duration::MAX));
}
