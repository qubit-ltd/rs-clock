// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::Poll;
use std::task::Wake;
use std::task::Waker;
use std::time::Duration;
use std::time::Instant;

use qubit_clock::StdMonotonicClock;
use qubit_clock::StdTimer;
use qubit_clock::Timer;

/// Deadline used when observing replacement of a pending standard Timer Waker.
const REPLACEMENT_DEADLINE: Duration = Duration::from_millis(50);

/// Number of attempts allowed when a deadline completes before both polls run.
const REPLACEMENT_ATTEMPTS: usize = 4;

/// Maximum time allowed for the scheduler worker to invoke the retained Waker.
const REPLACEMENT_GUARD: Duration = Duration::from_secs(2);

#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn test_std_timer_waiter_state_replaces_registered_waker() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    for _ in 0..REPLACEMENT_ATTEMPTS {
        let mut future = timer
            .after(REPLACEMENT_DEADLINE)
            .expect("short deadline should register");
        let first_counter = Arc::new(WakeCounter::default());
        let second_counter = Arc::new(WakeCounter::default());
        let first_waker = Waker::from(Arc::clone(&first_counter));
        let second_waker = Waker::from(Arc::clone(&second_counter));
        let mut first_context = Context::from_waker(&first_waker);
        let mut second_context = Context::from_waker(&second_waker);
        let first_poll = future.as_mut().poll(&mut first_context);
        let second_poll = future.as_mut().poll(&mut second_context);

        if first_poll.is_ready() || second_poll.is_ready() {
            continue;
        }

        let started = Instant::now();
        while second_counter.0.load(Ordering::Relaxed) == 0
            && started.elapsed() < REPLACEMENT_GUARD
        {
            std::thread::sleep(Duration::from_millis(1));
        }

        assert_eq!(0, first_counter.0.load(Ordering::Relaxed));
        assert_eq!(1, second_counter.0.load(Ordering::Relaxed));
        assert!(matches!(
            future.as_mut().poll(&mut second_context),
            Poll::Ready(Ok(()))
        ));
        return;
    }

    panic!("standard Timer waiter should be observable before its deadline");
}
