// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ClockDomain,
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
};
use std::sync::Arc;
use std::time::Duration;

struct ExternalMonotonicClock {
    domain: ClockDomain,
    elapsed: Duration,
}

impl MonotonicClock for ExternalMonotonicClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.elapsed)
    }
}

#[test]
fn test_monotonic_clock_can_be_implemented_outside_crate() {
    let clock = ExternalMonotonicClock {
        domain: ClockDomain::new(),
        elapsed: Duration::from_secs(7),
    };

    assert_eq!(clock.domain, clock.now().domain());
    assert_eq!(clock.elapsed, clock.now().elapsed_since_origin());
}

#[test]
fn test_monotonic_clock_supports_trait_object() {
    let clock: Arc<dyn MonotonicClock> = Arc::new(ManualMonotonicClock::new());
    let first = clock.now();
    let second = clock.now();

    assert_eq!(first.domain(), second.domain());
}

#[test]
fn test_monotonic_clock_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ManualMonotonicClock>();
}

#[test]
fn test_monotonic_clock_box_delegates_to_inner_clock() {
    let inner = ManualMonotonicClock::new();
    let domain = inner.now().domain();
    let clock: Box<dyn MonotonicClock> = Box::new(inner);

    assert_eq!(domain, clock.now().domain());
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
}
