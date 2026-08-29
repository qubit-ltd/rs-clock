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
use std::task::Waker;
use std::time::Duration;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::TimeError;
use qubit_clock::Timer;
use qubit_clock::TimerFuture;
use qubit_clock::TimerUnavailableError;
use qubit_clock::test_util::FaultInjectingTimer;
use qubit_clock::test_util::TimerFailurePoint;

/// Polls an immediate test Timer future to completion.
///
/// # Parameters
///
/// * `future` - Immediate future returned by a fault-injecting Timer.
///
/// # Returns
///
/// The completion result produced by `future`.
///
/// # Panics
///
/// Panics if the test utility returns a pending future.
fn poll_ready(mut future: TimerFuture) -> Result<(), TimeError> {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("fault-injecting Timer future must be ready"),
    }
}

/// Verifies a registration fault is returned by `Timer::at` itself.
#[test]
fn test_fault_injecting_timer_fails_registration() {
    let timer = FaultInjectingTimer::new(TimerFailurePoint::Registration, || TimeError::TimerUnavailable {
        source: TimerUnavailableError::SchedulerWorkerTerminated,
    });

    let Err(error) = timer.after(Duration::from_secs(1)) else {
        panic!("registration failure should be returned immediately");
    };

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::SchedulerWorkerTerminated,
        }
    ));
    assert_eq!(TimerFailurePoint::Registration, timer.failure_point());
    assert_eq!(1, timer.registration_count());
}

/// Verifies a completion fault is returned by the registered future.
#[test]
fn test_fault_injecting_timer_fails_completion() {
    let timer = FaultInjectingTimer::new(TimerFailurePoint::Completion, || TimeError::TimerUnavailable {
        source: TimerUnavailableError::SchedulerWorkerTerminated,
    });
    let future = timer
        .after(Duration::from_secs(1))
        .expect("completion failure should register a future");

    assert!(matches!(
        poll_ready(future),
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::SchedulerWorkerTerminated,
        })
    ));
    assert_eq!(TimerFailurePoint::Completion, timer.failure_point());
    assert_eq!(1, timer.registration_count());
}

/// Verifies a foreign deadline is rejected before invoking the error factory.
#[test]
fn test_fault_injecting_timer_rejects_foreign_domain_first() {
    let factory_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = Arc::clone(&factory_calls);
    let timer = FaultInjectingTimer::new(TimerFailurePoint::Registration, move || {
        observed_calls.fetch_add(1, Ordering::Relaxed);
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::SchedulerWorkerTerminated,
        }
    });
    let foreign_deadline = ManualMonotonicClock::new().now();

    let Err(error) = timer.at(foreign_deadline) else {
        panic!("foreign deadline should be rejected");
    };

    assert!(matches!(error, TimeError::ClockDomainMismatch { .. }));
    assert_eq!(0, factory_calls.load(Ordering::Relaxed));
    assert_eq!(0, timer.registration_count());
}

/// Verifies an already reached deadline bypasses fault injection.
#[test]
fn test_fault_injecting_timer_completes_reached_deadline() {
    let factory_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = Arc::clone(&factory_calls);
    let timer = FaultInjectingTimer::new(TimerFailurePoint::Registration, move || {
        observed_calls.fetch_add(1, Ordering::Relaxed);
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::SchedulerWorkerTerminated,
        }
    });
    let reached_deadline = timer.clock().now();
    let future = timer
        .at(reached_deadline)
        .expect("reached deadline should not touch the failing backend");

    assert!(poll_ready(future).is_ok());
    assert_eq!(0, factory_calls.load(Ordering::Relaxed));
    assert_eq!(0, timer.registration_count());
}

/// Verifies every registration obtains a fresh error from the factory.
#[test]
fn test_fault_injecting_timer_invokes_factory_per_registration() {
    let factory_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = Arc::clone(&factory_calls);
    let timer = FaultInjectingTimer::new(TimerFailurePoint::Registration, move || {
        observed_calls.fetch_add(1, Ordering::Relaxed);
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::SchedulerWorkerTerminated,
        }
    });

    for _ in 0..2 {
        assert!(timer.after(Duration::from_secs(1)).is_err());
    }

    assert_eq!(2, factory_calls.load(Ordering::Relaxed));
    assert_eq!(2, timer.registration_count());
}

/// Verifies the convenience constructor preserves backend error metadata.
#[test]
fn test_fault_injecting_timer_builds_backend_unavailable_error() {
    let timer = FaultInjectingTimer::backend_unavailable(TimerFailurePoint::Registration, "example", "backend offline");

    let Err(error) = timer.after(Duration::from_secs(1)) else {
        panic!("backend-unavailable Timer should reject registration");
    };
    let TimeError::TimerUnavailable {
        source: TimerUnavailableError::BackendUnavailable { backend, source },
    } = error
    else {
        panic!("convenience constructor should preserve the error category");
    };

    assert_eq!("example", backend);
    assert_eq!("backend offline", source.to_string());
}

/// Verifies the public fixture can be injected through shared Timer objects.
#[test]
fn test_fault_injecting_timer_is_send_and_sync() {
    /// Requires a value to satisfy the Timer thread-safety contract.
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<FaultInjectingTimer>();
}
