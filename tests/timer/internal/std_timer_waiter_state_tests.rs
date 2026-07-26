// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{StdMonotonicClock, StdTimer, Timer};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn test_std_timer_waiter_state_replaces_registered_waker() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let mut future = timer
        .after(Duration::from_millis(10))
        .expect("short deadline should register");
    let first_counter = Arc::new(WakeCounter::default());
    let second_counter = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_counter));
    let second_waker = Waker::from(Arc::clone(&second_counter));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    assert!(future.as_mut().poll(&mut first_context).is_pending());
    assert!(future.as_mut().poll(&mut second_context).is_pending());

    std::thread::sleep(Duration::from_millis(30));

    assert_eq!(0, first_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, second_counter.0.load(Ordering::Relaxed));
    assert!(matches!(
        future.as_mut().poll(&mut second_context),
        Poll::Ready(Ok(()))
    ));
}
