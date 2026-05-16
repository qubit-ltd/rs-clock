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
    MonotonicTimer,
    TimerError,
};

#[test]
fn test_checked_add_keeps_timer_domain() {
    let timer = MockTimer::new();
    let instant = timer.now();
    let later = instant
        .checked_add(Duration::from_millis(250))
        .expect("adding a small duration should succeed");

    assert_eq!(instant.domain(), later.domain());
    assert_eq!(
        Duration::from_millis(250),
        later
            .checked_duration_since(instant)
            .expect("instants from the same timer domain should compare")
            .expect("later instant should be after the original instant"),
    );
}

#[test]
fn test_checked_duration_since_rejects_foreign_timer_domain() {
    let first = MockTimer::new();
    let second = MockTimer::new();

    let error = first
        .now()
        .checked_duration_since(second.now())
        .expect_err("instants from different timer domains should be rejected");

    assert!(matches!(
        error,
        TimerError::TimerDomainMismatch {
            expected: _,
            actual: _
        }
    ));
}

#[test]
fn test_checked_duration_since_returns_none_for_later_reference() {
    let timer = MockTimer::new();
    let start = timer.now();
    let later = start
        .checked_add(Duration::from_millis(10))
        .expect("adding a small duration should succeed");

    assert_eq!(
        None,
        start
            .checked_duration_since(later)
            .expect("instants from the same timer domain should compare"),
    );
}

#[test]
fn test_checked_add_returns_none_on_duration_overflow() {
    let timer = MockTimer::new();
    let max = timer.now().saturating_add(Duration::MAX);

    assert_eq!(None, max.checked_add(Duration::from_nanos(1)));
}

#[test]
fn test_saturating_duration_since_clamps_to_zero() {
    let timer = MockTimer::new();
    let start = timer.now();
    let later = start
        .checked_add(Duration::from_millis(10))
        .expect("adding a small duration should succeed");

    assert_eq!(
        Duration::ZERO,
        start
            .saturating_duration_since(later)
            .expect("instants from the same timer domain should compare"),
    );
}
