// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use std::time::Duration;

use qubit_clock::BlockingSleeper;
use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::Timer;

#[tokio::test]
async fn test_manual_time_domain_drives_mixed_waiters_in_deadline_order() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let blocking_sleeper = Arc::new(BlockingSleeper::new(Arc::clone(&timer)));
    let async_wait = timer
        .after(Duration::from_secs(2))
        .expect("timer deadline should register");
    let worker_sleeper = Arc::clone(&blocking_sleeper);
    let worker = thread::spawn(move || {
        worker_sleeper
            .sleep_for(Duration::from_secs(5))
            .expect("blocking wait should complete");
    });
    assert!(clock.wait_for_waiters(2, Duration::from_secs(1)));

    assert_eq!(2, clock.pending_waiters());
    assert_eq!(
        Duration::from_secs(2),
        clock
            .advance_to_next_deadline()
            .expect("async deadline should exist")
            .elapsed_since_origin(),
    );
    async_wait.await.expect("manual timer should complete");
    assert_eq!(1, clock.pending_waiters());
    assert_eq!(
        Duration::from_secs(5),
        clock
            .advance_to_next_deadline()
            .expect("blocking deadline should exist")
            .elapsed_since_origin(),
    );
    worker.join().expect("blocking waiter should finish");
    assert_eq!(None, clock.advance_to_next_deadline());
}

#[test]
fn test_manual_time_domain_preserves_concurrent_advances() {
    const THREADS: usize = 8;
    const ADVANCES_PER_THREAD: usize = 100;
    let clock = Arc::new(ManualMonotonicClock::new());
    let barrier = Arc::new(Barrier::new(THREADS));
    let workers: Vec<_> = (0..THREADS)
        .map(|_| {
            let clock = Arc::clone(&clock);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..ADVANCES_PER_THREAD {
                    clock
                        .advance(Duration::from_nanos(1))
                        .expect("concurrent advance should succeed");
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("advance worker should finish");
    }

    assert_eq!(
        Duration::from_nanos((THREADS * ADVANCES_PER_THREAD) as u64),
        clock.now().elapsed_since_origin(),
    );
}
