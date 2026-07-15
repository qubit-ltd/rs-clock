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
