// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    ManualMonotonicClock,
    MonotonicClock,
};
use std::sync::Arc;
use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use std::time::Duration;

/// Production source containing the subscription's public API declaration.
const SUBSCRIPTION_SOURCE: &str =
    include_str!("../../src/monotonic/manual_advance_subscription.rs");

#[test]
fn test_manual_advance_subscription_is_must_use() {
    assert!(
        SUBSCRIPTION_SOURCE.contains(
            "#[must_use = \"dropping the subscription unregisters the callback\"]\n\
             pub struct ManualAdvanceSubscription",
        ),
        "the RAII subscription handle must warn when its return value is ignored",
    );
}

#[test]
fn test_manual_advance_subscription_observes_time_changes() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let notifications = Arc::new(AtomicUsize::new(0));
    let observed_notifications = Arc::clone(&notifications);
    let _subscription = clock.subscribe_advances(move || {
        observed_notifications.fetch_add(1, Ordering::SeqCst);
    });

    clock
        .advance(Duration::from_secs(1))
        .expect("manual time should advance");
    let target = clock
        .now()
        .checked_add(Duration::from_secs(1))
        .expect("short deadline should fit");
    clock
        .advance_to(target)
        .expect("manual time should advance to target");
    clock
        .advance(Duration::ZERO)
        .expect("zero advance should be a no-op");

    assert_eq!(2, notifications.load(Ordering::SeqCst));
}

#[test]
fn test_manual_advance_subscription_unregisters_on_drop() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let notifications = Arc::new(AtomicUsize::new(0));
    let observed_notifications = Arc::clone(&notifications);
    let subscription = clock.subscribe_advances(move || {
        observed_notifications.fetch_add(1, Ordering::SeqCst);
    });
    drop(subscription);

    clock
        .advance(Duration::from_secs(1))
        .expect("manual time should advance");

    assert_eq!(0, notifications.load(Ordering::SeqCst));
}

#[test]
fn test_manual_advance_subscription_debug_and_expired_clock_drop() {
    let subscription = {
        let clock = Arc::new(ManualMonotonicClock::new());
        let subscription = clock.subscribe_advances(|| {});
        assert!(format!("{subscription:?}").contains("subscriber_id"));
        subscription
    };

    drop(subscription);
}

#[test]
fn test_manual_advance_subscription_callback_can_read_clock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let callback_clock = Arc::downgrade(&clock);
    let observed_elapsed = Arc::new(std::sync::Mutex::new(Duration::ZERO));
    let callback_elapsed = Arc::clone(&observed_elapsed);
    let _subscription = clock.subscribe_advances(move || {
        let clock = callback_clock
            .upgrade()
            .expect("clock should live while subscription is active");
        let mut elapsed = callback_elapsed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *elapsed = clock.now().elapsed_since_origin();
    });

    clock
        .advance(Duration::from_secs(3))
        .expect("manual time should advance");

    assert_eq!(
        Duration::from_secs(3),
        *observed_elapsed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
}

#[test]
fn test_manual_advance_subscription_runs_all_callbacks_before_resuming_panic() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let attempted_callbacks = Arc::new(AtomicUsize::new(0));
    let first_counter = Arc::clone(&attempted_callbacks);
    let second_counter = Arc::clone(&attempted_callbacks);
    let _first = clock.subscribe_advances(move || {
        first_counter.fetch_add(1, Ordering::SeqCst);
        panic!("first callback panic");
    });
    let _second = clock.subscribe_advances(move || {
        second_counter.fetch_add(1, Ordering::SeqCst);
        panic!("second callback panic");
    });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        clock.advance(Duration::from_secs(1))
    }));

    assert!(result.is_err());
    assert_eq!(2, attempted_callbacks.load(Ordering::SeqCst));
}

#[test]
fn test_manual_advance_subscription_drop_during_callback_blocks_later_calls() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let entered = Arc::new(std::sync::Barrier::new(2));
    let release = Arc::new(std::sync::Barrier::new(2));
    let callback_count = Arc::new(AtomicUsize::new(0));
    let callback_entered = Arc::clone(&entered);
    let callback_release = Arc::clone(&release);
    let observed_count = Arc::clone(&callback_count);
    let subscription = clock.subscribe_advances(move || {
        observed_count.fetch_add(1, Ordering::SeqCst);
        callback_entered.wait();
        callback_release.wait();
    });
    let advancing_clock = Arc::clone(&clock);
    let advance = std::thread::spawn(move || {
        advancing_clock
            .advance(Duration::from_secs(1))
            .expect("first advance should succeed");
    });
    entered.wait();

    drop(subscription);
    release.wait();
    advance.join().expect("advance thread should finish");
    clock
        .advance(Duration::from_secs(1))
        .expect("second advance should succeed");

    assert_eq!(1, callback_count.load(Ordering::SeqCst));
}
