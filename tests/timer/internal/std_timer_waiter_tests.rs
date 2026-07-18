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
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;

#[test]
fn test_std_timer_waiter_retains_same_registered_waker() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let mut future = timer
        .after(Duration::from_secs(30))
        .expect("long deadline should register");
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Pending, future.as_mut().poll(&mut context));
    assert_eq!(Poll::Pending, future.as_mut().poll(&mut context));
}
