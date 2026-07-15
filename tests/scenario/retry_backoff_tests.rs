// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    AsyncSleeper,
    BlockingSleeper,
    ManualAsyncSleeper,
    ManualBlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
};
use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use std::thread;
use std::time::{
    Duration,
    Instant,
};

#[test]
fn test_retry_exponential_backoff_uses_no_real_delay() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper =
        Arc::new(ManualBlockingSleeper::from_clock(Arc::clone(&clock)));
    let attempts = Arc::new(AtomicUsize::new(0));
    let worker_sleeper = Arc::clone(&sleeper);
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
        clock
            .advance(Duration::from_secs(seconds))
            .expect("manual retry advance should succeed");
    }

    worker.join().expect("retry worker should not panic");
    assert_eq!(5, attempts.load(Ordering::SeqCst));
}

/// Waits until the retry worker registers the expected blocking deadline.
fn wait_for_blocking_deadline(
    clock: &ManualMonotonicClock,
    expected_deadline: qubit_clock::MonotonicInstant,
) {
    let real_deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < real_deadline {
        if clock.next_deadline() == Some(expected_deadline) {
            return;
        }
        thread::yield_now();
    }
    panic!("retry worker did not register the expected deadline");
}

#[tokio::test]
async fn test_async_retry_timeout_uses_manual_deadline() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = Arc::new(ManualAsyncSleeper::from_clock(Arc::clone(&clock)));
    let worker_sleeper = Arc::clone(&sleeper);
    let timeout_task = tokio::spawn(async move {
        tokio::select! {
            () = pending::<()>() => false,
            result = worker_sleeper
                .sleep_for_async(Duration::from_secs(30)) => {
                result.expect("manual timeout sleep should complete");
                true
            },
        }
    });

    clock.wait_for_waiters_async(1).await;
    clock
        .advance(Duration::from_secs(30))
        .expect("manual timeout advance should succeed");

    assert!(timeout_task.await.expect("timeout task should not panic"),);
}
