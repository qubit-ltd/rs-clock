// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::future::Future;
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

struct PanicWaker;

impl Wake for PanicWaker {
    fn wake(self: Arc<Self>) {
        panic!("observer waker panic");
    }
}

#[test]
fn test_waiter_registration_guard_rolls_back_after_observer_waker_panics() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let panic_waker = Waker::from(Arc::new(PanicWaker));
    let wake_counter = Arc::new(WakeCounter::default());
    let counting_waker = Waker::from(Arc::clone(&wake_counter));
    let mut panic_context = Context::from_waker(&panic_waker);
    let mut counting_context = Context::from_waker(&counting_waker);
    let mut panic_observer = Box::pin(clock.wait_for_waiters_async(1));
    let mut counting_observer = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(
        Poll::Pending,
        panic_observer.as_mut().poll(&mut panic_context)
    );
    assert_eq!(
        Poll::Pending,
        counting_observer.as_mut().poll(&mut counting_context),
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        timer.after(Duration::from_secs(1))
    }));

    assert!(result.is_err());
    assert_eq!(1, wake_counter.0.load(Ordering::Relaxed));
    assert_eq!(0, clock.pending_waiters());
    assert_eq!(
        Poll::Ready(()),
        panic_observer.as_mut().poll(&mut panic_context)
    );
    assert_eq!(
        Poll::Ready(()),
        counting_observer.as_mut().poll(&mut counting_context),
    );
}
