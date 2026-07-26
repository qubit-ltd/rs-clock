// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow coverage-cfg

#[cfg(coverage)]
use qubit_clock::{
    StdMonotonicClock, StdTimer, Timer, reset_std_timer_worker_notification_count,
    std_timer_worker_notification_count,
};
#[cfg(coverage)]
use std::time::Duration;

/// Verifies the worker is notified only when the earliest deadline changes.
#[cfg(coverage)]
#[test]
fn test_std_timer_scheduler_notifies_only_for_next_deadline_changes() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let anchor = timer
        .after(Duration::from_secs(60))
        .expect("anchor deadline should register");

    reset_std_timer_worker_notification_count();
    let later = timer
        .after(Duration::from_secs(120))
        .expect("later deadline should register");
    drop(later);
    assert_eq!(
        std_timer_worker_notification_count(),
        0,
        "a later registration and cancellation must not wake the worker",
    );

    let earlier = timer
        .after(Duration::from_secs(30))
        .expect("earlier deadline should register");
    assert_eq!(
        std_timer_worker_notification_count(),
        1,
        "an earlier registration must wake the worker",
    );

    reset_std_timer_worker_notification_count();
    drop(earlier);
    assert_eq!(
        std_timer_worker_notification_count(),
        1,
        "cancelling the earliest deadline must wake the worker",
    );

    reset_std_timer_worker_notification_count();
    drop(anchor);
    assert_eq!(
        std_timer_worker_notification_count(),
        1,
        "cancelling the last deadline must wake the worker",
    );
}
