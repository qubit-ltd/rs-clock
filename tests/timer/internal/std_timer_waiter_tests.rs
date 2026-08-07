// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::task::Context;
#[cfg(all(loom, feature = "loom-model"))]
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;
use std::time::Duration;

use qubit_clock::StdMonotonicClock;
use qubit_clock::StdTimer;
use qubit_clock::Timer;
#[cfg(all(loom, feature = "loom-model"))]
use qubit_clock::test_util::loom::LoomStdTimerWaiter;

/// Provides stable Waker identity without performing work when invoked.
struct NoopWake;

impl Wake for NoopWake {
    /// Ignores the notification.
    fn wake(self: Arc<Self>) {
        drop(self);
    }
}

#[test]
fn test_std_timer_waiter_retains_same_registered_waker() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let mut future = timer
        .after(Duration::from_secs(30))
        .expect("long deadline should register");
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);

    assert!(future.as_mut().poll(&mut context).is_pending());
    assert!(future.as_mut().poll(&mut context).is_pending());
}

/// Verifies polling concurrently with completion cannot lose the terminal
/// state or its registered Waker.
#[cfg(all(loom, feature = "loom-model"))]
#[test]
fn test_loom_std_timer_waiter_poll_races_with_complete() {
    loom::model(|| {
        let waiter = loom::sync::Arc::new(LoomStdTimerWaiter::new());
        let poll_waiter = loom::sync::Arc::clone(&waiter);
        let complete_waiter = loom::sync::Arc::clone(&waiter);
        let poller = loom::thread::spawn(move || {
            let waker = Waker::noop();
            let context = Context::from_waker(waker);
            poll_waiter.poll(&context)
        });
        let completer =
            loom::thread::spawn(move || complete_waiter.complete().is_some());

        let first_poll = poller.join().expect("poller should finish");
        let detached_waker = completer.join().expect("completer should finish");
        match first_poll {
            Poll::Pending => assert!(detached_waker),
            Poll::Ready(Ok(())) => assert!(!detached_waker),
            Poll::Ready(Err(())) => {
                panic!("completion race must not report worker failure");
            }
        }

        let waker = Waker::noop();
        let context = Context::from_waker(waker);
        assert_eq!(Poll::Ready(Ok(())), waiter.poll(&context));
        assert!(waiter.complete().is_none());
        assert!(waiter.fail().is_none());
    });
}

/// Verifies polling concurrently with worker failure cannot lose the terminal
/// state or its registered Waker.
#[cfg(all(loom, feature = "loom-model"))]
#[test]
fn test_loom_std_timer_waiter_poll_races_with_worker_failure() {
    loom::model(|| {
        let waiter = loom::sync::Arc::new(LoomStdTimerWaiter::new());
        let poll_waiter = loom::sync::Arc::clone(&waiter);
        let fail_waiter = loom::sync::Arc::clone(&waiter);
        let poller = loom::thread::spawn(move || {
            let waker = Waker::noop();
            let context = Context::from_waker(waker);
            poll_waiter.poll(&context)
        });
        let failure = loom::thread::spawn(move || fail_waiter.fail().is_some());

        let first_poll = poller.join().expect("poller should finish");
        let detached_waker =
            failure.join().expect("failure task should finish");
        match first_poll {
            Poll::Pending => assert!(detached_waker),
            Poll::Ready(Err(())) => assert!(!detached_waker),
            Poll::Ready(Ok(())) => {
                panic!("worker-failure race must not report completion");
            }
        }

        let waker = Waker::noop();
        let context = Context::from_waker(waker);
        assert_eq!(Poll::Ready(Err(())), waiter.poll(&context));
        assert!(waiter.fail().is_none());
        assert!(waiter.complete().is_none());
    });
}
