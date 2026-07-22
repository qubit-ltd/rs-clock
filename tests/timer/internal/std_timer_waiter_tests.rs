// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    StdMonotonicClock,
    StdTimer,
    Timer,
};
use std::sync::Arc;
use std::task::{
    Context,
    Wake,
    Waker,
};
use std::time::Duration;

/// Provides stable Waker identity without performing work when invoked.
struct NoopWake;

#[allow(clippy::manual_noop_waker)]
impl Wake for NoopWake {
    /// Ignores the notification.
    fn wake(self: Arc<Self>) {}
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
