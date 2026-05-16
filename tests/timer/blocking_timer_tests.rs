/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use qubit_clock::timer::{
    BlockingTimer,
    MockTimer,
    MonotonicTimer,
    TimerError,
    TimerInstant,
    TimerWaitOutcome,
};

struct ScriptedBlockingTimer {
    timer: MockTimer,
    outcomes: Mutex<VecDeque<TimerWaitOutcome>>,
}

impl ScriptedBlockingTimer {
    fn new(outcomes: impl IntoIterator<Item = TimerWaitOutcome>) -> Self {
        Self {
            timer: MockTimer::new(),
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        }
    }
}

impl MonotonicTimer for ScriptedBlockingTimer {
    fn timer_domain_id(&self) -> qubit_clock::timer::TimerDomainId {
        self.timer.timer_domain_id()
    }

    fn now(&self) -> qubit_clock::timer::TimerInstant {
        self.timer.now()
    }
}

impl BlockingTimer for ScriptedBlockingTimer {
    fn wait_until(&self, deadline: TimerInstant) -> Result<TimerWaitOutcome, TimerError> {
        let _ = self.timer.duration_until(deadline)?;
        let mut outcomes = self
            .outcomes
            .lock()
            .expect("scripted timer outcomes should not be poisoned");
        let outcome = outcomes
            .pop_front()
            .expect("scripted timer should have enough wait outcomes");
        Ok(outcome)
    }

    fn notify_waiters(&self) {}
}

#[test]
fn test_sleep_for_ignores_notifications_until_deadline_is_reached() {
    let timer = MockTimer::new();
    let worker_timer = timer.clone();
    let (started_sender, started_receiver) = mpsc::channel();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        started_sender
            .send(())
            .expect("worker should report when it starts waiting");
        worker_timer
            .sleep_for(Duration::from_millis(100))
            .expect("sleeping with the timer's own deadline should succeed");
        done_sender
            .send(())
            .expect("worker should report when the deadline is reached");
    });

    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("worker should start waiting promptly");
    timer.notify_waiters();
    assert!(
        done_receiver.try_recv().is_err(),
        "notification should not complete sleep_for before the deadline",
    );

    timer.advance(Duration::from_millis(100));
    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("advancing mock time should complete sleep_for");
    worker.join().expect("worker thread should finish cleanly");
}

#[test]
fn test_sleep_until_continues_after_notified_outcome() {
    let timer = ScriptedBlockingTimer::new([
        TimerWaitOutcome::Notified,
        TimerWaitOutcome::DeadlineReached,
    ]);
    let deadline = timer.deadline_after(Duration::from_millis(10));

    timer
        .sleep_until(deadline)
        .expect("scripted same-domain deadline should succeed");
}
