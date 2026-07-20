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
    MonotonicInstant,
    TimeError,
    Timer,
    TimerUnavailableError,
    TokioMonotonicClock,
    TokioRuntimeError,
    TokioTimer,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn test_tokio_timer_fixes_deadline_before_first_poll() {
    let clock = TokioMonotonicClock::current();
    let timer = TokioTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_secs(8))
        .expect("Tokio deadline should register");

    tokio::time::advance(Duration::from_secs(8)).await;
    future.await;
}

/// Verifies that relative deadline overflow remains a structured time error.
#[tokio::test]
async fn test_tokio_timer_after_reports_duration_overflow() {
    let timer = TokioTimer::current();

    assert!(matches!(
        timer.after(Duration::MAX),
        Err(TimeError::InstantOverflow),
    ));
}

/// Verifies that fallible timer construction reports a missing runtime.
#[test]
fn test_tokio_timer_try_current_reports_missing_runtime() {
    assert!(matches!(
        TokioTimer::try_current(),
        Err(TokioRuntimeError::NotEntered { .. }),
    ));
}

/// Verifies that infallible timer construction rejects a missing runtime.
#[test]
#[should_panic(expected = "cannot create Tokio timer")]
fn test_tokio_timer_current_panics_outside_runtime() {
    let _ = TokioTimer::current();
}

/// Verifies that fallible timer construction allocates a new clock domain.
#[tokio::test]
async fn test_tokio_timer_try_current_creates_independent_timer() {
    let timer = TokioTimer::try_current()
        .expect("entered runtime should create a Tokio timer");
    let other = TokioTimer::current();

    assert_ne!(timer.clock().now().domain(), other.clock().now().domain());
}

/// Verifies that future deadlines require the timer's bound runtime.
#[test]
fn test_tokio_timer_reports_missing_runtime_for_future_deadline() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let (timer, deadline) = runtime.block_on(async {
        let clock = TokioMonotonicClock::current();
        let deadline = clock
            .now()
            .checked_add(Duration::from_secs(1))
            .expect("deadline should fit");
        (TokioTimer::from_clock(&clock), deadline)
    });

    let Err(TimeError::TimerUnavailable {
        source:
            TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::NotEntered { source },
            },
    }) = timer.at(deadline)
    else {
        panic!("future deadline should require an entered runtime");
    };
    assert!(source.is_missing_context());
}

/// Verifies that relative deadlines report a missing bound runtime.
#[test]
fn test_tokio_timer_after_reports_missing_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer = runtime.block_on(async { TokioTimer::current() });

    assert!(matches!(
        timer.after(Duration::from_secs(1)),
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::NotEntered { .. },
            },
        }),
    ));
}

/// Verifies that an Arc Timer preserves the concrete relative-time error.
#[test]
fn test_tokio_timer_arc_after_preserves_runtime_error() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer: Arc<dyn Timer> =
        runtime.block_on(async { Arc::new(TokioTimer::current()) });

    assert!(matches!(
        timer.after(Duration::from_secs(1)),
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::NotEntered { .. },
            },
        }),
    ));
}

/// Verifies that a boxed Timer preserves the concrete relative-time error.
#[test]
fn test_tokio_timer_box_after_preserves_runtime_error() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer: Box<dyn Timer> =
        runtime.block_on(async { Box::new(TokioTimer::current()) });

    assert!(matches!(
        timer.after(Duration::from_secs(1)),
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::NotEntered { .. },
            },
        }),
    ));
}

#[test]
fn test_tokio_timer_reports_disabled_time_driver_at_registration() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime should build");

    runtime.block_on(async {
        let timer = TokioTimer::current();
        assert!(matches!(
            timer.after(Duration::from_secs(1)),
            Err(TimeError::TimerUnavailable {
                source: TimerUnavailableError::TimeDriverDisabled,
            }),
        ));
    });
}

/// Verifies that reached deadlines still require the timer's bound runtime.
#[test]
fn test_tokio_timer_reports_missing_runtime_for_reached_deadline() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let (timer, deadline) = runtime.block_on(async {
        let clock = TokioMonotonicClock::current();
        let deadline = clock.now();
        (TokioTimer::from_clock(&clock), deadline)
    });

    assert!(matches!(
        timer.at(deadline),
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::NotEntered { .. },
            },
        }),
    ));
}

