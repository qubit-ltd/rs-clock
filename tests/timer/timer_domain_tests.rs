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
    SystemTimer,
    TimerDomain,
};

#[test]
fn test_timer_domain_distinguishes_independent_timers() {
    let first = MockTimer::new();
    let second = MockTimer::new();

    assert_ne!(first.id(), second.id());
}

#[test]
fn test_timer_domain_is_shared_by_clones() {
    let timer = SystemTimer::new();
    let clone = timer.clone();

    assert_eq!(timer.id(), clone.id());
}

#[test]
fn test_timer_domain_id_is_plain_u64() {
    let timer = MockTimer::new();
    let id: u64 = timer.id();

    assert_eq!(id, timer.now().domain_id());
}
