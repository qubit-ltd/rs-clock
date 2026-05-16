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
    TimerDomain,
    TimerWaitOutcome,
};

#[test]
fn test_sleep_until_ignores_notifications_until_deadline_is_reached() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();
    let deadline = timer.deadline_after(Duration::from_millis(100));
    let (started_sender, started_receiver) = mpsc::channel();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        started_sender
            .send(())
            .expect("worker should report when it starts waiting");
        worker_timer
            .sleep_until(deadline)
            .expect("sleeping with the timer's own deadline should succeed");
        done_sender
            .send(())
            .expect("worker should report when the deadline is reached");
    });

    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("worker should start waiting promptly");
    timer.notify_all_waiters();
    assert!(
        done_receiver.try_recv().is_err(),
        "notification should not complete sleep_until before the deadline",
    );

    timer.advance(Duration::from_millis(100));
    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("advancing mock time should complete sleep_until");
    worker.join().expect("worker thread should finish cleanly");
}

#[test]
fn test_wait_for_returns_notified_after_notification() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();
    let (outcome_sender, outcome_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        let outcome = worker_timer
            .wait_for(Duration::from_millis(100))
            .expect("waiting with a self-created deadline should succeed");
        outcome_sender
            .send(outcome)
            .expect("worker should report the wait outcome");
    });

    for _ in 0..100 {
        timer.notify_all_waiters();
        if let Ok(outcome) = outcome_receiver.recv_timeout(Duration::from_millis(10)) {
            assert_eq!(TimerWaitOutcome::Notified, outcome);
            worker.join().expect("worker thread should finish cleanly");
            return;
        }
    }

    panic!("notification should complete wait_for");
}

#[test]
fn test_wait_until_continues_after_time_advance_before_deadline() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();
    let deadline = timer.deadline_after(Duration::from_millis(100));
    let (outcome_sender, outcome_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        let outcome = worker_timer
            .wait_until(deadline)
            .expect("waiting with a same-domain deadline should succeed");
        outcome_sender
            .send(outcome)
            .expect("worker should report the wait outcome");
    });

    timer.advance(Duration::from_millis(50));
    assert!(
        outcome_receiver
            .recv_timeout(Duration::from_millis(20))
            .is_err(),
        "time advancement before the deadline should only re-check wait_for",
    );

    timer.advance(Duration::from_millis(50));

    assert_eq!(
        TimerWaitOutcome::DeadlineReached,
        outcome_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("reaching the deadline should complete wait_for"),
    );
    worker.join().expect("worker thread should finish cleanly");
}
