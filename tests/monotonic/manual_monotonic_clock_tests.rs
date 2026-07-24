// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{
    BlockingSleeper, ManualMonotonicClock, MonotonicClock, TimeError, Timer, WallClock,
};
use std::future::Future;
use std::pin::pin;
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, UNIX_EPOCH};

#[test]
fn test_manual_monotonic_clock_shared_helpers_use_same_timeline() {
    let clock = ManualMonotonicClock::new_shared();
    let wall_clock = clock.new_wall_clock(UNIX_EPOCH);
    let timer = clock.new_timer();
    let blocking_sleeper = BlockingSleeper::new(Arc::clone(&timer));

    assert_eq!(UNIX_EPOCH, wall_clock.now());
    assert_eq!(clock.now(), timer.clock().now());
    assert_eq!(clock.now(), blocking_sleeper.timer().clock().now());

    clock
        .advance(Duration::from_secs(4))
        .expect("short manual advance should succeed");

    assert_eq!(UNIX_EPOCH + Duration::from_secs(4), wall_clock.now());
    assert_eq!(clock.now(), timer.clock().now());
    assert_eq!(clock.now(), blocking_sleeper.timer().clock().now());
}

#[test]
fn test_manual_monotonic_clock_starts_at_zero() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();

    assert_eq!(Duration::ZERO, start.elapsed_since_origin());
}

#[test]
fn test_manual_monotonic_clock_advance_moves_forward() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();

    clock
        .advance(Duration::from_secs(10))
        .expect("short advance should succeed");

    assert_eq!(
        Duration::from_secs(10),
        clock
            .now()
            .duration_since(start)
            .expect("instants should share one domain"),
    );
}

#[test]
fn test_manual_monotonic_clock_advance_to_rejects_backward_target() {
    let clock = ManualMonotonicClock::new();
    let start = clock.now();
    clock
        .advance(Duration::from_secs(10))
        .expect("short advance should succeed");

    let Err(TimeError::CannotMoveBackward {
        current_elapsed,
        requested_elapsed,
    }) = clock.advance_to(start)
    else {
        panic!("backward target should report both elapsed values");
    };
    assert_eq!(Duration::from_secs(10), current_elapsed);
    assert_eq!(Duration::ZERO, requested_elapsed);
}

#[test]
fn test_manual_monotonic_clock_advance_to_rejects_foreign_domain() {
    let clock = ManualMonotonicClock::new();
    let foreign = ManualMonotonicClock::new().now();
    let expected = clock.now().domain();

    let Err(TimeError::ClockDomainMismatch {
        expected: actual_expected,
        actual,
    }) = clock.advance_to(foreign)
    else {
        panic!("foreign target should report a domain mismatch");
    };
    assert_eq!(expected, actual_expected);
    assert_eq!(foreign.domain(), actual);
}

#[test]
fn test_manual_monotonic_clock_instances_have_distinct_domains() {
    let first = ManualMonotonicClock::new();
    let second = ManualMonotonicClock::new();

    assert_ne!(first.now().domain(), second.now().domain());
}

#[test]
fn test_manual_monotonic_clock_default_starts_at_zero() {
    let clock = ManualMonotonicClock::default();
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
}

#[test]
fn test_manual_monotonic_clock_debug_includes_domain() {
    let clock = ManualMonotonicClock::new();
    assert!(format!("{clock:?}").contains("domain"));
}

#[test]
fn test_manual_monotonic_clock_zero_advance_is_noop() {
    let clock = ManualMonotonicClock::new();
    let before = clock.now();
    clock
        .advance(Duration::ZERO)
        .expect("zero advance should succeed");
    assert_eq!(before, clock.now());
}

#[test]
fn test_manual_monotonic_clock_reports_advance_overflow() {
    let clock = ManualMonotonicClock::new();
    clock
        .advance(Duration::MAX)
        .expect("maximum duration should fit from zero");
    assert!(matches!(
        clock.advance(Duration::from_nanos(1)),
        Err(TimeError::InstantOverflow),
    ));
}

#[test]
fn test_manual_monotonic_clock_advance_to_current_is_noop() {
    let clock = ManualMonotonicClock::new();
    let current = clock.now();
    clock
        .advance_to(current)
        .expect("advancing to current instant should succeed");
    assert_eq!(current, clock.now());
}

