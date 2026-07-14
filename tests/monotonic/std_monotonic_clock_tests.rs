// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    MonotonicClock,
    StdMonotonicClock,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_std_monotonic_clock_progresses_with_real_time() {
    let clock = StdMonotonicClock::new();
    let start = clock.now();
    thread::sleep(Duration::from_millis(2));
    let end = clock.now();

    assert!(
        end.duration_since(start)
            .expect("instants should share one domain")
            >= Duration::from_millis(1),
    );
}

#[test]
fn test_std_monotonic_clock_instances_have_distinct_domains() {
    let first = StdMonotonicClock::new();
    let second = StdMonotonicClock::new();

    assert_ne!(first.now().domain(), second.now().domain());
}

#[test]
fn test_std_monotonic_clock_default_creates_clock() {
    let clock = StdMonotonicClock::default();
    let other = StdMonotonicClock::new();
    assert_ne!(clock.now().domain(), other.now().domain());
}
