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
    MonotonicClock,
    TimeError,
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

struct PanicWaker;

impl Wake for PanicWaker {
    fn wake(self: Arc<Self>) {
        panic!("timer waker panic");
    }
}

#[test]
fn test_manual_timer_registers_before_first_poll() {
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
async fn test_manual_timer_latches_completion_before_first_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let future = timer
        .after(Duration::from_secs(2))
        .expect("manual timer registration should succeed");

    clock
        .advance(Duration::from_secs(2))
        .expect("manual time should advance");
    future.await;

    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_timer_returns_ready_future_for_reached_deadline() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let mut future = timer
        .at(clock.now())
        .expect("reached deadline should register successfully");
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(()),
    ));
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_timer_rejects_foreign_deadline_at_registration() {
    let clock = ManualMonotonicClock::new_shared();
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();
    let timer = ManualTimer::from_clock(clock.as_ref());

    let error = match timer.at(foreign) {
        Ok(_) => panic!("foreign deadline should be rejected immediately"),
        Err(error) => error,
    };

    assert_eq!(
        TimeError::ClockDomainMismatch {
            expected,
            actual: foreign.domain(),
        },
        error,
    );
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_timer_replaces_registered_waker() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let mut future = timer
        .after(Duration::from_secs(3))
        .expect("manual timer registration should succeed");
    let first_counter = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_counter));
    let mut first_context = Context::from_waker(&first_waker);
    let second_counter = Arc::new(WakeCounter::default());
    let second_waker = Waker::from(Arc::clone(&second_counter));
    let mut second_context = Context::from_waker(&second_waker);

    assert!(matches!(
        future.as_mut().poll(&mut first_context),
        Poll::Pending,
    ));
    assert!(matches!(
        future.as_mut().poll(&mut second_context),
        Poll::Pending,
    ));
    clock
        .advance(Duration::from_secs(3))
        .expect("manual time should advance");

    assert_eq!(0, first_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, second_counter.0.load(Ordering::Relaxed));
    assert!(matches!(
        future.as_mut().poll(&mut second_context),
        Poll::Ready(()),
    ));
}

#[test]
fn test_manual_timer_retains_same_domain_after_source_clock_is_dropped() {
    let (timer, expected_domain) = {
        let clock = ManualMonotonicClock::new();
        let expected_domain = clock.now().domain();
        (ManualTimer::from_clock(&clock), expected_domain)
    };

    assert_eq!(expected_domain, timer.clock().now().domain());
}

#[test]
fn test_manual_timer_attempts_all_panicking_due_wakers() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let mut first = timer
        .after(Duration::from_secs(1))
        .expect("first deadline should register");
    let mut second = timer
        .after(Duration::from_secs(1))
        .expect("second deadline should register");
    let first_waker = Waker::from(Arc::new(PanicWaker));
    let second_waker = Waker::from(Arc::new(PanicWaker));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    assert_eq!(Poll::Pending, first.as_mut().poll(&mut first_context));
    assert_eq!(Poll::Pending, second.as_mut().poll(&mut second_context));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        clock.advance(Duration::from_secs(1))
    }));

    assert!(result.is_err());
}
