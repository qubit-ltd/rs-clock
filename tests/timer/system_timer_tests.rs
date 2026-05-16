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
    MonotonicTimer,
    SystemTimer,
    TimerWaitOutcome,
};

#[test]
fn test_wait_for_reaches_real_deadline() {
    let timer = SystemTimer::new();

    assert_eq!(
        TimerWaitOutcome::DeadlineReached,
        timer
            .wait_for(Duration::from_millis(1))
            .expect("waiting on a self-created deadline should succeed"),
    );
}

#[test]
fn test_default_creates_system_timer() {
    let timer = SystemTimer::default();

    assert_eq!(timer.timer_domain_id(), timer.now().domain_id());
}

#[test]
fn test_wait_until_can_be_notified_before_deadline() {
    let timer = SystemTimer::new();
    let worker_timer = timer.clone();
    let deadline = timer.deadline_after(Duration::from_secs(5));
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
    for _ in 0..100 {
        timer.notify_waiters();
        if let Ok(outcome) = outcome_receiver.recv_timeout(Duration::from_millis(10)) {
            assert_eq!(TimerWaitOutcome::Notified, outcome);
            worker.join().expect("worker thread should finish cleanly");
            return;
        }
    }

    panic!("notification should wake wait_until promptly");
}
