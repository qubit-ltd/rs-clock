// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::test_util::TimerFailurePoint;

/// Verifies that failure points retain value semantics for test assertions.
#[test]
fn test_timer_failure_point_has_value_semantics() {
    /// Requires a type to support copied and cloned test values.
    fn assert_copy_clone<T: Copy + Clone>() {}

    assert_copy_clone::<TimerFailurePoint>();
    assert_ne!(TimerFailurePoint::Registration, TimerFailurePoint::Completion,);
}

/// Verifies that failure points produce useful diagnostic output.
#[test]
fn test_timer_failure_point_debug_identifies_stage() {
    assert_eq!("Registration", format!("{:?}", TimerFailurePoint::Registration));
    assert_eq!("Completion", format!("{:?}", TimerFailurePoint::Completion));
}
