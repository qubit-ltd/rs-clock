/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_clock::timer::TimerWaitOutcome;

#[test]
fn test_wait_outcome_supports_equality_and_debugging() {
    assert_eq!(
        TimerWaitOutcome::DeadlineReached,
        TimerWaitOutcome::DeadlineReached,
    );
    assert_eq!("Notified", format!("{:?}", TimerWaitOutcome::Notified));
}
