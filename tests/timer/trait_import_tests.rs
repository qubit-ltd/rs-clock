/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
// qubit-style: allow explicit-imports
use std::time::Duration;

use qubit_clock::timer::*;

#[test]
fn test_glob_import_exposes_blocking_methods_without_ambiguity() {
    let timer = MockTimer::new();
    let deadline = timer.deadline_after(Duration::ZERO);

    timer.notify_all_waiters();
    timer
        .sleep_until(deadline)
        .expect("deadline belongs to this timer");
    assert_eq!(
        TimerWaitOutcome::DeadlineReached,
        timer
            .wait_until(deadline)
            .expect("deadline belongs to this timer"),
    );
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn test_glob_import_exposes_async_methods_without_ambiguity() {
    let timer = MockTimer::new();
    let deadline = timer.deadline_after(Duration::ZERO);

    timer.notify_all_waiters();
    timer
        .sleep_until_async(deadline)
        .await
        .expect("deadline belongs to this timer");
    assert_eq!(
        TimerWaitOutcome::DeadlineReached,
        timer
            .wait_until_async(deadline)
            .await
            .expect("deadline belongs to this timer"),
    );
}
