/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for shared mock timelines.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use qubit_clock::{
    MockTimeError,
    MockTimeline,
    MockWaiterKind,
};

/// Verifies default timelines start with zero elapsed time.
///
/// # Errors
/// The test fails if the default timeline starts advanced.
#[test]
fn test_default_starts_at_zero_elapsed() {
    let timeline = MockTimeline::default();

    assert_eq!(Duration::ZERO, timeline.elapsed());
}

/// Verifies independently created timelines receive unique ids.
///
/// # Errors
/// The test fails if timeline ids are reused.
#[test]
fn test_new_assigns_unique_timeline_ids() {
    let first = MockTimeline::new();
    let second = MockTimeline::new();
    let first_clone = first.clone();

    assert_ne!(first.id(), second.id());
    assert_eq!(first.id(), first_clone.id());
}

/// Verifies elapsed conversion saturates when internal nanoseconds exceed `Duration`.
///
/// # Errors
/// The test fails if elapsed conversion overflows instead of saturating.
#[test]
fn test_elapsed_saturates_to_duration_max() {
    let timeline = MockTimeline::new();

    timeline.advance(Duration::MAX);
    timeline.advance(Duration::MAX);

    assert_eq!(Duration::MAX, timeline.elapsed());
}

/// Verifies external notifications wake event waiters without advancing time.
///
/// # Errors
/// The test fails if event waiters are not notified or elapsed time changes.
#[test]
fn test_notify_external_change_wakes_event_waiter_without_advancing_time() {
    let timeline = MockTimeline::new();
    let observed_epoch = timeline.event_epoch();
    let worker_timeline = timeline.clone();
    let (ready_sender, ready_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        ready_sender
            .send(())
            .expect("test should observe event waiter startup");
        worker_timeline.wait_for_event_after(observed_epoch);
        worker_timeline.event_epoch()
    });

    ready_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("event waiter should start");
    timeline.notify_external_change();

    assert_eq!(
        observed_epoch.wrapping_add(1),
        worker.join().expect("event waiter should finish"),
    );
    assert_eq!(Duration::ZERO, timeline.elapsed());
}

/// Verifies deadline waiters block until the timeline reaches the deadline.
///
/// # Errors
/// The test fails if deadline waits complete too early or never complete.
#[test]
fn test_wait_for_blocks_until_timeline_reaches_deadline() {
    let timeline = MockTimeline::new();
    let worker_timeline = timeline.clone();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        worker_timeline.wait_for(Duration::from_millis(100));
        done_sender
            .send(())
            .expect("test should receive deadline completion");
    });

    assert!(
        timeline.wait_for_blocked_waiters(MockWaiterKind::Deadline, 1, Duration::from_secs(1),),
        "deadline waiter should register before time advances",
    );
    assert_eq!(Err(MockTimeError::ActiveWaiters), timeline.reset());

    timeline.advance(Duration::from_millis(99));
    assert!(
        done_receiver
            .recv_timeout(Duration::from_millis(20))
            .is_err(),
        "deadline wait should not complete before target elapsed time",
    );

    timeline.advance(Duration::from_millis(1));
    done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("deadline wait should complete after target elapsed time");
    worker
        .join()
        .expect("deadline waiter should finish cleanly");
}

/// Verifies wait-until returns immediately for a reached deadline.
///
/// # Errors
/// The test fails if an already reached deadline blocks.
#[test]
fn test_wait_until_returns_immediately_for_reached_deadline() {
    let timeline = MockTimeline::new();
    let deadline = timeline.now();

    timeline
        .wait_until(deadline)
        .expect("deadline should belong to its timeline");

    assert_eq!(Duration::ZERO, timeline.elapsed());
}

/// Verifies deadline waits reject instants from another timeline.
///
/// # Errors
/// The test fails if a foreign instant is accepted as a local deadline.
#[test]
fn test_wait_until_rejects_foreign_timeline_deadline() {
    let timeline = MockTimeline::new();
    let foreign_timeline = MockTimeline::new();
    let foreign_deadline = foreign_timeline.now();

    assert_eq!(
        Err(MockTimeError::MismatchedTimeline {
            expected: timeline.id(),
            actual: foreign_timeline.id(),
        }),
        timeline.wait_until(foreign_deadline),
    );
}

/// Verifies instant ordering is defined only within the same timeline.
///
/// # Errors
/// The test fails if foreign instants are ordered as if they shared a timeline.
#[test]
fn test_mock_instant_partial_order_requires_same_timeline() {
    let timeline = MockTimeline::new();
    let first = timeline.now();
    timeline.advance(Duration::from_nanos(1));
    let second = timeline.now();
    let foreign = MockTimeline::new().now();

    assert!(first < second);
    assert_eq!(None, first.partial_cmp(&foreign));
}

/// Verifies waiter observation returns false when the real timeout is elapsed.
///
/// # Errors
/// The test fails if waiter observation waits forever when no waiter exists.
#[test]
fn test_wait_for_blocked_waiters_returns_false_after_timeout() {
    let timeline = MockTimeline::new();

    assert!(!timeline.wait_for_blocked_waiters(MockWaiterKind::Sleep, 1, Duration::ZERO,));
    assert!(
        !timeline.wait_for_blocked_waiters(MockWaiterKind::Sleep, 1, Duration::from_millis(1),)
    );
}

/// Verifies waiter observation returns false when the real deadline cannot exist.
///
/// # Errors
/// The test fails if an unrepresentable real timeout is not rejected.
#[test]
fn test_wait_for_blocked_waiters_rejects_unrepresentable_timeout() {
    let timeline = MockTimeline::new();

    assert!(!timeline.wait_for_blocked_waiters(MockWaiterKind::Sleep, 1, Duration::MAX,));
}
