// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{ClockDomain, TimeError, TimerUnavailableError};
use std::{io, time::Duration};

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
        "manual monotonic time cannot move backward from 10s to 5s",
        TimeError::CannotMoveBackward {
            current_elapsed: Duration::from_secs(10),
            requested_elapsed: Duration::from_secs(5),
        }
        .to_string(),
    );
    assert_eq!(
        "instant at 2s cannot be earlier than current instant at 1s",
        TimeError::InvalidInstantOrder {
            current_elapsed: Duration::from_secs(1),
            earlier_elapsed: Duration::from_secs(2),
        }
        .to_string(),
    );
    assert_eq!(
        "monotonic timer is unavailable: timer backend 'test' is unavailable: \
         offline",
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::BackendUnavailable {
                backend: "test",
                source: Box::new(io::Error::other("offline")),
            },
        }
        .to_string(),
    );
}

#[test]
fn test_time_error_implements_std_error() {
    fn assert_error<T: std::error::Error>() {}
    assert_error::<TimeError>();
}

#[test]
fn test_timer_unavailable_error_converts_to_time_error() {
    let error: TimeError = TimerUnavailableError::SchedulerWorkerTerminated.into();

    assert!(matches!(
        error,
        TimeError::TimerUnavailable {
            source: TimerUnavailableError::SchedulerWorkerTerminated,
        }
    ));
}
