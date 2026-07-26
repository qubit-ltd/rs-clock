// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock, MonotonicClock, MonotonicInstant, TimeError, Timer,
    TimerUnavailableError, TokioMonotonicClock, TokioRuntimeError, TokioTimer,
};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

/// Environment flag selecting the isolated runtime-shutdown child path.
const TOKIO_SHUTDOWN_CHILD: &str = "QUBIT_CLOCK_TOKIO_SHUTDOWN_CHILD";

/// Exact integration-test path executed in the isolated child process.
const TOKIO_SHUTDOWN_TEST: &str = concat!(
    "timer::tokio_timer_tests::",
    "test_tokio_timer_reports_retained_runtime_shutdown_without_panicking",
);

/// Environment flag selecting post-shutdown registration in an isolated child.
const TOKIO_POST_SHUTDOWN_REGISTRATION_CHILD: &str =
    "QUBIT_CLOCK_TOKIO_POST_SHUTDOWN_REGISTRATION_CHILD";

/// Exact post-shutdown registration test path executed in the isolated child.
const TOKIO_POST_SHUTDOWN_REGISTRATION_TEST: &str = concat!(
    "timer::tokio_timer_tests::",
    "test_tokio_timer_registers_after_retained_runtime_shutdown_without_panicking",
);

/// Runs one runtime-shutdown case in an isolated child process.
///
/// # Parameters
///
/// * `child_variable` - Environment variable selecting the child path.
/// * `test_path` - Exact integration-test path executed in the child.
///
/// # Returns
///
/// `true` after the parent validates its child, or `false` inside that child.
///
/// # Panics
///
/// Panics when the child cannot start, fails, or invokes Tokio's shutdown panic
/// hook.
fn run_isolated_shutdown_test(child_variable: &str, test_path: &str) -> bool {
    if std::env::var_os(child_variable).is_some() {
        return false;
    }
    let output =
        Command::new(std::env::current_exe().expect("current test executable should exist"))
            .args(["--exact", test_path, "--nocapture"])
            .env(child_variable, "1")
            .output()
            .expect("isolated shutdown test should start");
    assert!(
        output.status.success(),
        "isolated shutdown test should pass: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("A Tokio 1.x context was found, but it is being shutdown."),
        "runtime shutdown must not invoke the panic hook: {stderr}",
    );
    true
}

#[tokio::test(start_paused = true)]
async fn test_tokio_timer_fixes_deadline_before_first_poll() {
    let clock = TokioMonotonicClock::current();
    let timer = TokioTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_secs(8))
        .expect("Tokio deadline should register");

    tokio::time::advance(Duration::from_secs(8)).await;
    future.await.expect("Tokio timer should complete");
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
    let timer = TokioTimer::try_current().expect("entered runtime should create a Tokio timer");
    let other = TokioTimer::current();

    assert_ne!(timer.clock().now().domain(), other.clock().now().domain());
}

/// Verifies that a future deadline can be registered without an ambient
/// runtime and is driven by the retained handle.
#[test]
fn test_tokio_timer_registers_future_deadline_outside_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let future = timer
        .after(Duration::from_secs(1))
        .expect("future deadline should register outside the runtime");

    runtime.block_on(async {
        tokio::time::advance(Duration::from_secs(1)).await;
        future.await.expect("Tokio timer should complete");
    });
}

/// Verifies that relative reached deadlines need no ambient runtime.
#[test]
fn test_tokio_timer_after_zero_succeeds_outside_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let future = timer
        .after(Duration::ZERO)
        .expect("zero delay should be ready without a time driver");

    runtime
        .block_on(future)
        .expect("reached Tokio timer should complete");
}

/// Verifies that Arc delegation preserves retained-runtime registration.
#[test]
fn test_tokio_timer_arc_after_uses_retained_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime should build");
    let timer: Arc<dyn Timer> = Arc::new(TokioTimer::from_handle(runtime.handle().clone()));
    let future = timer
        .after(Duration::ZERO)
        .expect("Arc timer should register through its retained runtime");

    runtime
        .block_on(future)
        .expect("Arc Tokio timer should complete");
}

