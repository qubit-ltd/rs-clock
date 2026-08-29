// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use qubit_clock::BlockingSleeper;
use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::MonotonicInstant;
use qubit_clock::Timer;

#[test]
fn test_retry_exponential_backoff_uses_no_real_delay() {
    let clock = ManualMonotonicClock::new_shared();
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let attempts = Arc::new(AtomicUsize::new(0));
    let worker_sleeper = sleeper.clone();
    let worker_attempts = Arc::clone(&attempts);
    let delays = [1_u64, 2, 4, 8];
    let worker = thread::spawn(move || {
        for seconds in delays {
            worker_attempts.fetch_add(1, Ordering::SeqCst);
            worker_sleeper
                .sleep_for(Duration::from_secs(seconds))
                .expect("manual retry delay should complete");
        }
        worker_attempts.fetch_add(1, Ordering::SeqCst);
    });

    for (index, seconds) in delays.into_iter().enumerate() {
        let expected_deadline = clock
            .now()
            .checked_add(Duration::from_secs(seconds))
            .expect("retry deadline should be representable");
        wait_for_blocking_deadline(&clock, expected_deadline);
        assert_eq!(index + 1, attempts.load(Ordering::SeqCst));
        assert_eq!(
            Some(expected_deadline),
            clock.advance_to_next_deadline(),
            "the observed retry deadline should remain active",
        );
    }

    worker.join().expect("retry worker should not panic");
    assert_eq!(5, attempts.load(Ordering::SeqCst));
}

/// Waits until the retry worker registers the expected blocking deadline.
///
/// # Parameters
///
/// * `clock` - Manual clock shared with the retry worker.
/// * `expected_deadline` - Deadline the worker is expected to register.
///
/// # Panics
///
/// Panics if the expected deadline is not registered before the real-time
/// coordination guard expires.
fn wait_for_blocking_deadline(clock: &ManualMonotonicClock, expected_deadline: MonotonicInstant) {
    assert_eq!(
        Some(expected_deadline),
        clock.wait_for_next_deadline(Duration::from_secs(1)),
        "retry worker did not register the expected deadline",
    );
}

#[tokio::test]
async fn test_async_retry_timeout_uses_manual_deadline() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let worker_timer = Arc::clone(&timer);
    let timeout_task = tokio::spawn(async move {
        tokio::select! {
            () = pending::<()>() => false,
            result = worker_timer
                .after(Duration::from_secs(30))
                .expect("manual timeout should register") => {
                result.expect("manual timeout should complete");
                true
            },
        }
    });

    let observed = clock.wait_for_next_deadline_async().await;
    assert_eq!(Duration::from_secs(30), observed.elapsed_since_origin());
    assert_eq!(
        Some(observed),
        clock.advance_to_next_deadline(),
        "the observed timeout deadline should remain active",
    );

    assert!(timeout_task.await.expect("timeout task should not panic"),);
}
