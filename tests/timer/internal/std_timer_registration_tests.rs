// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_clock::StdMonotonicClock;
use qubit_clock::StdTimer;
use qubit_clock::Timer;

use super::super::support::block_on_timer_future;

#[test]
fn test_std_timer_registration_handles_cancellation_churn() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let anchor = timer
        .after(Duration::from_secs(30))
        .expect("anchor deadline should register");
    for offset in 0..4096_u64 {
        let cancelled = timer
            .after(Duration::from_secs(31 + offset))
            .expect("churn deadline should register");
        drop(cancelled);
    }
    drop(anchor);

    let survivor = timer
        .after(Duration::from_millis(5))
        .expect("post-churn deadline should register");
    block_on_timer_future(survivor);
}
