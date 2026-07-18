// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    StdMonotonicClock,
    StdTimer,
    Timer,
};
use std::time::{
    Duration,
    Instant,
};

use super::block_on_timer_future;

#[test]
fn test_std_timer_scheduler_state_wakes_for_new_earlier_deadline() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let later = timer
        .after(Duration::from_millis(250))
        .expect("later deadline should register");
    let started = Instant::now();
    let earlier = timer
        .after(Duration::from_millis(5))
        .expect("earlier deadline should register");

    block_on_timer_future(earlier);

    assert!(started.elapsed() < Duration::from_millis(150));
    drop(later);
}

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
