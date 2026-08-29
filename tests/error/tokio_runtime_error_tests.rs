// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;

use qubit_clock::TokioMonotonicClock;
use qubit_clock::TokioRuntimeError;
use tokio::runtime::TryCurrentError;

/// Verifies that ambient runtime lookup failures retain Tokio's source.
#[test]
fn test_tokio_runtime_error_retains_runtime_lookup_source() {
    let error = TokioMonotonicClock::try_current().expect_err("construction outside a runtime should fail");

    assert!(
        error.to_string().starts_with("no Tokio runtime is entered:"),
        "error should identify the missing ambient runtime"
    );
    let source = error
        .source()
        .and_then(|source| source.downcast_ref::<TryCurrentError>())
        .expect("Tokio lookup error should remain in the source chain");
    assert!(source.is_missing_context());
}

/// Verifies that Tokio runtime errors satisfy standard error bounds.
#[test]
fn test_tokio_runtime_error_implements_std_error() {
    fn assert_error<T: Error>() {}
    assert_error::<TokioRuntimeError>();
}
