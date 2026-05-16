/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use qubit_clock::timer::{
    BlockingTimer,
    MockTimer,
    MonotonicTimer,
    TimerError,
    TimerWaitOutcome,
};

#[test]
fn test_advance_updates_now_and_wakes_waiters() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();
    let deadline = timer.deadline_after(Duration::from_millis(25));
    let (started_sender, started_receiver) = mpsc::channel();
    let (outcome_sender, outcome_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        started_sender
            .send(())
            .expect("worker should report when it starts waiting");
        let outcome = worker_timer
            .wait_until(deadline)
            .expect("deadline belongs to the worker timer clone");
        outcome_sender
            .send(outcome)
            .expect("worker should report the wait outcome");
    });

    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("worker should start waiting promptly");
    timer.advance(Duration::from_millis(25));

    assert_eq!(
        TimerWaitOutcome::DeadlineReached,
        outcome_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("advancing mock time should wake wait_until"),
    );
    worker.join().expect("worker thread should finish cleanly");
}

#[test]
fn test_set_elapsed_and_reset_control_mock_time() {
    let timer = MockTimer::new();

    timer.set_elapsed(Duration::from_secs(2));
    assert_eq!(
        Duration::from_secs(2),
        timer.now().elapsed_since_timer_start()
    );

    timer.reset();
    assert_eq!(Duration::ZERO, timer.now().elapsed_since_timer_start());
}

#[test]
fn test_default_creates_mock_timer_at_zero() {
    let timer = MockTimer::default();

    assert_eq!(Duration::ZERO, timer.now().elapsed_since_timer_start());
}

#[test]
fn test_wait_until_rejects_foreign_deadline() {
    let timer = MockTimer::new();
    let other = MockTimer::new();

    let error = timer
        .wait_until(other.now())
        .expect_err("foreign deadline should be rejected");

    assert!(matches!(
        error,
        TimerError::TimerDomainMismatch {
            expected: _,
            actual: _
        }
    ));
}
