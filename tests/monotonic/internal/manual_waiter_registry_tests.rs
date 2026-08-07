// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[cfg(all(loom, feature = "loom-model"))]
use std::task::Context;
#[cfg(all(loom, feature = "loom-model"))]
use std::task::Poll;
#[cfg(all(loom, feature = "loom-model"))]
use std::task::Waker;
#[cfg(all(loom, feature = "loom-model"))]
use std::time::Duration;

#[cfg(all(loom, feature = "loom-model"))]
use loom::model;
#[cfg(all(loom, feature = "loom-model"))]
use loom::sync::Arc;
#[cfg(all(loom, feature = "loom-model"))]
use loom::sync::Mutex;
#[cfg(all(loom, feature = "loom-model"))]
use loom::thread;
#[cfg(all(loom, feature = "loom-model"))]
use qubit_clock::test_util::loom::LoomManualWaiterRegistry;

/// Models the race between advancing a due timer and cancelling its future.
#[cfg(all(loom, feature = "loom-model"))]
#[test]
fn test_loom_manual_waiter_registry_advance_races_with_cancellation() {
    model(|| {
        let mut registry = LoomManualWaiterRegistry::new();
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
#[cfg(all(loom, feature = "loom-model"))]
#[test]
fn test_loom_manual_waiter_registry_publication_races_with_observer_cancellation()
 {
    model(|| {
        let mut registry = LoomManualWaiterRegistry::new();
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
