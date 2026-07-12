// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    BlockingSleeper,
    MonotonicClock,
    StdBlockingSleeper,
    StdMonotonicClock,
    TimeError,
};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_std_blocking_sleeper_uses_supplied_clock_domain() {
    let clock = Arc::new(StdMonotonicClock::new());
    let sleeper = StdBlockingSleeper::from_clock(Arc::clone(&clock));
    assert_eq!(clock.now().domain_id(), sleeper.now().domain_id());
}

#[test]
fn test_std_blocking_sleeper_waits_until_deadline() {
    let clock = Arc::new(StdMonotonicClock::new());
    let sleeper = StdBlockingSleeper::from_clock(Arc::clone(&clock));
    let start = clock.now();

    sleeper
        .sleep_for(Duration::from_millis(2))
        .expect("short real sleep should succeed");

    assert!(
        clock
            .now()
            .duration_since(start)
            .expect("instants should share one domain")
            >= Duration::from_millis(1),
    );
}

#[test]
fn test_std_blocking_sleeper_rejects_foreign_deadline() {
    let clock = Arc::new(StdMonotonicClock::new());
    let sleeper = StdBlockingSleeper::from_clock(Arc::clone(&clock));
    let foreign = StdMonotonicClock::new().now();

    assert!(matches!(
        sleeper.sleep_until(foreign),
        Err(TimeError::ClockDomainMismatch { .. }),
    ));
}

#[test]
fn test_std_blocking_sleeper_reports_native_deadline_overflow() {
    let clock = Arc::new(StdMonotonicClock::new());
    let sleeper = StdBlockingSleeper::from_clock(Arc::clone(&clock));
    let now = clock.now();
    let remaining = Duration::MAX
        .checked_sub(now.elapsed_since_origin())
        .expect("current elapsed should be below Duration maximum");
    let deadline = now
        .checked_add(remaining)
        .expect("maximum monotonic deadline should be representable");

    assert_eq!(
        Err(TimeError::InstantOverflow),
        sleeper.sleep_until(deadline),
    );
}
