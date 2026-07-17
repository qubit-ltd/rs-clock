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
    StdMonotonicClock,
    StdTimer,
    TimeError,
    Timer,
};
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::{
    Duration,
    Instant,
};

use super::internal::{
    DestructorPanickingWaker,
    PanickingWaker,
    block_on_timer_future,
};

#[test]
fn test_std_timer_returns_ready_future_for_reached_deadline() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let deadline = clock.now();
    std::thread::sleep(Duration::from_millis(1));
    let future = timer
        .at(deadline)
        .expect("reached deadline should register successfully");

    block_on_timer_future(future);
}

#[test]
fn test_std_timer_completes_real_short_deadline() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_millis(5))
        .expect("short deadline should register");

    block_on_timer_future(future);
}

#[test]
fn test_std_timer_continues_after_registered_waker_panics() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let mut panicking = timer
        .after(Duration::from_millis(10))
        .expect("panicking deadline should register");
    let waker = Waker::from(Arc::new(PanickingWaker));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Poll::Pending, panicking.as_mut().poll(&mut context));

    let survivor = timer
        .after(Duration::from_millis(30))
        .expect("surviving deadline should register");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        block_on_timer_future(survivor);
        sender.send(()).expect("receiver should remain available");
    });

    assert_eq!(Ok(()), receiver.recv_timeout(Duration::from_secs(1)),);
}

#[test]
fn test_std_timer_survives_panicking_waker_payload_destructor() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let mut panicking = timer
        .after(Duration::from_millis(10))
        .expect("panicking deadline should register");
    let waker = Waker::from(Arc::new(DestructorPanickingWaker));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Poll::Pending, panicking.as_mut().poll(&mut context));

    let survivor = timer
        .after(Duration::from_millis(30))
        .expect("surviving deadline should register");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        block_on_timer_future(survivor);
        sender.send(()).expect("receiver should remain available");
    });

    assert_eq!(Ok(()), receiver.recv_timeout(Duration::from_secs(1)),);
}

#[test]
fn test_std_timer_cancellation_does_not_block_later_registration() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let cancelled = timer
        .after(Duration::from_secs(30))
        .expect("long deadline should register");
    drop(cancelled);

    let future = timer
        .after(Duration::from_millis(5))
        .expect("later deadline should register after cancellation");
    block_on_timer_future(future);
}

#[test]
fn test_std_timer_handles_cancellation_churn_behind_active_anchor() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let anchor = timer
        .after(Duration::from_secs(30))
        .expect("anchor deadline should register");
    for offset in 0..4096_u64 {
        let cancelled = timer
            .after(Duration::from_secs(31 + offset))
            .expect("churn deadline should register");
        drop(cancelled);
    }
    drop(anchor);

    let survivor = timer
        .after(Duration::from_millis(5))
        .expect("post-churn deadline should register");
    block_on_timer_future(survivor);
}

#[test]
fn test_std_timer_wakes_scheduler_for_new_earlier_deadline() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let later = timer
        .after(Duration::from_millis(250))
        .expect("later deadline should register");
    let started = Instant::now();
    let earlier = timer
        .after(Duration::from_millis(5))
        .expect("earlier deadline should register");

    block_on_timer_future(earlier);

    assert!(started.elapsed() < Duration::from_millis(150));
    drop(later);
}

#[test]
fn test_std_timer_completes_many_deadlines_with_one_scheduler() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let futures = (0..32)
        .map(|_| timer.after(Duration::from_millis(5)))
        .collect::<Result<Vec<_>, _>>()
        .expect("all deadlines should register");

    futures.into_iter().for_each(block_on_timer_future);
}

#[test]
fn test_std_timer_reuses_worker_during_idle_grace_period() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    for _ in 0..64 {
        let future = timer
            .after(Duration::from_micros(100))
            .expect("short deadline should register");
        block_on_timer_future(future);
    }
}

#[test]
fn test_std_timer_rejects_foreign_deadline_immediately() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();

    let error = match timer.at(foreign) {
        Ok(_) => panic!("foreign deadline should fail at registration"),
        Err(error) => error,
    };

    assert_eq!(
        TimeError::ClockDomainMismatch {
            expected,
            actual: foreign.domain(),
        },
        error,
    );
}

#[test]
fn test_std_timer_retains_clock_domain_after_source_is_dropped() {
    let (timer, domain) = {
        let clock = StdMonotonicClock::new();
        let domain = clock.now().domain();
        (StdTimer::from_clock(&clock), domain)
    };

    assert_eq!(domain, timer.clock().now().domain());
    assert!(format!("{timer:?}").starts_with("StdTimer"));
}

#[test]
fn test_std_timer_latches_completion_before_first_poll() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_millis(2))
        .expect("short deadline should register");

    std::thread::sleep(Duration::from_millis(10));
    block_on_timer_future(future);
}

#[test]
fn test_std_timer_retains_same_registered_waker() {
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

#[test]
fn test_std_timer_drop_after_scheduler_completion_is_harmless() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_millis(2))
        .expect("short deadline should register");

    std::thread::sleep(Duration::from_millis(10));
    drop(future);
}
