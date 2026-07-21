// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    Timer,
};
use std::sync::{
    Arc,
    atomic::{
        AtomicUsize,
        Ordering,
    },
};
use std::task::{
    Context,
    Wake,
    Waker,
};
use std::time::Duration;

#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn test_manual_advance_effects_wake_every_due_waiter() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let first_counter = Arc::new(WakeCounter::default());
    let second_counter = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_counter));
    let second_waker = Waker::from(Arc::clone(&second_counter));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    let mut first = timer
        .after(Duration::from_secs(2))
        .expect("first deadline should register");
    let mut second = timer
        .after(Duration::from_secs(2))
        .expect("second deadline should register");
    assert!(first.as_mut().poll(&mut first_context).is_pending());
    assert!(second.as_mut().poll(&mut second_context).is_pending());

    clock
        .advance(Duration::from_secs(2))
        .expect("manual time should advance");

    assert_eq!(1, first_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, second_counter.0.load(Ordering::Relaxed));
}