/// Verifies that native overflow is reported before Tokio runtime validation.
#[test]
fn test_tokio_timer_reports_native_instant_overflow() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let (timer, deadline) = runtime.block_on(async {
        let clock = TokioMonotonicClock::current();
        let deadline =
            MonotonicInstant::new(clock.now().domain(), Duration::MAX);
        (TokioTimer::from_clock(&clock), deadline)
    });

    let error = match timer.at(deadline) {
        Ok(_) => panic!("overflowing native deadline should fail"),
        Err(error) => error,
    };

    assert!(matches!(error, TimeError::InstantOverflow));
}

#[tokio::test]
async fn test_tokio_timer_returns_ready_future_for_reached_deadline() {
    let clock = TokioMonotonicClock::current();
    let timer = TokioTimer::from_clock(&clock);
    let deadline = clock.now();
    let future = timer
        .at(deadline)
        .expect("reached deadline should register successfully");

    future.await;
}

#[tokio::test]
async fn test_tokio_timer_rejects_foreign_deadline_immediately() {
    let clock = TokioMonotonicClock::current();
    let timer = TokioTimer::from_clock(&clock);
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();

    let error = match timer.at(foreign) {
        Ok(_) => panic!("foreign deadline should fail at registration"),
        Err(error) => error,
    };

    let TimeError::ClockDomainMismatch {
        expected: actual_expected,
        actual,
    } = error
    else {
        panic!("foreign deadline should report a domain mismatch");
    };
    assert_eq!(expected, actual_expected);
    assert_eq!(foreign.domain(), actual);
}

#[tokio::test]
async fn test_tokio_timer_retains_domain_after_source_is_dropped() {
    let (timer, domain) = {
        let clock = TokioMonotonicClock::current();
        let domain = clock.now().domain();
        (TokioTimer::from_clock(&clock), domain)
    };

    assert_eq!(domain, timer.clock().now().domain());
}

/// Verifies that reached deadlines reject an independent runtime.
#[test]
fn test_tokio_timer_rejects_reached_deadline_from_different_runtime() {
    let first = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("first runtime should build");
    let second = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("second runtime should build");
    let expected = first.handle().id();
    let actual = second.handle().id();
    let (timer, deadline) = first.block_on(async {
        let clock = TokioMonotonicClock::current();
        let deadline = clock.now();
        (TokioTimer::from_clock(&clock), deadline)
    });

    let error = second.block_on(async {
        match timer.at(deadline) {
            Ok(_) => panic!("a reached deadline should reject another runtime"),
            Err(error) => error,
        }
    });

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::Mismatch {
                    expected: actual_expected,
                    actual: actual_runtime,
                },
            },
        } if actual_expected == expected && actual_runtime == actual
    ));
}

/// Verifies that future deadlines reject an independent runtime.
#[test]
fn test_tokio_timer_rejects_future_deadline_from_different_runtime() {
    let first = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("first runtime should build");
    let second = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("second runtime should build");
    let expected = first.handle().id();
    let actual = second.handle().id();
    let (timer, deadline) = first.block_on(async {
        let clock = TokioMonotonicClock::current();
        let deadline = clock
            .now()
            .checked_add(Duration::from_secs(1))
            .expect("deadline should fit");
        (TokioTimer::from_clock(&clock), deadline)
    });

    let error = second.block_on(async {
        match timer.at(deadline) {
            Ok(_) => panic!("a future deadline should reject another runtime"),
            Err(error) => error,
        }
    });

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::Mismatch {
                    expected: actual_expected,
                    actual: actual_runtime,
                },
            },
        } if actual_expected == expected && actual_runtime == actual
    ));
}

/// Verifies that relative deadlines reject an independent runtime.
#[test]
fn test_tokio_timer_after_rejects_different_runtime() {
    let first = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("first runtime should build");
    let second = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("second runtime should build");
    let expected = first.handle().id();
    let actual = second.handle().id();
    let timer = first.block_on(async { TokioTimer::current() });

    let error = second.block_on(async {
        match timer.after(Duration::from_secs(1)) {
            Ok(_) => {
                panic!("a relative deadline should reject another runtime")
            }
            Err(error) => error,
        }
    });

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::TokioRuntime {
                source: TokioRuntimeError::Mismatch {
                    expected: actual_expected,
                    actual: actual_runtime,
                },
            },
        } if actual_expected == expected && actual_runtime == actual
    ));
}

/// Verifies that domain validation precedes Tokio runtime validation.
#[test]
fn test_tokio_timer_reports_foreign_deadline_before_runtime_validation() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer = runtime.block_on(async { TokioTimer::current() });
    let foreign = ManualMonotonicClock::new().now();

    assert!(matches!(
        timer.at(foreign),
        Err(TimeError::ClockDomainMismatch { .. }),
    ));
}
