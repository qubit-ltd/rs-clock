/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Integration tests for the unified mock time runtime.

use std::sync::mpsc;
use std::thread;
use std::time::Duration as StdDuration;

use chrono::{
    DateTime,
    Duration,
    Utc,
};
use qubit_clock::meter::NanoTimeMeter;
use qubit_clock::sleep::Sleeper;
use qubit_clock::{
    Clock,
    MockTime,
    MockTimeError,
    MockWaiterKind,
    NanoClock,
};

/// Parses a fixed UTC timestamp used by mock-time tests.
///
/// # Returns
/// A UTC timestamp with nanosecond precision.
fn fixed_time() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-01-01T00:00:00.000000123Z")
        .expect("fixed timestamp should parse")
        .with_timezone(&Utc)
}

/// Verifies the Unix epoch runtime exposes a zero-elapsed shared timeline.
///
/// # Errors
/// The test fails if the facade accessors do not share the expected initial state.
#[test]
fn test_unix_epoch_starts_at_epoch_and_zero_elapsed() {
    let mock = MockTime::unix_epoch();

    assert_eq!(DateTime::<Utc>::UNIX_EPOCH, mock.clock().time());
    assert_eq!(StdDuration::ZERO, mock.elapsed());
    assert_eq!(StdDuration::ZERO, mock.timeline().elapsed());
}

/// Verifies one mock time advance drives clock reads and sleeper completion.
///
/// # Errors
/// The test fails if the clock and sleeper do not share the same timeline.
#[test]
fn test_mock_time_advance_drives_clock_and_sleeper() {
    let mock = MockTime::at(fixed_time());
    let clock = mock.clock();
    let sleeper = mock.sleeper();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        sleeper.sleep_for(StdDuration::from_millis(100));
        done_sender
            .send(clock.time())
            .expect("test should receive sleeper completion time");
    });

    assert!(
        mock.timeline().wait_for_blocked_waiters(
            MockWaiterKind::Sleep,
            1,
            StdDuration::from_secs(1),
        ),
        "sleeper should be blocked before mock time advances",
    );

    mock.advance(StdDuration::from_millis(99));
    assert!(
        done_receiver
            .recv_timeout(StdDuration::from_millis(20))
            .is_err(),
        "sleep should not complete before the shared timeline reaches the deadline",
    );

    mock.advance(StdDuration::from_millis(1));
    assert_eq!(
        done_receiver
            .recv_timeout(StdDuration::from_secs(1))
            .expect("mock sleep should complete after timeline advances"),
        fixed_time() + Duration::milliseconds(100),
    );
    worker.join().expect("worker should finish cleanly");
}

/// Verifies `MockClock` exposes nanosecond precision through the shared timeline.
///
/// # Errors
/// The test fails if `MockClock` no longer implements [`NanoClock`] semantics.
#[test]
fn test_mock_clock_is_nanosecond_clock_backed_by_timeline() {
    let mock = MockTime::at(fixed_time());
    let clock = mock.clock();
    let start = clock.nanos();

    mock.advance(StdDuration::from_nanos(1_500));

    assert_eq!(clock.nanos() - start, 1_500);
}

/// Verifies existing nano meters can use the unified `MockClock` directly.
///
/// # Errors
/// The test fails if meter elapsed time does not follow the mock timeline.
#[test]
fn test_nano_time_meter_uses_mock_clock_timeline() {
    let mock = MockTime::at(fixed_time());
    let mut meter = NanoTimeMeter::with_clock(mock.clock());

    meter.start();
    mock.advance(StdDuration::from_nanos(2_250));
    meter.stop();

    assert_eq!(meter.nanos(), 2_250);
}

/// Verifies setting time reanchors the clock without changing elapsed time.
///
/// # Errors
/// The test fails if `set_time` advances the timeline or ignores the new anchor.
#[test]
fn test_set_time_reanchors_clock_without_changing_elapsed() {
    let mock = MockTime::unix_epoch();
    mock.advance(StdDuration::from_secs(10));
    let new_time = fixed_time();

    mock.set_time(new_time);

    assert_eq!(StdDuration::from_secs(10), mock.elapsed());
    assert_eq!(new_time, mock.clock().time());
}

/// Verifies reset restores the runtime's initial timeline and wall-clock anchor.
///
/// # Errors
/// The test fails if reset leaves elapsed time or clock reads advanced.
#[test]
fn test_reset_restores_initial_state() {
    let mock = MockTime::at(fixed_time());

    mock.advance(StdDuration::from_secs(10));
    mock.set_time(fixed_time() + Duration::hours(1));
    mock.reset()
        .expect("mock runtime without waiters should reset");

    assert_eq!(StdDuration::ZERO, mock.elapsed());
    assert_eq!(fixed_time(), mock.clock().time());
}

/// Verifies reset is rejected while the runtime has active waiters.
///
/// # Errors
/// The test fails if reset rewinds the timeline under an active sleeper.
#[test]
fn test_reset_rejects_active_waiters() {
    let mock = MockTime::unix_epoch();
    let sleeper = mock.sleeper();
    let (done_sender, done_receiver) = mpsc::channel();

    let worker = thread::spawn(move || {
        sleeper.sleep_for(StdDuration::from_millis(100));
        done_sender
            .send(())
            .expect("test should receive sleeper completion");
    });

    assert!(
        mock.timeline().wait_for_blocked_waiters(
            MockWaiterKind::Sleep,
            1,
            StdDuration::from_secs(1),
        ),
        "sleeper should be blocked before reset is attempted",
    );
    assert_eq!(Err(MockTimeError::ActiveWaiters), mock.reset());

    mock.advance(StdDuration::from_millis(100));
    done_receiver
        .recv_timeout(StdDuration::from_secs(1))
        .expect("sleeper should complete after mock time advances");
    worker.join().expect("sleeper worker should finish");
}
