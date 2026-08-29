// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use qubit_clock::BlockingSleeper;
use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::MonotonicInstant;
use qubit_clock::TimeError;
use qubit_clock::Timer;
use qubit_clock::TimerFuture;

/// Returns pending once after synchronously waking its caller.
struct WakeBeforeParkFuture {
    /// Records whether the initial pending poll has completed.
    polled: bool,
}

impl Future for WakeBeforeParkFuture {
    type Output = Result<(), TimeError>;

    /// Wakes the caller before returning pending on the first poll.
    ///
    /// # Parameters
    ///
    /// * `context` - Provides the waker that is notified before parking.
    ///
    /// # Returns
    ///
    /// Returns pending once and then completes successfully.
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.polled {
            Poll::Ready(Ok(()))
        } else {
            self.polled = true;
            #[allow(clippy::waker_clone_wake)]
            context.waker().clone().wake();
            Poll::Pending
        }
    }
}

/// Produces futures that wake before the sleeper can park its current thread.
struct WakeBeforeParkTimer {
    /// Supplies the deadline used by the blocking sleeper.
    clock: ManualMonotonicClock,
}

impl Timer for WakeBeforeParkTimer {
    /// Returns the clock that defines this timer's deadlines.
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Creates a future that wakes its caller before returning pending.
    ///
    /// # Parameters
    ///
    /// * `_deadline` - The deadline requested by the blocking sleeper.
    ///
    /// # Returns
    ///
    /// Returns a future that completes after the blocking sleeper observes the
    /// latched wake-up.
    fn at(&self, _deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        Ok(Box::pin(WakeBeforeParkFuture { polled: false }))
    }
}

/// Verifies that a wake issued before parking triggers an immediate repoll.
#[test]
fn test_thread_waker_handles_wake_before_park() {
    let clock = ManualMonotonicClock::new();
    let deadline = clock.now();
    let timer = Arc::new(WakeBeforeParkTimer { clock });
    let sleeper = BlockingSleeper::new(timer);

    sleeper
        .sleep_until(deadline)
        .expect("latched wake should cause an immediate repoll");
}
