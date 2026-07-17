// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ClockDomain,
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    Timer,
    TimerFuture,
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

    fn new_timer(&self) -> Arc<dyn Timer> {
        Arc::new(ExternalTimer {
            clock: ExternalMonotonicClock {
                domain: self.domain,
                elapsed: self.elapsed,
            },
        })
    }
}

struct ExternalTimer {
    clock: ExternalMonotonicClock,
}

impl Timer for ExternalTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Ok(Box::pin(std::future::ready(())))
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
fn test_new_timer_does_not_consume_trait_object_clock() {
    let clock: Arc<dyn MonotonicClock> = Arc::new(ManualMonotonicClock::new());

    let timer = clock.new_timer();

    assert_eq!(clock.now().domain(), timer.clock().now().domain());
    let _still_usable = clock.now();
}

#[test]
fn test_monotonic_clock_with_timer_factory_is_object_safe() {
    fn assert_object_safe(_clock: &dyn MonotonicClock) {}

    let clock = ManualMonotonicClock::new();
    assert_object_safe(&clock);
}

#[test]
fn test_monotonic_clock_box_delegates_to_inner_clock() {
    let inner = ManualMonotonicClock::new();
    let domain = inner.now().domain();
    let clock: Box<dyn MonotonicClock> = Box::new(inner);

    assert_eq!(domain, clock.now().domain());
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
    assert_eq!(domain, clock.new_timer().clock().now().domain());
}
