// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::support::manual_waiter_registry::ManualWaiterRegistry;
use crate::support::manual_waiter_registry::allocate_identifier;
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;

#[cfg(loom)]
use loom::sync::{
    Arc,
    Mutex,
};
#[cfg(loom)]
use loom::thread;

#[test]
fn test_manual_registry_latches_reached_observer_after_waiter_unregisters() {
    let mut registry = ManualWaiterRegistry::new();
    let observer_id = registry
        .register_observer(1, registry.count())
        .expect("an unsatisfied observer should be registered");
    let waiter_id = registry.register_timer(Duration::from_secs(1));

    assert!(registry.reached_observer_wakers(Duration::ZERO).is_empty());
    assert!(registry.unregister_timer(waiter_id).is_some());

    assert!(!registry.contains_observer(observer_id));
    assert_eq!(0, registry.count());
}

#[test]
fn test_manual_registry_poll_observer_becomes_ready_after_reaching_count() {
    let mut registry = ManualWaiterRegistry::new();
    let observer_id = registry
        .register_observer(1, registry.count())
        .expect("an unsatisfied observer should be registered");
    let context = Context::from_waker(Waker::noop());

    let (poll, replaced_waker) = registry.poll_observer(observer_id, &context);
    assert_eq!(Poll::Pending, poll);
    assert!(replaced_waker.is_none());

    let _ = registry.register_timer(Duration::from_secs(1));
    let wakers = registry.reached_observer_wakers(Duration::ZERO);
    let (poll, removed_waker) = registry.poll_observer(observer_id, &context);
    assert_eq!(Poll::Ready(()), poll);
    assert_eq!(1, wakers.len());
    assert!(removed_waker.is_none());
    assert!(!registry.contains_observer(observer_id));
}

#[test]
fn test_manual_registry_keeps_observer_below_expected_count() {
    let mut registry = ManualWaiterRegistry::new();
    let observer_id = registry
        .register_observer(2, registry.count())
        .expect("an unsatisfied observer should be registered");
    let _ = registry.register_timer(Duration::from_secs(1));

    assert!(registry.reached_observer_wakers(Duration::ZERO).is_empty());
    assert!(registry.contains_observer(observer_id));
}

#[test]
fn test_manual_registry_deadline_observer_tracks_current_deadline() {
    let mut registry = ManualWaiterRegistry::new();
    let observer_id = registry.register_deadline_observer();
    let context = Context::from_waker(Waker::noop());

    let (poll, replaced_waker) =
        registry.poll_deadline_observer(observer_id, Duration::ZERO, &context);
    assert_eq!(Poll::Pending, poll);
    assert!(replaced_waker.is_none());

    let cancelled = registry.register_timer(Duration::from_secs(2));
    assert_eq!(1, registry.reached_observer_wakers(Duration::ZERO).len());
    assert!(registry.unregister_timer(cancelled).is_some());

    let (poll, replaced_waker) =
        registry.poll_deadline_observer(observer_id, Duration::ZERO, &context);
    assert_eq!(Poll::Pending, poll);
    assert!(replaced_waker.is_none());

    let _ = registry.register_timer(Duration::from_secs(3));
    assert_eq!(1, registry.reached_observer_wakers(Duration::ZERO).len());

    let (poll, removed_waker) =
        registry.poll_deadline_observer(observer_id, Duration::ZERO, &context);
    assert_eq!(Poll::Ready(Duration::from_secs(3)), poll);
    assert!(removed_waker.is_none());
}

#[test]
fn test_manual_registry_deadline_observer_returns_existing_deadline() {
    let mut registry = ManualWaiterRegistry::new();
    let _ = registry.register_timer(Duration::from_secs(3));
    let observer_id = registry.register_deadline_observer();
    let context = Context::from_waker(Waker::noop());

    let (poll, removed_waker) =
        registry.poll_deadline_observer(observer_id, Duration::ZERO, &context);

    assert_eq!(Poll::Ready(Duration::from_secs(3)), poll);
    assert!(removed_waker.is_none());
}

#[test]
fn test_manual_registry_unregisters_pending_deadline_observer() {
    let mut registry = ManualWaiterRegistry::new();
    let observer_id = registry.register_deadline_observer();
    let context = Context::from_waker(Waker::noop());
    let (poll, replaced_waker) =
        registry.poll_deadline_observer(observer_id, Duration::ZERO, &context);
    assert_eq!(Poll::Pending, poll);
    assert!(replaced_waker.is_none());

    assert!(registry.unregister_observer(observer_id).is_some());
    assert!(registry.unregister_observer(observer_id).is_none());
}

