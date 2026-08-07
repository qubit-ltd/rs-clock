// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_clock::StdMonotonicClock;
use qubit_clock::StdTimer;
use qubit_clock::Timer;

use super::super::support::block_on_timer_future;

#[test]
fn test_std_timer_future_cancellation_does_not_block_later_registration() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let cancelled = timer
        .after(Duration::from_secs(30))
        .expect("long deadline should register");
    drop(cancelled);

    let future = timer
        .after(Duration::from_millis(5))
        .expect("later deadline should register after cancellation");
    block_on_timer_future(future);
}

#[test]
fn test_std_timer_future_latches_completion_before_first_poll() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_millis(2))
        .expect("short deadline should register");

    std::thread::sleep(Duration::from_millis(10));
    block_on_timer_future(future);
}

#[test]
fn test_std_timer_future_drop_after_scheduler_completion_is_harmless() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_millis(2))
        .expect("short deadline should register");

    std::thread::sleep(Duration::from_millis(10));
    drop(future);
}
