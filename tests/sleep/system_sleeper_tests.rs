// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::time::{
    Duration,
    Instant,
};

use qubit_clock::sleep::{
    Sleeper,
    SystemSleeper,
};

#[test]
fn test_new_creates_system_sleeper() {
    let sleeper = SystemSleeper::new();

    sleeper.sleep_for(Duration::ZERO);
}

#[test]
fn test_sleep_for_waits_real_duration() {
    let sleeper = SystemSleeper::new();
    let start = Instant::now();

    sleeper.sleep_for(Duration::from_millis(2));

    assert!(start.elapsed() >= Duration::from_millis(1));
}
