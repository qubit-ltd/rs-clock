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

/// Counts task wake requests issued by the manual clock.
#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    /// Records one wake request.
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn test_manual_sleep_future_drop_unregisters_waiter() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let future = sleeper.sleep_for_async(Duration::from_secs(30));

    assert_eq!(1, clock.pending_waiters());
    drop(future);
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_sleep_future_reuses_registered_waker() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut future = sleeper.sleep_for_async(Duration::from_secs(30));
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
}

#[test]
fn test_manual_sleep_future_is_woken_once_after_deadline_is_reached() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut future = sleeper.sleep_for_async(Duration::from_secs(5));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    clock
        .advance(Duration::from_secs(5))
        .expect("deadline advance should succeed");
    assert_eq!(1, wake_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, clock.pending_waiters());

    clock
        .advance(Duration::from_secs(1))
        .expect("post-deadline advance should succeed");
    assert_eq!(1, wake_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, clock.pending_waiters());

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(Ok(())),
    ));
    assert_eq!(0, clock.pending_waiters());
}