/// Verifies high-cardinality deadline bookkeeping across cancellation and due
/// futures that have not yet been polled for cleanup.
#[test]
fn test_manual_monotonic_clock_tracks_many_deadlines() {
    const EARLIEST_COUNT: usize = 64;
    const LATER_COUNT: usize = 192;

    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut cancelled = (0..EARLIEST_COUNT / 2)
        .map(|_| {
            timer
                .after(Duration::from_secs(1))
                .expect("earliest deadline should register")
        })
        .collect::<Vec<_>>();
    let mut due = (EARLIEST_COUNT / 2..EARLIEST_COUNT)
        .map(|_| {
            timer
                .after(Duration::from_secs(1))
                .expect("duplicate earliest deadline should register")
        })
        .collect::<Vec<_>>();
    let later = (0..LATER_COUNT)
        .map(|offset| {
            timer
                .after(Duration::from_secs(2 + (offset % 3) as u64))
                .expect("later deadline should register")
        })
        .collect::<Vec<_>>();

    cancelled.clear();
    assert_eq!(EARLIEST_COUNT / 2 + LATER_COUNT, clock.pending_waiters());
    assert_eq!(
        Some(
            clock
                .now()
                .checked_add(Duration::from_secs(1))
                .expect("earliest deadline should be representable"),
        ),
        clock.next_deadline(),
    );

    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    for future in &mut due {
        assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    }
    clock
        .advance(Duration::from_secs(1))
        .expect("manual clock should reach earliest deadline");
    assert_eq!(EARLIEST_COUNT / 2 + LATER_COUNT, clock.pending_waiters());
    assert_eq!(
        Some(
            clock
                .now()
                .checked_add(Duration::from_secs(1))
                .expect("next deadline should be representable"),
        ),
        clock.next_deadline(),
    );

    for future in &mut due {
        assert!(matches!(
            future.as_mut().poll(&mut context),
            Poll::Ready(Ok(())),
        ));
    }
    assert_eq!(LATER_COUNT, clock.pending_waiters());

    drop(later);
    assert_eq!(0, clock.pending_waiters());
    assert_eq!(None, clock.next_deadline());
}

/// Verifies a large shared-deadline population completes in one advance.
#[test]
fn test_manual_monotonic_clock_completes_many_waiters_at_one_deadline() {
    const WAITER_COUNT: usize = 65;

    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut futures = (0..WAITER_COUNT)
        .map(|_| {
            timer
                .after(Duration::from_secs(1))
                .expect("shared deadline should register")
        })
        .collect::<Vec<_>>();
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    for future in &mut futures {
        assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    }

    clock
        .advance(Duration::from_secs(1))
        .expect("manual clock should reach shared deadline");
    for future in &mut futures {
        assert!(matches!(
            future.as_mut().poll(&mut context),
            Poll::Ready(Ok(())),
        ));
    }
    assert_eq!(0, clock.pending_waiters());
}

/// Verifies that an existing waiter advances before guard representation.
#[test]
fn test_manual_monotonic_clock_advances_after_existing_waiter() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let _pending = timer
        .after(Duration::from_secs(5))
        .expect("manual deadline should register");

    let reached = clock
        .advance_to_next_deadline_after_waiters(1, Duration::MAX)
        .expect("existing waiter should advance immediately");

    assert_eq!(Duration::from_secs(5), reached.elapsed_since_origin());
    assert_eq!(reached, clock.now());
}

/// Verifies that the synchronous driver waits for a waiter registration.
#[test]
fn test_manual_monotonic_clock_waits_then_advances_after_waiter() {
    let clock = ManualMonotonicClock::new_shared();
    let driver_clock = Arc::clone(&clock);
    let (started_sender, started_receiver) = mpsc::channel();
    let driver = thread::spawn(move || {
        started_sender
            .send(())
            .expect("driver start should be observable");
        driver_clock.advance_to_next_deadline_after_waiters(1, Duration::from_secs(1))
    });
    started_receiver
        .recv()
        .expect("driver should start before registration");

    let timer = clock.new_timer();
    let _pending = timer
        .after(Duration::from_secs(4))
        .expect("manual deadline should register");
    let reached = driver
        .join()
        .expect("manual-time driver should finish")
        .expect("registered waiter should satisfy the driver");

    assert_eq!(Duration::from_secs(4), reached.elapsed_since_origin());
}

/// Verifies that the synchronous driver selects the earliest active deadline.
#[test]
fn test_manual_monotonic_clock_waiter_driver_selects_earliest_deadline() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let _later = timer
        .after(Duration::from_secs(7))
        .expect("later deadline should register");
    let _earlier = timer
        .after(Duration::from_secs(3))
        .expect("earlier deadline should register");

    let reached = clock
        .advance_to_next_deadline_after_waiters(2, Duration::ZERO)
        .expect("two existing waiters should satisfy the driver");

    assert_eq!(Duration::from_secs(3), reached.elapsed_since_origin());
}

