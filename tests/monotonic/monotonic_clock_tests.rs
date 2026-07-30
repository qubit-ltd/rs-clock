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
    fn domain(&self) -> ClockDomain {
        self.domain
    }

    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.elapsed)
    }

    fn deadline_after(
        &self,
        _duration: Duration,
    ) -> Result<MonotonicInstant, TimeError> {
        Ok(MonotonicInstant::new(self.domain, Duration::from_secs(11)))
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
        Ok(Box::pin(std::future::ready(Ok(()))))
    }
}

#[test]
fn test_monotonic_clock_can_be_implemented_outside_crate() {
    let clock = ExternalMonotonicClock {
        domain: ClockDomain::new(),
        elapsed: Duration::from_secs(7),
    };

    assert_eq!(clock.domain, clock.domain());
    assert_eq!(clock.domain(), clock.now().domain());
    assert_eq!(clock.elapsed, clock.now().elapsed_since_origin());
}

#[test]
fn test_monotonic_clock_domain_is_stable() {
    let clock = ManualMonotonicClock::new();

    assert_eq!(clock.domain(), clock.domain());
    assert_eq!(clock.domain(), clock.now().domain());
}

#[test]
fn test_monotonic_clock_deadline_after_uses_current_instant() {
    let clock = ManualMonotonicClock::new();
    clock
        .advance(Duration::from_secs(7))
        .expect("manual time should advance");

    let deadline = clock
        .deadline_after(Duration::from_secs(5))
        .expect("deadline should be representable");

    assert_eq!(Duration::from_secs(12), deadline.elapsed_since_origin());
    assert_eq!(clock.domain(), deadline.domain());
}

#[test]
fn test_monotonic_clock_deadline_after_reports_duration_overflow() {
    let clock = ManualMonotonicClock::new();
    clock
        .advance(Duration::from_nanos(1))
        .expect("manual time should advance");

    assert!(matches!(
        clock.deadline_after(Duration::MAX),
        Err(TimeError::InstantOverflow)
    ));
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

    assert_eq!(clock.domain(), timer.clock().domain());
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
    let domain = inner.domain();
    let clock: Box<dyn MonotonicClock> = Box::new(inner);

    assert_eq!(domain, clock.domain());
    assert_eq!(domain, clock.now().domain());
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
    assert_eq!(domain, clock.new_timer().clock().domain());
}

#[test]
fn test_monotonic_clock_arc_delegates_domain_and_deadline_after() {
    let concrete_clock = ManualMonotonicClock::new();
    concrete_clock
        .advance(Duration::from_secs(7))
        .expect("manual time should advance");
    let clock: Arc<dyn MonotonicClock> = Arc::new(concrete_clock);

    let deadline = clock
        .deadline_after(Duration::from_secs(5))
        .expect("deadline should be representable");

    assert_eq!(clock.domain(), deadline.domain());
    assert_eq!(Duration::from_secs(12), deadline.elapsed_since_origin());
}

#[test]
fn test_monotonic_clock_arc_and_box_delegate_overridden_deadline_after() {
    let domain = ClockDomain::new();
    let shared: Arc<dyn MonotonicClock> = Arc::new(ExternalMonotonicClock {
        domain,
        elapsed: Duration::from_secs(7),
    });
    let boxed: Box<dyn MonotonicClock> = Box::new(ExternalMonotonicClock {
        domain,
        elapsed: Duration::from_secs(7),
    });

    let shared_deadline = shared
        .deadline_after(Duration::from_secs(5))
        .expect("deadline should be supplied by the wrapped clock");
    let boxed_deadline = boxed
        .deadline_after(Duration::from_secs(5))
        .expect("deadline should be supplied by the wrapped clock");

    assert_eq!(
        Duration::from_secs(11),
        shared_deadline.elapsed_since_origin()
    );
    assert_eq!(
        Duration::from_secs(11),
        boxed_deadline.elapsed_since_origin()
    );
}
