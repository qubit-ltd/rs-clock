/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_clock::timer::{
    MockTimer,
    MonotonicTimer,
};

#[test]
fn test_display_describes_timer_domain_mismatch() {
    let first = MockTimer::new();
    let second = MockTimer::new();

    let error = first
        .duration_until(second.now())
        .expect_err("foreign timer instant should be rejected");

    let message = error.to_string();

    assert!(
        message.contains("timer domain mismatch"),
        "unexpected error message: {message}",
    );
}
