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
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;

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
