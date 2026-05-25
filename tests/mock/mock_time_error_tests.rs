/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for mock time errors.

use std::error::Error;

use qubit_clock::MockTimeError;

/// Verifies active-waiter errors expose a useful display message.
///
/// # Errors
/// The test fails if the display text no longer identifies active waiters.
#[test]
fn test_active_waiters_display_message() {
    let error = MockTimeError::ActiveWaiters;

    assert_eq!("mock timeline has active waiters", error.to_string());
}

/// Verifies mismatched-timeline errors expose both timeline ids.
///
/// # Errors
/// The test fails if the display text no longer identifies the mismatched ids.
#[test]
fn test_mismatched_timeline_display_message() {
    let error = MockTimeError::MismatchedTimeline { expected: 7, actual: 9 };

    assert_eq!(
        "mock instant belongs to timeline 9, but timeline 7 was expected",
        error.to_string(),
    );
}

/// Verifies mock time errors implement the standard error trait.
///
/// # Errors
/// The test fails if the error type stops implementing [`Error`].
#[test]
fn test_mock_time_error_implements_error() {
    fn assert_error<E: Error>() {}

    assert_error::<MockTimeError>();
}
