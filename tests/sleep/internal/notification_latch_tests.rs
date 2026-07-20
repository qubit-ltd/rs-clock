// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[cfg(loom)]
use crate::common::notification_latch::NotificationLatch;
#[cfg(loom)]
use loom::sync::Arc;
#[cfg(loom)]
use loom::thread;

/// Verifies a notification racing with a take is observed exactly once.
#[cfg(loom)]
#[test]
fn test_notification_latch_model_preserves_notify_racing_with_take() {
    loom::model(|| {
        let latch = Arc::new(NotificationLatch::new());
        let take_latch = Arc::clone(&latch);
        let notify_latch = Arc::clone(&latch);
        let taker = thread::spawn(move || take_latch.take_notification());
        let notifier = thread::spawn(move || notify_latch.notify());

        let observed_while_racing = taker.join().expect("taker should finish");
        notifier.join().expect("notifier should finish");
        let observed_after_join = latch.take_notification();

        assert_ne!(
            observed_while_racing, observed_after_join,
            "the racing notification must be observed exactly once: racing={observed_while_racing}, after_join={observed_after_join}",
        );
    });
}

/// Verifies concurrent notifications coalesce into one latched observation.
#[cfg(loom)]
#[test]
fn test_notification_latch_model_coalesces_concurrent_notifications() {
    loom::model(|| {
        let latch = Arc::new(NotificationLatch::new());
        let first_latch = Arc::clone(&latch);
        let second_latch = Arc::clone(&latch);
        let first = thread::spawn(move || first_latch.notify());
        let second = thread::spawn(move || second_latch.notify());

        first.join().expect("first notifier should finish");
        second.join().expect("second notifier should finish");

        assert!(latch.take_notification());
        assert!(!latch.take_notification());
        latch.notify();
        latch.clear_notification();
        assert!(!latch.take_notification());
    });
}
