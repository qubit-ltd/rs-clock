// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    Timer,
    TimerFuture,
};
use std::future;
use std::sync::{
    Arc,
    Mutex,
};
use std::time::Duration;

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
        Ok(Box::pin(future::ready(())))
    }
}

struct FailingTimer {
    clock: Arc<ManualMonotonicClock>,
}

impl Timer for FailingTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Err(TimeError::TimerUnavailable)
    }
}

#[test]
fn test_timer_supports_trait_object() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer: Arc<dyn Timer> = Arc::new(RecordingTimer::new(clock));

    let _future = timer
        .after(Duration::from_secs(1))
        .expect("timer registration should succeed");
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
fn test_timer_after_returns_registration_error_immediately() {
    let timer = FailingTimer {
        clock: Arc::new(ManualMonotonicClock::new()),
    };

    let error = match timer.after(Duration::from_secs(1)) {
        Ok(_) => panic!("registration should fail before returning a future"),
        Err(error) => error,
    };

    assert_eq!(TimeError::TimerUnavailable, error);
}

#[test]
fn test_timer_future_is_send_and_static() {
    fn assert_send_static<T: Send + 'static>(_: T) {}

    let future: TimerFuture = Box::pin(future::pending());
    assert_send_static(future);
}

#[test]
fn test_timer_arc_and_box_delegate_to_inner_timer() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let shared = Arc::new(RecordingTimer::new(Arc::clone(&clock)));
    let boxed: Box<dyn Timer> = Box::new(RecordingTimer::new(clock));

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
    assert_eq!(Duration::ZERO, boxed.clock().now().elapsed_since_origin());
}
