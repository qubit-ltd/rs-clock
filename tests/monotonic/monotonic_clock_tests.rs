// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    allocate_clock_domain_id,
};
use std::sync::Arc;
use std::time::Duration;

struct ExternalMonotonicClock {
    domain_id: u64,
    elapsed: Duration,
}

impl MonotonicClock for ExternalMonotonicClock {
    fn domain_id(&self) -> u64 {
        self.domain_id
    }

    fn elapsed_since_origin(&self) -> Duration {
        self.elapsed
    }
}

#[test]
fn test_monotonic_clock_can_be_implemented_outside_crate() {
    let clock = ExternalMonotonicClock {
        domain_id: allocate_clock_domain_id(),
        elapsed: Duration::from_secs(7),
    };

    assert_eq!(clock.domain_id, clock.now().domain_id());
    assert_eq!(clock.elapsed, clock.now().elapsed_since_origin());
}

#[test]
fn test_monotonic_clock_supports_trait_object() {
    let clock: Arc<dyn MonotonicClock> = Arc::new(ManualMonotonicClock::new());
    let first = clock.now();
    let second = clock.now();

    assert_eq!(first.domain_id(), second.domain_id());
}

#[test]
fn test_monotonic_clock_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ManualMonotonicClock>();
}

#[test]
fn test_monotonic_clock_box_delegates_to_inner_clock() {
    let inner = ManualMonotonicClock::new();
    let domain_id = inner.now().domain_id();
    let clock: Box<dyn MonotonicClock> = Box::new(inner);

    assert_eq!(domain_id, clock.now().domain_id());
    assert_eq!(domain_id, clock.domain_id());
    assert_eq!(Duration::ZERO, clock.elapsed_since_origin());
}
