// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use qubit_clock::BlockingSleeper;
use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::MonotonicInstant;
use qubit_clock::StdMonotonicClock;
use qubit_clock::TimeError;
use qubit_clock::Timer;
use qubit_clock::TimerFuture;
use qubit_clock::TimerUnavailableError;

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

struct CompletionFailingTimer {
    clock: ManualMonotonicClock,
}

impl Timer for CompletionFailingTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Ok(Box::pin(std::future::ready(Err(
            TimeError::TimerUnavailable {
                source: TimerUnavailableError::BackendUnavailable {
                    backend: "test",
                    source: Box::new(io::Error::other(
                        "test timer completion failed",
                    )),
                },
            },
        ))))
    }
}

#[test]
fn test_blocking_sleeper_returns_completion_error() {
    let clock = ManualMonotonicClock::new();
    let sleeper =
        BlockingSleeper::new(Arc::new(CompletionFailingTimer { clock }));

    let Err(TimeError::TimerUnavailable {
        source: TimerUnavailableError::BackendUnavailable { backend, source },
    }) = sleeper.sleep_for(Duration::from_secs(1))
    else {
        panic!("failing timer should report completion failure");
    };
    assert_eq!("test", backend);
    assert_eq!("test timer completion failed", source.to_string());
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
            source: TimerUnavailableError::BackendUnavailable {
                backend: "test",
                source: Box::new(io::Error::other("test backend unavailable")),
            },
        })
    }
}

#[test]
fn test_blocking_sleeper_returns_registration_error_without_parking() {
    let clock = ManualMonotonicClock::new();
    let deadline = clock.now();
    let sleeper = BlockingSleeper::new(Arc::new(FailingTimer { clock }));

    let Err(TimeError::TimerUnavailable {
        source: TimerUnavailableError::BackendUnavailable { backend, source },
    }) = sleeper.sleep_until(deadline)
    else {
        panic!("failing timer should report backend unavailability");
    };
    assert_eq!("test", backend);
    assert_eq!("test backend unavailable", source.to_string());
}