/// Verifies that cancelled waiters do not satisfy the active-count condition.
#[test]
fn test_manual_monotonic_clock_waiter_driver_ignores_cancelled_waiters() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let cancelled = timer
        .after(Duration::from_secs(2))
        .expect("cancelled deadline should register");
    drop(cancelled);
    assert_eq!(0, clock.pending_waiters());

    let _first = timer
        .after(Duration::from_secs(5))
        .expect("first active deadline should register");
    assert_eq!(
        None,
        clock.advance_to_next_deadline_after_waiters(2, Duration::ZERO),
    );
    let _second = timer
        .after(Duration::from_secs(8))
        .expect("second active deadline should register");
    let reached = clock
        .advance_to_next_deadline_after_waiters(2, Duration::ZERO)
        .expect("two active waiters should satisfy the driver");

    assert_eq!(Duration::from_secs(5), reached.elapsed_since_origin());
}

/// Verifies that the synchronous driver returns without changing manual time.
#[test]
fn test_manual_monotonic_clock_waiter_driver_times_out() {
    let clock = ManualMonotonicClock::new();

    assert_eq!(
        None,
        clock.advance_to_next_deadline_after_waiters(1, Duration::ZERO),
    );
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
}

/// Verifies that a positive guard waits before returning without a waiter.
#[test]
fn test_manual_monotonic_clock_waiter_driver_waits_for_guard_timeout() {
    let clock = ManualMonotonicClock::new();

    assert_eq!(
        None,
        clock.advance_to_next_deadline_after_waiters(1, Duration::from_millis(100),),
    );
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
}

/// Verifies that an unrepresentable real-time guard is rejected.
#[test]
fn test_manual_monotonic_clock_waiter_driver_rejects_unrepresentable_guard() {
    let clock = ManualMonotonicClock::new();

    assert_eq!(
        None,
        clock.advance_to_next_deadline_after_waiters(1, Duration::MAX),
    );
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
}

/// Verifies that a zero count still requires an active future deadline.
#[test]
fn test_manual_monotonic_clock_zero_waiter_count_requires_deadline() {
    let clock = ManualMonotonicClock::new_shared();
    assert_eq!(
        None,
        clock.advance_to_next_deadline_after_waiters(0, Duration::ZERO),
    );

    let timer = clock.new_timer();
    let _pending = timer
        .after(Duration::from_secs(6))
        .expect("manual deadline should register");
    let reached = clock
        .advance_to_next_deadline_after_waiters(0, Duration::ZERO)
        .expect("zero count should advance once a deadline exists");

    assert_eq!(Duration::from_secs(6), reached.elapsed_since_origin());
}

#[tokio::test]
async fn test_manual_monotonic_clock_waits_and_advances_to_next_deadline_async() {
    let clock = ManualMonotonicClock::new_shared();
    let driver_clock = Arc::clone(&clock);
    let driver = tokio::spawn(async move { driver_clock.advance_to_next_deadline_async().await });
    tokio::task::yield_now().await;
    assert!(!driver.is_finished());

    let timer = clock.new_timer();
    let timer_future = timer
        .after(Duration::from_secs(5))
        .expect("manual deadline should register");
    let reached = driver.await.expect("manual-time driver should finish");

    assert_eq!(Duration::from_secs(5), reached.elapsed_since_origin());
    timer_future.await.expect("manual timer should complete");
}

#[test]
fn test_manual_monotonic_clock_async_driver_retries_after_deadline_cancellation() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut driver = pin!(clock.advance_to_next_deadline_async());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert_eq!(Poll::Pending, driver.as_mut().poll(&mut context));

    let cancelled = timer
        .after(Duration::from_secs(3))
        .expect("cancelled deadline should register");
    drop(cancelled);
    assert_eq!(Poll::Pending, driver.as_mut().poll(&mut context));
    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());

    let mut active = pin!(
        timer
            .after(Duration::from_secs(5))
            .expect("active deadline should register")
    );
    let expected = clock
        .next_deadline()
        .expect("active deadline should remain observable");
    assert_eq!(Poll::Ready(expected), driver.as_mut().poll(&mut context),);
    assert_eq!(Duration::from_secs(5), clock.now().elapsed_since_origin());
    assert!(matches!(
        active.as_mut().poll(&mut context),
        Poll::Ready(Ok(()))
    ));
}

