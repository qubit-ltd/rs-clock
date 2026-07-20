// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::TokioRuntimeError;
use std::error::Error;

/// Verifies that runtime mismatches report both runtime identities.
#[test]
fn test_tokio_runtime_error_reports_runtime_mismatch() {
    let first = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("first runtime should build");
    let second = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("second runtime should build");
    let expected = first.handle().id();
    let actual = second.handle().id();
    let error = TokioRuntimeError::Mismatch { expected, actual };

    assert_eq!(
        format!("Tokio runtime mismatch: expected {expected}, actual {actual}"),
        error.to_string(),
    );
    let TokioRuntimeError::Mismatch {
        expected: reported_expected,
        actual: reported_actual,
    } = error
    else {
        panic!("runtime mismatch should retain both runtime identities");
    };
    assert_eq!(expected, reported_expected);
    assert_eq!(actual, reported_actual);
}

/// Verifies that Tokio runtime errors satisfy standard error bounds.
#[test]
fn test_tokio_runtime_error_implements_std_error() {
    fn assert_error<T: Error>() {}
    assert_error::<TokioRuntimeError>();
}
