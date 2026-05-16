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
    SystemTimer,
};

#[test]
fn test_timer_domain_distinguishes_independent_timers() {
    let first = MockTimer::new();
    let second = MockTimer::new();

    assert_ne!(first.timer_domain_id(), second.timer_domain_id());
}

#[test]
fn test_timer_domain_is_shared_by_clones() {
    let timer = SystemTimer::new();
    let clone = timer.clone();

    assert_eq!(timer.timer_domain_id(), clone.timer_domain_id());
}

#[test]
fn test_timer_domain_exposes_non_zero_diagnostic_value() {
    let timer = MockTimer::new();

    assert!(timer.timer_domain_id().get() > 0);
}
