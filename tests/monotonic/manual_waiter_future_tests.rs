// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    AsyncSleeper,
    BlockingSleeper,
    ManualAsyncSleeper,
    ManualBlockingSleeper,
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
use std::thread;
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
fn test_manual_waiter_future_is_ready_for_zero_expected_count() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let mut waiter = pin!(clock.wait_for_waiters_async(0));
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
}

#[test]
fn test_manual_waiter_future_is_ready_when_count_is_already_satisfied() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));
    let mut waiter = pin!(clock.wait_for_waiters_async(1));
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
    drop(sleep);
}

#[test]
fn test_manual_waiter_future_observes_blocking_waiter_registration() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));

    let worker = thread::spawn(move || {
        sleeper
            .sleep_for(Duration::from_secs(1))
            .expect("manual blocking sleep should complete");
    });
    assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
    clock
        .advance(Duration::from_secs(1))
        .expect("short manual advance should succeed");
    worker.join().expect("blocking waiter should finish");

    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
}

#[test]
fn test_manual_waiter_future_wakes_when_expected_waiter_registers() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut waiter = pin!(clock.wait_for_waiters_async(1));
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
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut first_context));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut second_context));

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));

    assert_eq!(0, first_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(1, second_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut second_context));
    drop(sleep);
}

/// Verifies that polling with the same waker keeps the observer pending
/// without replacing its registration.
#[test]
fn test_manual_waiter_future_reuses_registered_waker() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));

    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
}

#[test]
fn test_manual_waiter_future_unregisters_on_drop() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    drop(waiter);

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));

    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
    drop(sleep);
}

#[test]
fn test_manual_waiter_future_latches_reached_count_before_waiter_drops() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));
    drop(sleep);

    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
}
