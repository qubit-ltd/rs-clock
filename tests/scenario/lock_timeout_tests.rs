// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{BlockingSleeper, ManualMonotonicClock, MonotonicClock, Timer};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_blocking_lock_timeout_is_driven_by_manual_time() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let worker_sleeper = sleeper.clone();
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
    let timer = clock.new_timer();
    let wait = timer
        .after(Duration::from_secs(20))
        .expect("manual timeout should register");
    assert_eq!(1, clock.pending_waiters());

    clock
        .advance(Duration::from_secs(20))
        .expect("manual timeout advance should succeed");
    wait.await.expect("manual timeout should complete");
}
