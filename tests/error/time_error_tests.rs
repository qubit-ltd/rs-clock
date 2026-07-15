// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ClockDomain,
    TimeError,
};

#[test]
fn test_time_error_clock_domain_mismatch_display() {
    let error = TimeError::ClockDomainMismatch {
        expected: ClockDomain::new(),
        actual: ClockDomain::new(),
    };
    assert!(
        error
            .to_string()
            .starts_with("monotonic clock domain mismatch: expected "),
        "domain mismatch error should include the expected domain",
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
