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
use std::task::Wake;
use std::task::Waker;
use std::time::Duration;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::ManualTimer;
use qubit_clock::Timer;

struct PanicWaker {
    attempts: Arc<AtomicUsize>,
}

impl Wake for PanicWaker {
    fn wake(self: Arc<Self>) {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        panic!("timer waker panic");
    }
}

#[test]
fn test_panic_fanout_attempts_every_due_waker_before_resuming_panic() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut first = timer
        .after(Duration::from_secs(1))
        .expect("first deadline should register");
    let mut second = timer
        .after(Duration::from_secs(1))
        .expect("second deadline should register");
    let first_waker = Waker::from(Arc::new(PanicWaker {
        attempts: Arc::clone(&attempts),
    }));
    let second_waker = Waker::from(Arc::new(PanicWaker {
        attempts: Arc::clone(&attempts),
    }));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    assert!(first.as_mut().poll(&mut first_context).is_pending());
    assert!(second.as_mut().poll(&mut second_context).is_pending());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| clock.advance(Duration::from_secs(1))));

    assert!(result.is_err());
    assert_eq!(2, attempts.load(Ordering::Relaxed));
}
