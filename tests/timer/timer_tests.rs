// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use qubit_clock::ClockDomain;
use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::MonotonicInstant;
use qubit_clock::TimeError;
use qubit_clock::Timer;
use qubit_clock::TimerFuture;
use qubit_clock::TimerUnavailableError;

struct RecordingTimer {
    clock: Arc<ManualMonotonicClock>,
    deadline: Mutex<Option<MonotonicInstant>>,
}

impl RecordingTimer {
    fn new(clock: Arc<ManualMonotonicClock>) -> Self {
        Self {
            clock,
            deadline: Mutex::new(None),
        }
    }

    fn deadline(&self) -> Option<MonotonicInstant> {
        *self.deadline.lock().expect("deadline mutex is poisoned")
    }
}

impl Timer for RecordingTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        *self.deadline.lock().expect("deadline mutex is poisoned") =
            Some(deadline);
        Ok(Box::pin(future::ready(Ok(()))))
    }
}

fn timer_domain<T: Timer>(timer: T) -> ClockDomain {
    timer.clock().domain()
}

#[test]
fn test_timer_reference_delegates_to_concrete_and_trait_object() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = RecordingTimer::new(clock.clone());
    let trait_object: &dyn Timer = &timer;

    assert_eq!(clock.domain(), timer_domain(&timer));
    assert_eq!(clock.domain(), timer_domain(trait_object));
}

struct FailingTimer {
    clock: Arc<ManualMonotonicClock>,
}

struct DeadlineOverrideClock {
    domain: ClockDomain,
}

impl DeadlineOverrideClock {
    fn new() -> Self {
        Self {
            domain: ClockDomain::new(),
        }
    }
}

impl MonotonicClock for DeadlineOverrideClock {
    fn domain(&self) -> ClockDomain {
        self.domain
    }

    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, Duration::ZERO)
    }

    fn deadline_after(
        &self,
        _duration: Duration,
    ) -> Result<MonotonicInstant, TimeError> {
        Ok(MonotonicInstant::new(self.domain, Duration::from_secs(9)))
    }

    fn new_timer(&self) -> Arc<dyn Timer> {
        Arc::new(RecordingDeadlineTimer {
            clock: DeadlineOverrideClock {
                domain: self.domain,
            },
            deadline: Mutex::new(None),
        })
    }
}

struct RecordingDeadlineTimer {
    clock: DeadlineOverrideClock,
    deadline: Mutex<Option<MonotonicInstant>>,
}

impl RecordingDeadlineTimer {
    fn deadline(&self) -> Option<MonotonicInstant> {
        *self.deadline.lock().expect("deadline mutex is poisoned")
    }
}

impl Timer for RecordingDeadlineTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        *self.deadline.lock().expect("deadline mutex is poisoned") =
            Some(deadline);
        Ok(Box::pin(future::ready(Ok(()))))
    }
}

impl Timer for FailingTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::BackendUnavailable {
                backend: "test",
                source: Box::new(io::Error::other("test backend unavailable")),
            },
        })
    }
}

#[test]
fn test_timer_supports_trait_object() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer: Arc<dyn Timer> = Arc::new(RecordingTimer::new(clock));

    assert_eq!(Duration::ZERO, timer.clock().now().elapsed_since_origin());
    let _future = timer
        .after(Duration::from_secs(1))
        .expect("timer registration should succeed");
}

#[test]
fn test_timer_clock_exposes_its_clock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    clock
        .advance(Duration::from_secs(3))
        .expect("manual time should advance");
    let timer = RecordingTimer::new(Arc::clone(&clock));

    assert_eq!(clock.now(), timer.clock().now());
}

#[test]
fn test_timer_after_fixes_deadline_when_called() {
    let clock = Arc::new(ManualMonotonicClock::new());
    clock
        .advance(Duration::from_secs(3))
        .expect("manual time should advance");
    let timer = RecordingTimer::new(Arc::clone(&clock));

    let _future = timer
        .after(Duration::from_secs(5))
        .expect("timer registration should succeed");

    assert_eq!(
        Some(Duration::from_secs(8)),
        timer.deadline().map(MonotonicInstant::elapsed_since_origin)
    );
}

#[test]
fn test_timer_clock_deadline_after_fixes_an_unregistered_deadline() {
    let clock = Arc::new(ManualMonotonicClock::new());
    clock
        .advance(Duration::from_secs(3))
        .expect("manual time should advance");
    let timer = RecordingTimer::new(Arc::clone(&clock));

    let deadline = timer
        .clock()
        .deadline_after(Duration::from_secs(5))
        .expect("deadline should be representable");

    assert_eq!(Duration::from_secs(8), deadline.elapsed_since_origin());
    assert_eq!(None, timer.deadline());
}

#[test]
fn test_timer_clock_deadline_after_reports_duration_overflow() {
    let clock = Arc::new(ManualMonotonicClock::new());
    clock
        .advance(Duration::from_nanos(1))
        .expect("manual time should advance");
    let timer = RecordingTimer::new(clock);

    assert!(matches!(
        timer.clock().deadline_after(Duration::MAX),
        Err(TimeError::InstantOverflow)
    ));
}

#[test]
fn test_timer_after_delegates_deadline_calculation_to_its_clock() {
    let timer = RecordingDeadlineTimer {
        clock: DeadlineOverrideClock::new(),
        deadline: Mutex::new(None),
    };

    let _future = timer
        .after(Duration::from_secs(2))
        .expect("timer registration should succeed");

    assert_eq!(
        Some(Duration::from_secs(9)),
        timer.deadline().map(MonotonicInstant::elapsed_since_origin)
    );
}

#[test]
fn test_timer_after_returns_registration_error_immediately() {
    let timer = FailingTimer {
        clock: Arc::new(ManualMonotonicClock::new()),
    };

    let error = match timer.after(Duration::from_secs(1)) {
        Ok(_) => panic!("registration should fail before returning a future"),
        Err(error) => error,
    };

    let TimeError::TimerUnavailable {
        source: TimerUnavailableError::BackendUnavailable { backend, source },
    } = error
    else {
        panic!("failing timer should report backend unavailability");
    };
    assert_eq!("test", backend);
    assert_eq!("test backend unavailable", source.to_string());
}

#[test]
fn test_timer_arc_and_box_delegate_to_inner_timer() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let shared = Arc::new(RecordingTimer::new(Arc::clone(&clock)));
    let boxed = Box::new(RecordingTimer::new(clock));

    assert_eq!(
        Duration::ZERO,
        <Box<RecordingTimer> as Timer>::clock(&boxed)
            .now()
            .elapsed_since_origin()
    );
    let direct_deadline = boxed
        .clock()
        .now()
        .checked_add(Duration::from_secs(3))
        .expect("boxed timer deadline should be representable");
    let _direct_future =
        <Box<RecordingTimer> as Timer>::at(&boxed, direct_deadline)
            .expect("boxed timer registration should succeed");

    let _shared_future = shared
        .after(Duration::from_secs(2))
        .expect("shared timer registration should succeed");
    let _boxed_future = boxed
        .after(Duration::from_secs(4))
        .expect("boxed timer registration should succeed");

    assert_eq!(
        Some(Duration::from_secs(2)),
        shared
            .deadline()
            .map(MonotonicInstant::elapsed_since_origin)
    );
    assert_eq!(
        Some(Duration::from_secs(4)),
        boxed.deadline().map(MonotonicInstant::elapsed_since_origin)
    );
}
