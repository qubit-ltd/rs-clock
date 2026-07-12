// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    AsyncSleeper,
    ManualAsyncSleeper,
    ManualMonotonicClock,
};
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use std::task::{
    Context,
    Poll,
    Wake,
    Waker,
};
use std::time::Duration;

#[derive(Default)]
struct WakeCounter {
    wakes: AtomicUsize,
}

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_manual_waiter_future_wakes_when_expected_waiter_registers() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut waiter =
        pin!(ManualMonotonicClock::wait_for_waiters_async(&clock, 1,));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));

    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
    drop(sleep);
}

#[test]
fn test_manual_waiter_future_replaces_registered_waker() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let first_counter = Arc::new(WakeCounter::default());
    let second_counter = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_counter));
    let second_waker = Waker::from(Arc::clone(&second_counter));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    let mut waiter =
        Box::pin(ManualMonotonicClock::wait_for_waiters_async(&clock, 1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut first_context));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut second_context));

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));

    assert_eq!(0, first_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(1, second_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut second_context));
    drop(sleep);
}

#[test]
fn test_manual_waiter_future_unregisters_on_drop() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut waiter =
        Box::pin(ManualMonotonicClock::wait_for_waiters_async(&clock, 1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    drop(waiter);

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));

    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
    drop(sleep);
}
