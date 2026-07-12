// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    AsyncSleeper,
    BlockingSleeper,
    ManualAsyncSleeper,
    ManualBlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    TimeError,
};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
    let expected = clock.now().domain_id();

    assert_eq!(
        Err(TimeError::ClockDomainMismatch {
            expected,
            actual: foreign.domain_id(),
        }),
        clock.advance_to(foreign),
    );
}

#[test]
fn test_manual_monotonic_clock_instances_have_distinct_domains() {
    let first = ManualMonotonicClock::new();
    let second = ManualMonotonicClock::new();

    assert_ne!(first.now().domain_id(), second.now().domain_id());
}

#[test]
fn test_manual_monotonic_clock_default_starts_at_zero() {
    let clock = ManualMonotonicClock::default();
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
}

#[test]
fn test_manual_monotonic_clock_debug_includes_domain_id() {
    let clock = ManualMonotonicClock::new();
    assert!(format!("{clock:?}").contains("domain_id"));
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
async fn test_manual_monotonic_clock_drives_mixed_waiters_in_deadline_order() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let async_sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let blocking_sleeper =
        Arc::new(ManualBlockingSleeper::from_clock(Arc::clone(&clock)));
    let async_wait = async_sleeper.sleep_for_async(Duration::from_secs(2));
    let worker_sleeper = Arc::clone(&blocking_sleeper);
    let worker = thread::spawn(move || {
        worker_sleeper
            .sleep_for(Duration::from_secs(5))
            .expect("blocking wait should complete");
    });
    assert!(blocking_sleeper.wait_for_waiters(1, Duration::from_secs(1)));

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
            .expect("advancing to registered deadline should succeed")
            .expect("async deadline should exist")
            .elapsed_since_origin(),
    );
    async_wait.await.expect("async wait should complete");

    assert_eq!(1, clock.pending_waiters());
    assert_eq!(
        Duration::from_secs(5),
        clock
            .advance_to_next_deadline()
            .expect("advancing to blocking deadline should succeed")
            .expect("blocking deadline should exist")
            .elapsed_since_origin(),
    );
    worker.join().expect("blocking waiter should finish");
    assert_eq!(None, clock.advance_to_next_deadline().unwrap());
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
        clock.elapsed_since_origin(),
    );
}
