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

use qubit_clock::sleep::{
    MockSleeper,
    Sleeper,
};

#[test]
fn test_new_starts_at_zero_elapsed() {
    let sleeper = MockSleeper::new();

    assert_eq!(Duration::ZERO, sleeper.elapsed());
}

#[test]
fn test_advance_updates_elapsed() {
    let sleeper = MockSleeper::new();

    sleeper.advance(Duration::from_millis(40));

    assert_eq!(Duration::from_millis(40), sleeper.elapsed());
}

#[test]
fn test_set_elapsed_replaces_elapsed() {
    let sleeper = MockSleeper::new();

    sleeper.advance(Duration::from_millis(40));
    sleeper.set_elapsed(Duration::from_millis(7));

    assert_eq!(Duration::from_millis(7), sleeper.elapsed());
}

#[test]
fn test_reset_sets_elapsed_to_zero() {
    let sleeper = MockSleeper::new();

    sleeper.advance(Duration::from_millis(40));
    sleeper.reset();

    assert_eq!(Duration::ZERO, sleeper.elapsed());
}

#[test]
fn test_sleep_for_blocks_until_mock_time_advances() {
    let sleeper = MockSleeper::new();
    let worker_sleeper = sleeper.clone();
    let (started_sender, started_receiver) = mpsc::channel();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        started_sender
            .send(())
            .expect("worker should report when sleep starts");
        worker_sleeper.sleep_for(Duration::from_millis(100));
        done_sender
            .send(())
            .expect("worker should report when sleep completes");
    });

    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("worker should start promptly");
    assert!(
        done_receiver.try_recv().is_err(),
        "mock sleep should not complete before time advances",
    );

    sleeper.advance(Duration::from_millis(99));
    assert!(
        done_receiver
            .recv_timeout(Duration::from_millis(20))
            .is_err(),
        "mock sleep should not complete before target elapsed time",
    );

    sleeper.advance(Duration::from_millis(1));
    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("mock sleep should complete after target elapsed time");
    worker.join().expect("worker should finish cleanly");
}

#[test]
fn test_sleep_for_uses_elapsed_at_call_time() {
    let sleeper = MockSleeper::new();
    sleeper.advance(Duration::from_millis(10));
    let worker_sleeper = sleeper.clone();
    let (started_sender, started_receiver) = mpsc::channel();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        started_sender
            .send(())
            .expect("worker should report when sleep starts");
        worker_sleeper.sleep_for(Duration::from_millis(100));
        done_sender
            .send(())
            .expect("worker should report when sleep completes");
    });

    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("worker should start promptly");
    sleeper.advance(Duration::from_millis(99));
    assert!(
        done_receiver
            .recv_timeout(Duration::from_millis(20))
            .is_err(),
        "sleep should be relative to elapsed at call time",
    );

    sleeper.advance(Duration::from_millis(1));
    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("sleep should complete after the full relative duration");
    worker.join().expect("worker should finish cleanly");
}
