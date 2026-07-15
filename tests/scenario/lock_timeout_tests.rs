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
use std::sync::{
    Arc,
    Condvar,
    Mutex,
};
use std::thread;
use std::time::{
    Duration,
    Instant,
};

#[test]
fn test_blocking_lock_timeout_is_driven_by_manual_time() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper =
        Arc::new(ManualBlockingSleeper::from_clock(Arc::clone(&clock)));
    let worker_sleeper = Arc::clone(&sleeper);
    let worker = thread::spawn(move || {
        worker_sleeper
            .sleep_for(Duration::from_secs(20))
            .expect("manual lock timeout should complete");
        true
    });

    assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
    clock
        .advance(Duration::from_secs(20))
        .expect("manual timeout advance should succeed");

    assert!(worker.join().expect("lock waiter should not panic"));
}

#[tokio::test]
async fn test_async_lock_timeout_is_driven_by_manual_time() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let wait = sleeper.sleep_for_async(Duration::from_secs(20));
    assert_eq!(1, clock.pending_waiters());

    clock
        .advance(Duration::from_secs(20))
        .expect("manual timeout advance should succeed");
    wait.await
        .expect("manual async lock timeout should complete");
}

#[test]
fn test_mock_monitor_timeout_observes_manual_time_advance() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let monitor_state = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let advance_notification_state = Arc::clone(&monitor_state);
    let _subscription = clock.subscribe_advances(move || {
        let (lock, condition) = &*advance_notification_state;
        let _state = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        condition.notify_all();
    });
    let worker_clock = Arc::clone(&clock);
    let worker_state = Arc::clone(&monitor_state);
    let worker = thread::spawn(move || {
        let deadline = worker_clock
            .now()
            .checked_add(Duration::from_secs(20))
            .expect("short deadline should fit");
        let (lock, condition) = &*worker_state;
        let mut state = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.1 = true;
        condition.notify_all();
        while !state.0 && worker_clock.now() < deadline {
            state = condition
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        !state.0 && worker_clock.now() >= deadline
    });

    let (lock, condition) = &*monitor_state;
    let real_deadline = Instant::now() + Duration::from_secs(1);
    let mut state = lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    while !state.1 {
        let remaining = real_deadline.saturating_duration_since(Instant::now());
        let (next_state, result) = condition
            .wait_timeout(state, remaining)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state = next_state;
        assert!(!result.timed_out(), "mock monitor did not begin waiting");
    }
    drop(state);

    clock
        .advance(Duration::from_secs(20))
        .expect("manual timeout advance should succeed");

    assert!(worker.join().expect("mock monitor should not panic"));
}