#[test]
fn test_manual_monotonic_clock_async_driver_selects_current_earliest_deadline() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut driver = pin!(clock.advance_to_next_deadline_async());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert_eq!(Poll::Pending, driver.as_mut().poll(&mut context));

    let mut later = pin!(
        timer
            .after(Duration::from_secs(5))
            .expect("later deadline should register")
    );
    let mut earlier = pin!(
        timer
            .after(Duration::from_secs(2))
            .expect("earlier deadline should register")
    );

    let Poll::Ready(reached) = driver.as_mut().poll(&mut context) else {
        panic!("the async driver should reach the current earliest deadline");
    };
    assert_eq!(Duration::from_secs(2), reached.elapsed_since_origin());
    assert!(later.as_mut().poll(&mut context).is_pending());
    assert!(matches!(
        earlier.as_mut().poll(&mut context),
        Poll::Ready(Ok(()))
    ));
}

#[test]
fn test_manual_monotonic_clock_cancelled_async_driver_has_no_side_effects() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut driver = Box::pin(clock.advance_to_next_deadline_async());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert_eq!(Poll::Pending, driver.as_mut().poll(&mut context));

    let pending_timer = timer
        .after(Duration::from_secs(4))
        .expect("timer deadline should register");
    let expected_deadline = clock
        .next_deadline()
        .expect("timer deadline should remain observable");
    drop(driver);

    assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());
    assert_eq!(1, clock.pending_waiters());
    assert_eq!(Some(expected_deadline), clock.next_deadline());
    drop(pending_timer);
}

#[test]
fn test_manual_monotonic_clock_wait_for_waiters_times_out() {
    let clock = ManualMonotonicClock::new();
    assert!(!clock.wait_for_waiters(1, Duration::from_millis(1)));
}

/// Verifies that an already satisfied waiter count needs no real-time wait.
#[test]
fn test_manual_monotonic_clock_wait_for_waiters_is_already_satisfied() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let pending_sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");

    assert_eq!(1, clock.pending_waiters());
    assert!(clock.wait_for_waiters(1, Duration::ZERO));

    drop(pending_sleep);
    assert_eq!(0, clock.pending_waiters());
}

/// Verifies that an already satisfied waiter count takes precedence over an
/// unrepresentable real-time guard.
#[test]
fn test_manual_monotonic_clock_wait_for_waiters_prefers_satisfied_count() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let pending_sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");

    assert_eq!(1, clock.pending_waiters());
    assert!(clock.wait_for_waiters(1, Duration::MAX));

    drop(pending_sleep);
    assert_eq!(0, clock.pending_waiters());
}

/// Verifies that deadline coordination waits for a later registration after
/// the previous blocking waiter becomes due.
#[test]
fn test_manual_monotonic_clock_wait_for_next_deadline_tracks_retries() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let worker = thread::spawn(move || {
        sleeper
            .sleep_for(Duration::from_secs(1))
            .expect("first manual wait should complete");
        sleeper
            .sleep_for(Duration::from_secs(2))
            .expect("second manual wait should complete");
    });

    let first_deadline = clock
        .wait_for_next_deadline(Duration::from_secs(1))
        .expect("first deadline should be registered");
    assert_eq!(
        Duration::from_secs(1),
        first_deadline
            .duration_since(clock.now())
            .expect("first deadline should share the manual domain"),
    );
    clock
        .advance_to(first_deadline)
        .expect("manual time should reach the first deadline");

    let second_deadline = clock
        .wait_for_next_deadline(Duration::from_secs(1))
        .expect("second deadline should be registered");
    assert_eq!(
        Duration::from_secs(2),
        second_deadline
            .duration_since(clock.now())
            .expect("second deadline should share the manual domain"),
    );
    clock
        .advance_to(second_deadline)
        .expect("manual time should reach the second deadline");

    worker.join().expect("retry worker should finish");
}

/// Verifies that deadline coordination uses its real-time timeout as a guard.
#[test]
fn test_manual_monotonic_clock_wait_for_next_deadline_times_out() {
    let clock = ManualMonotonicClock::new();
    assert_eq!(None, clock.wait_for_next_deadline(Duration::ZERO));
    assert_eq!(None, clock.wait_for_next_deadline(Duration::from_millis(1)),);
    assert_eq!(None, clock.wait_for_next_deadline(Duration::MAX));
}

/// Verifies that an existing deadline takes precedence over an
/// unrepresentable real-time guard.
#[test]
fn test_manual_monotonic_clock_wait_for_next_deadline_prefers_existing() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let pending_sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");
    let expected_deadline = clock
        .next_deadline()
        .expect("pending sleep should register a deadline");

    assert_eq!(
        Some(expected_deadline),
        clock.wait_for_next_deadline(Duration::MAX),
    );

    drop(pending_sleep);
}

#[test]
fn test_manual_monotonic_clock_rejects_unrepresentable_guard_timeout() {
    let clock = ManualMonotonicClock::new();
    assert!(!clock.wait_for_waiters(1, Duration::MAX));
}
