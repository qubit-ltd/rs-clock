// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::TimeError;

#[test]
fn test_time_error_clock_domain_mismatch_display() {
    let error = TimeError::ClockDomainMismatch {
        expected: 7,
        actual: 11,
    };
    assert_eq!(
        "monotonic clock domain mismatch: expected 7, actual 11",
        error.to_string(),
    );
}

#[test]
fn test_time_error_other_variants_display() {
    assert_eq!(
        "monotonic instant overflow",
        TimeError::InstantOverflow.to_string(),
    );
    assert_eq!(
        "manual monotonic time cannot move backward",
        TimeError::CannotMoveBackward.to_string(),
    );
    assert_eq!(
        "earlier monotonic instant is later than the current instant",
        TimeError::InvalidInstantOrder.to_string(),
    );
}

#[test]
fn test_time_error_implements_std_error() {
    fn assert_error<T: std::error::Error>() {}
    assert_error::<TimeError>();
}
