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
    assert_eq!(0, clock.pending_waiters());
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

    assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
    assert_eq!(1, clock.pending_waiters());
    assert_eq!(
        Some(
            clock
                .now()
                .checked_add(Duration::from_secs(10))
                .expect("short deadline should be representable"),
        ),
        clock.next_deadline(),
    );

    clock
        .advance(Duration::from_secs(9))
        .expect("short advance should succeed");
    assert_eq!(1, clock.pending_waiters());

    clock
        .advance(Duration::from_secs(1))
        .expect("short advance should succeed");
    worker.join().expect("worker should finish without panic");
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_blocking_sleeper_registration_racing_advance_does_not_hang() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
    let deadline = clock
        .now()
        .checked_add(Duration::from_secs(1))
        .expect("short deadline should fit");
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let worker_barrier = Arc::clone(&barrier);
    let (done_sender, done_receiver) = std::sync::mpsc::channel();
    let worker = thread::spawn(move || {
        worker_barrier.wait();
        let result = sleeper.sleep_until(deadline);
        done_sender
            .send(result)
            .expect("test should receive sleep result");
    });

    barrier.wait();
    clock
        .advance(Duration::from_secs(1))
        .expect("short concurrent advance should succeed");

    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("racing registration must not lose the time change")
        .expect("same-domain sleep should succeed");
    worker.join().expect("sleep worker should finish");
}
