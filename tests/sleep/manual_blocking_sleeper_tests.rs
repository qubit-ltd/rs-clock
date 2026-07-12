// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    BlockingSleeper,
    ManualBlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    TimeError,
};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_manual_blocking_sleeper_uses_supplied_clock_domain() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
    assert_eq!(clock.now().domain_id(), sleeper.now().domain_id());
}

#[test]
fn test_manual_blocking_sleeper_rejects_foreign_deadline() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
    let foreign = ManualMonotonicClock::new().now();

    assert_eq!(
        Err(TimeError::ClockDomainMismatch {
            expected: clock.now().domain_id(),
            actual: foreign.domain_id(),
        }),
        sleeper.sleep_until(foreign),
    );
}

#[test]
fn test_manual_blocking_sleeper_reached_deadline_returns_immediately() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
    let deadline = clock.now();

    sleeper
        .sleep_until(deadline)
        .expect("reached deadline should complete immediately");
    assert_eq!(0, sleeper.pending_waiters());
}

#[test]
fn test_manual_blocking_sleeper_blocks_until_clock_advances() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper =
        Arc::new(ManualBlockingSleeper::from_clock(Arc::clone(&clock)));
    let worker_sleeper = Arc::clone(&sleeper);
    let worker = thread::spawn(move || {
        worker_sleeper
            .sleep_for(Duration::from_secs(10))
            .expect("manual sleep should complete after advance");
    });

    assert!(sleeper.wait_for_waiters(1, Duration::from_secs(1)));
    assert_eq!(1, sleeper.pending_waiters());
    assert_eq!(
        Some(
            clock
                .now()
                .checked_add(Duration::from_secs(10))
                .expect("short deadline should be representable"),
        ),
        sleeper.next_deadline(),
    );

    clock
        .advance(Duration::from_secs(9))
        .expect("short advance should succeed");
    assert_eq!(1, sleeper.pending_waiters());

    clock
        .advance(Duration::from_secs(1))
        .expect("short advance should succeed");
    worker.join().expect("worker should finish without panic");
    assert_eq!(0, sleeper.pending_waiters());
}

#[test]
fn test_manual_blocking_sleeper_wait_for_waiters_times_out() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
    assert!(!sleeper.wait_for_waiters(1, Duration::from_millis(1)));
}

#[test]
fn test_manual_blocking_sleeper_rejects_unrepresentable_guard_timeout() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
    assert!(!sleeper.wait_for_waiters(1, Duration::MAX));
}
