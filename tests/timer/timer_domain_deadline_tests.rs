/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::time::Duration;

use qubit_clock::timer::{
    MockTimer,
    TimerDomain,
};

#[test]
fn test_deadline_after_uses_timer_domain_relative_time() {
    let timer = MockTimer::new();
    timer.advance(Duration::from_millis(40));

    let deadline = timer.deadline_after(Duration::from_millis(60));

    assert_eq!(timer.id(), deadline.domain_id());
    assert_eq!(
        Some(Duration::from_millis(60)),
        timer
            .duration_until(deadline)
            .expect("deadline belongs to this timer")
    );
}

#[test]
fn test_duration_until_returns_none_after_deadline() {
    let timer = MockTimer::new();
    let deadline = timer.deadline_after(Duration::from_millis(10));

    timer.advance(Duration::from_millis(11));

    assert_eq!(
        None,
        timer
            .duration_until(deadline)
            .expect("deadline belongs to this timer"),
    );
}