/// Verifies that Box delegation preserves retained-runtime registration.
#[test]
fn test_tokio_timer_box_after_uses_retained_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime should build");
    let timer: Box<dyn Timer> = Box::new(TokioTimer::from_handle(runtime.handle().clone()));
    let future = timer
        .after(Duration::ZERO)
        .expect("boxed timer should register through its retained runtime");

    runtime
        .block_on(future)
        .expect("boxed Tokio timer should complete");
}

#[test]
fn test_tokio_timer_reports_disabled_time_driver_at_registration() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());

    assert!(matches!(
        timer.after(Duration::from_secs(1)),
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::TimeDriverDisabled,
        }),
    ));
}

/// Verifies that reached deadlines do not require a Tokio time driver.
#[test]
fn test_tokio_timer_reached_deadline_needs_no_time_driver() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let deadline = timer.clock().now();
    let future = timer
        .at(deadline)
        .expect("reached deadline should be immediately ready");

    runtime
        .block_on(future)
        .expect("reached Tokio timer should complete");
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
        let deadline = MonotonicInstant::new(clock.now().domain(), Duration::MAX);
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

    future.await.expect("Tokio timer should complete");
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

/// Verifies that the retained runtime, rather than the polling runtime, drives
/// a future deadline.
#[test]
fn test_tokio_timer_future_is_driven_by_retained_runtime() {
    let target = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("target runtime should build");
    let polling = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("polling runtime should build");
    let timer = TokioTimer::from_handle(target.handle().clone());
    let mut future = timer
        .after(Duration::from_secs(5))
        .expect("future deadline should register on the retained runtime");

    let completed = polling.block_on(async {
        tokio::select! {
            result = &mut future => {
                result.expect("retained-runtime timer should complete");
                true
            },
            () = tokio::time::sleep(Duration::from_secs(1)) => false,
        }
    });
    assert!(!completed, "advancing the polling runtime must not fire it");

    target.block_on(tokio::time::advance(Duration::from_secs(5)));
    polling
        .block_on(future)
        .expect("retained-runtime timer should complete");
}

/// Verifies runtime shutdown becomes a structured error without invoking the
/// process panic hook.
#[test]
fn test_tokio_timer_reports_retained_runtime_shutdown_without_panicking() {
    if run_isolated_shutdown_test(TOKIO_SHUTDOWN_CHILD, TOKIO_SHUTDOWN_TEST) {
        return;
    }

    let future = {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("retained runtime should build");
        let timer = TokioTimer::from_handle(runtime.handle().clone());
        timer
            .after(Duration::from_secs(1))
            .expect("future deadline should register")
    };
    let polling = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("polling runtime should build");

    let error = polling
        .block_on(future)
        .expect_err("shutdown target runtime should fail the timer future");

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::RuntimeShuttingDown,
        },
    ));
}

/// Verifies first registration after retained-runtime shutdown remains typed
/// and does not invoke the process panic hook.
#[test]
fn test_tokio_timer_registers_after_retained_runtime_shutdown_without_panicking() {
    if run_isolated_shutdown_test(
        TOKIO_POST_SHUTDOWN_REGISTRATION_CHILD,
        TOKIO_POST_SHUTDOWN_REGISTRATION_TEST,
    ) {
        return;
    }

    let timer = {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("retained runtime should build");
        TokioTimer::from_handle(runtime.handle().clone())
    };
    let future = timer
        .after(Duration::from_secs(1))
        .expect("shutdown runtime should still create a diagnostic future");
    let polling = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("polling runtime should build");

    let error = polling
        .block_on(future)
        .expect_err("shutdown target runtime should fail the timer future");

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::RuntimeShuttingDown,
        },
    ));
}

/// Verifies that domain validation does not depend on an ambient runtime.
#[test]
fn test_tokio_timer_reports_foreign_deadline_outside_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime should build");
    let timer = TokioTimer::from_handle(runtime.handle().clone());
    let foreign = ManualMonotonicClock::new().now();

    assert!(matches!(
        timer.at(foreign),
        Err(TimeError::ClockDomainMismatch { .. }),
    ));
}
