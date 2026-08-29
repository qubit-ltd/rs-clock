// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

use qubit_clock::StdMonotonicClock;
use qubit_clock::StdTimer;
use qubit_clock::Timer;

use super::super::support::block_on_timer_future;

/// Delay retained by the scheduler before the earlier registration arrives.
const LATER_DEADLINE_DELAY: Duration = Duration::from_secs(30);

/// Delay used by the registration that must interrupt the worker wait.
const EARLIER_DEADLINE_DELAY: Duration = Duration::from_millis(10);

/// Generous real-time guard that detects a worker left on the later deadline.
const EARLIER_COMPLETION_GUARD: Duration = Duration::from_secs(5);

/// Verifies that a newly registered earlier deadline interrupts the worker's
/// existing wait for a much later deadline.
#[test]
fn test_std_timer_scheduler_state_wakes_for_new_earlier_deadline() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let later = timer
        .after(LATER_DEADLINE_DELAY)
        .expect("later deadline should register");
    let earlier = timer
        .after(EARLIER_DEADLINE_DELAY)
        .expect("earlier deadline should register");
    let (completion_sender, completion_receiver) = sync_channel(1);
    let waiter = thread::spawn(move || {
        block_on_timer_future(earlier);
        let _ = completion_sender.send(());
    });

    let completion = completion_receiver.recv_timeout(EARLIER_COMPLETION_GUARD);
    drop(later);

    completion.expect("earlier deadline should complete before liveness guard");
    waiter.join().expect("earlier deadline waiter should finish");
}

/// Verifies that the shared scheduler completes a batch of equal deadlines.
#[test]
fn test_std_timer_scheduler_state_completes_many_deadlines() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let futures = (0..32)
        .map(|_| timer.after(Duration::from_millis(5)))
        .collect::<Result<Vec<_>, _>>()
        .expect("all deadlines should register");

    futures.into_iter().for_each(block_on_timer_future);
}
