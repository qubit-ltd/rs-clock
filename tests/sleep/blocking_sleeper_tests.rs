// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    BlockingSleeper,
    ManualBlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    TimeError,
};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_blocking_sleeper_supports_trait_object() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper: Arc<dyn BlockingSleeper> =
        Arc::new(ManualBlockingSleeper::from_clock(Arc::clone(&clock)));

    assert_eq!(clock.now().domain(), sleeper.clock().now().domain());
    sleeper
        .sleep_for(Duration::ZERO)
        .expect("zero sleep should complete immediately");
}

#[test]
fn test_blocking_sleeper_box_delegates_to_inner_sleeper() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper: Box<dyn BlockingSleeper> =
        Box::new(ManualBlockingSleeper::from_clock(Arc::clone(&clock)));

    assert_eq!(clock.now().domain(), sleeper.clock().now().domain());
    sleeper
        .sleep_until(clock.now())
        .expect("reached deadline should complete immediately");
}

/// Verifies that a relative wait reports an unrepresentable deadline.
#[test]
fn test_blocking_sleeper_sleep_for_reports_deadline_overflow() {
    let clock = Arc::new(ManualMonotonicClock::new());
    clock
        .advance(Duration::MAX)
        .expect("maximum elapsed duration should fit from zero");
    let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));

    assert_eq!(
        Err(TimeError::InstantOverflow),
        sleeper.sleep_for(Duration::from_nanos(1)),
    );
    assert_eq!(0, clock.pending_waiters());
}
