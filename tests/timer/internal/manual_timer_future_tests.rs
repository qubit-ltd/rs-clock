// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    ManualTimer,
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
    Poll,
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
fn test_manual_timer_future_registers_before_first_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let future = timer
        .after(Duration::from_secs(4))
        .expect("manual timer registration should succeed");

    assert_eq!(1, clock.pending_waiters());
    assert_eq!(
        Some(Duration::from_secs(4)),
        clock
            .next_deadline()
            .map(|value| value.elapsed_since_origin()),
    );
    drop(future);
    assert_eq!(0, clock.pending_waiters());
}

#[tokio::test]
async fn test_manual_timer_future_latches_completion_before_first_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let future = timer
        .after(Duration::from_secs(2))
        .expect("manual timer registration should succeed");

    clock
        .advance(Duration::from_secs(2))
        .expect("manual time should advance");
    future.await.expect("manual timer should complete");
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_timer_future_replaces_registered_waker() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let mut future = timer
        .after(Duration::from_secs(3))
        .expect("manual timer registration should succeed");
    let first_counter = Arc::new(WakeCounter::default());
    let second_counter = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_counter));
    let second_waker = Waker::from(Arc::clone(&second_counter));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    assert!(future.as_mut().poll(&mut first_context).is_pending());
    assert!(future.as_mut().poll(&mut second_context).is_pending());

    clock
        .advance(Duration::from_secs(3))
        .expect("manual time should advance");

    assert_eq!(0, first_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, second_counter.0.load(Ordering::Relaxed));
    assert!(matches!(
        future.as_mut().poll(&mut second_context),
        Poll::Ready(Ok(()))
    ));
}
