// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    MonotonicClock,
    TokioMonotonicClock,
};
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn test_tokio_monotonic_clock_follows_tokio_time() {
    let clock = TokioMonotonicClock::new();
    let start = clock.now();

    tokio::time::advance(Duration::from_secs(5)).await;

    assert_eq!(
        Duration::from_secs(5),
        clock
            .now()
            .duration_since(start)
            .expect("instants should share one domain"),
    );
}

#[tokio::test(start_paused = true)]
async fn test_tokio_monotonic_clock_default_creates_clock() {
    let clock = TokioMonotonicClock::default();
    let other = TokioMonotonicClock::new();
    assert_ne!(clock.now().domain(), other.now().domain());
}

#[tokio::test(start_paused = true)]
async fn test_tokio_monotonic_clock_creates_same_domain_timer_directly() {
    let clock = TokioMonotonicClock::new();

    let timer = clock.new_timer();

    assert_eq!(clock.now().domain(), timer.clock().now().domain());
}
