// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    BlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
    TimeError,
    Timer,
    TimerFuture,
    TimerUnavailableReason,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_blocking_sleeper_uses_manual_timer_without_real_delay() {
    let clock = ManualMonotonicClock::new_shared();
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let worker =
        thread::spawn(move || sleeper.sleep_for(Duration::from_secs(16)));

    assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
    let _reached = clock
        .advance_to_next_deadline()
        .expect("blocking deadline should exist");
    worker
        .join()
        .expect("blocking worker should finish")
        .expect("blocking sleep should succeed");
}

#[test]
fn test_blocking_sleeper_uses_standard_timer() {
    let clock = StdMonotonicClock::new();
    let sleeper = BlockingSleeper::new(clock.new_timer());

    sleeper
        .sleep_for(Duration::from_millis(2))
        .expect("standard timer should complete");
}

#[test]
fn test_blocking_sleeper_exposes_composed_timer() {
    let clock = ManualMonotonicClock::new();
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let cloned = sleeper.clone();

    assert_eq!(clock.now().domain(), sleeper.timer().clock().now().domain(),);
    assert_eq!(clock.now().domain(), cloned.timer().clock().now().domain(),);
    assert!(format!("{sleeper:?}").starts_with("BlockingSleeper"));
}

struct WakeBeforeParkFuture {
    polled: bool,
}

impl Future for WakeBeforeParkFuture {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.polled {
            Poll::Ready(())
        } else {
            this.polled = true;
            // Exercise the consuming Wake path; standard Timer tests cover
            // borrowed Waker notifications.
            #[allow(clippy::waker_clone_wake)]
            context.waker().clone().wake();
            Poll::Pending
        }
    }
}

struct WakeBeforeParkTimer {
    clock: ManualMonotonicClock,
}

impl Timer for WakeBeforeParkTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Ok(Box::pin(WakeBeforeParkFuture { polled: false }))
    }
}

#[test]
fn test_blocking_sleeper_handles_wake_before_park() {
    let clock = ManualMonotonicClock::new();
    let deadline = clock.now();
    let timer = Arc::new(WakeBeforeParkTimer { clock });
    let sleeper = BlockingSleeper::new(timer);

    sleeper
        .sleep_until(deadline)
        .expect("latched wake should cause an immediate repoll");
}

struct FailingTimer {
    clock: ManualMonotonicClock,
}

impl Timer for FailingTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Err(TimeError::TimerUnavailable {
            reason: TimerUnavailableReason::BackendUnavailable,
        })
    }
}

#[test]
fn test_blocking_sleeper_returns_registration_error_without_parking() {
    let clock = ManualMonotonicClock::new();
    let deadline = clock.now();
    let sleeper = BlockingSleeper::new(Arc::new(FailingTimer { clock }));

    assert_eq!(
        Err(TimeError::TimerUnavailable {
            reason: TimerUnavailableReason::BackendUnavailable,
        }),
        sleeper.sleep_until(deadline),
    );
}
