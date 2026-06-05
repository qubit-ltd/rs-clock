// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for timeline-backed mock sleepers.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use qubit_clock::MockWaiterKind;
use qubit_clock::sleep::{
    MockSleeper,
    Sleeper,
};

/// Verifies new sleepers start with a zero-elapsed timeline.
///
/// # Errors
/// The test fails if the sleeper timeline starts advanced.
#[test]
fn test_new_starts_with_zero_timeline_elapsed() {
    let sleeper = MockSleeper::new();

    assert_eq!(Duration::ZERO, sleeper.timeline().elapsed());
}

/// Verifies default sleepers start with a zero-elapsed timeline.
///
/// # Errors
/// The test fails if the default timeline starts advanced.
#[test]
fn test_default_starts_with_zero_timeline_elapsed() {
    let sleeper = MockSleeper::default();

    assert_eq!(Duration::ZERO, sleeper.timeline().elapsed());
}

/// Verifies sleep completion is driven by the backing timeline.
///
/// # Errors
/// The test fails if advancing the timeline does not unblock the sleeper.
#[test]
fn test_sleep_for_blocks_until_timeline_advances() {
    let sleeper = MockSleeper::new();
    let timeline = sleeper.timeline();
    let worker_sleeper = sleeper.clone();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        worker_sleeper.sleep_for(Duration::from_millis(100));
        done_sender
            .send(())
            .expect("worker should report when sleep completes");
    });

    assert!(
        timeline.wait_for_blocked_waiters(
            MockWaiterKind::Sleep,
            1,
            Duration::from_secs(1)
        ),
        "worker should block in mock sleep before time advances",
    );

    timeline.advance(Duration::from_millis(99));
    assert!(
        done_receiver
            .recv_timeout(Duration::from_millis(20))
            .is_err(),
        "mock sleep should not complete before target elapsed time",
    );

    timeline.advance(Duration::from_millis(1));
    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("mock sleep should complete after target elapsed time");
    worker.join().expect("worker should finish cleanly");
}

/// Verifies sleep deadlines are relative to the call-time timeline instant.
///
/// # Errors
/// The test fails if the sleeper ignores elapsed time at call start.
#[test]
fn test_sleep_for_uses_timeline_elapsed_at_call_time() {
    let sleeper = MockSleeper::new();
    let timeline = sleeper.timeline();
    timeline.advance(Duration::from_millis(10));
    let worker_sleeper = sleeper.clone();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        worker_sleeper.sleep_for(Duration::from_millis(100));
        done_sender
            .send(())
            .expect("worker should report when sleep completes");
    });

    assert!(
        timeline.wait_for_blocked_waiters(
            MockWaiterKind::Sleep,
            1,
            Duration::from_secs(1)
        ),
        "worker should block in mock sleep before time advances",
    );

    timeline.advance(Duration::from_millis(99));
    assert!(
        done_receiver
            .recv_timeout(Duration::from_millis(20))
            .is_err(),
        "sleep should be relative to elapsed at call time",
    );

    timeline.advance(Duration::from_millis(1));
    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("sleep should complete after the full relative duration");
    worker.join().expect("worker should finish cleanly");
}