#[test]
#[should_panic(expected = "manual deadline observer 1 is not registered")]
fn test_manual_registry_rejects_missing_deadline_observer() {
    let mut registry = ManualWaiterRegistry::new();
    let context = Context::from_waker(Waker::noop());

    let _ = registry.poll_deadline_observer(1, Duration::ZERO, &context);
}

#[test]
fn test_manual_registry_identifier_allocates_maximum_before_exhaustion() {
    let mut next_identifier = u64::MAX;

    assert_eq!(
        u64::MAX,
        allocate_identifier(&mut next_identifier, "identifiers exhausted"),
    );
    assert_eq!(0, next_identifier);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        allocate_identifier(&mut next_identifier, "identifiers exhausted")
    }));
    assert!(result.is_err());
}

/// Verifies that a lost timer registration fails instead of hanging forever.
#[test]
#[should_panic(expected = "manual timer waiter 1 is not registered")]
fn test_manual_registry_poll_timer_panics_for_missing_waiter() {
    let mut registry = ManualWaiterRegistry::new();
    let waiter_id = registry.register_timer(Duration::from_secs(1));
    assert!(registry.unregister_timer(waiter_id).is_some());
    let context = Context::from_waker(Waker::noop());

    let _ = registry.poll_timer(waiter_id, Duration::ZERO, &context);
}

/// Models the race between advancing a due timer and cancelling its future.
#[cfg(loom)]
#[test]
fn manual_waiter_registry_model_advance_races_with_timer_cancellation() {
    loom::model(|| {
        let mut registry = ManualWaiterRegistry::new();
        let waiter_id = registry.register_timer(Duration::from_secs(1));
        let context = Context::from_waker(Waker::noop());
        let (poll, replaced_waker) =
            registry.poll_timer(waiter_id, Duration::ZERO, &context);
        assert_eq!(Poll::Pending, poll);
        assert!(replaced_waker.is_none());
        let registry = Arc::new(Mutex::new(registry));

        let advancing_registry = Arc::clone(&registry);
        let advance = thread::spawn(move || {
            advancing_registry
                .lock()
                .expect("the model registry lock should remain usable")
                .take_due_timer_wakers(Duration::from_secs(1))
                .len()
        });
        let cancelling_registry = Arc::clone(&registry);
        let cancel = thread::spawn(move || {
            cancelling_registry
                .lock()
                .expect("the model registry lock should remain usable")
                .unregister_timer(waiter_id)
                .flatten()
                .is_some()
        });

        let advance_owned_waker =
            advance.join().expect("advance model thread should finish");
        let cancel_owned_waker =
            cancel.join().expect("cancel model thread should finish");
        assert_eq!(1, advance_owned_waker + usize::from(cancel_owned_waker));
        assert_eq!(
            0,
            registry
                .lock()
                .expect("the model registry lock should remain usable")
                .count(),
        );
    });
}

/// Models deadline publication racing with cancellation of its observer.
#[cfg(loom)]
#[test]
fn manual_waiter_registry_model_registration_races_with_observer_cancellation()
{
    loom::model(|| {
        let mut registry = ManualWaiterRegistry::new();
        let observer_id = registry.register_deadline_observer();
        let context = Context::from_waker(Waker::noop());
        let (poll, replaced_waker) = registry.poll_deadline_observer(
            observer_id,
            Duration::ZERO,
            &context,
        );
        assert_eq!(Poll::Pending, poll);
        assert!(replaced_waker.is_none());
        let registry = Arc::new(Mutex::new(registry));

        let registering_registry = Arc::clone(&registry);
        let register = thread::spawn(move || {
            let mut registry = registering_registry
                .lock()
                .expect("the model registry lock should remain usable");
            let _ = registry.register_timer(Duration::from_secs(1));
            registry.reached_observer_wakers(Duration::ZERO).len()
        });
        let cancelling_registry = Arc::clone(&registry);
        let cancel = thread::spawn(move || {
            cancelling_registry
                .lock()
                .expect("the model registry lock should remain usable")
                .unregister_observer(observer_id)
                .is_some()
        });

        let register_owned_waker = register
            .join()
            .expect("register model thread should finish");
        let cancel_owned_waker =
            cancel.join().expect("cancel model thread should finish");
        assert_eq!(1, register_owned_waker + usize::from(cancel_owned_waker));
        assert!(
            registry
                .lock()
                .expect("the model registry lock should remain usable")
                .unregister_observer(observer_id)
                .is_none(),
        );
    });
}
