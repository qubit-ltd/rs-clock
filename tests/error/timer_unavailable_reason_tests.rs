// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::TimerUnavailableReason;

/// Verifies that every built-in unavailability reason identifies its resource.
#[test]
fn test_timer_unavailable_reason_display() {
    assert_eq!(
        "the scheduler worker thread could not be spawned",
        TimerUnavailableReason::WorkerThreadSpawnFailed.to_string(),
    );
    assert_eq!(
        "no asynchronous runtime is entered",
        TimerUnavailableReason::RuntimeNotEntered.to_string(),
    );
    assert_eq!(
        "the asynchronous runtime time driver is disabled",
        TimerUnavailableReason::TimeDriverDisabled.to_string(),
    );
    assert_eq!(
        "the timer backend is unavailable",
        TimerUnavailableReason::BackendUnavailable.to_string(),
    );
}

/// Verifies that unavailability reasons satisfy standard error bounds.
#[test]
fn test_timer_unavailable_reason_implements_std_error() {
    fn assert_error<T: std::error::Error>() {}
    assert_error::<TimerUnavailableReason>();
}
