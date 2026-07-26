// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{
    Arc, Weak,
    mpsc::{SyncSender, sync_channel},
};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::Duration;

#[derive(Default)]
struct WakeCounter {
    wakes: AtomicUsize,
}

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
}

/// Re-enters its manual clock when the final observer waker is dropped.
struct ReentrantDropWaker {
    /// Clock whose state lock must already have been released.
    clock: Weak<ManualMonotonicClock>,
    /// Signals that the re-entrant destructor completed.
    drop_completed: SyncSender<()>,
}

impl Wake for ReentrantDropWaker {
    /// Ignores wake requests because this test exercises only destruction.
    fn wake(self: Arc<Self>) {
        drop(self);
    }
}

impl Drop for ReentrantDropWaker {
    /// Reads the clock during destruction and then signals completion.
    fn drop(&mut self) {
        if let Some(clock) = self.clock.upgrade() {
            let _ = clock.pending_waiters();
        }
        let _ = self.drop_completed.send(());
    }
}

#[test]
fn test_manual_deadline_future_returns_earliest_existing_deadline() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let _later = timer
        .after(Duration::from_secs(5))
        .expect("later deadline should register");
    let _earlier = timer
        .after(Duration::from_secs(2))
        .expect("earlier deadline should register");
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("existing future deadline should be ready");
    };
    assert_eq!(Duration::from_secs(2), deadline.elapsed_since_origin());
}

#[test]
fn test_manual_deadline_future_ignores_cancelled_deadline_before_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let cancelled = timer
        .after(Duration::from_secs(3))
        .expect("cancelled deadline should register");
    drop(cancelled);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));

    let _active = timer
        .after(Duration::from_secs(2))
        .expect("active deadline should register");
    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("the active future deadline should be ready");
    };
    assert_eq!(Duration::from_secs(2), deadline.elapsed_since_origin());
}

#[test]
fn test_manual_deadline_future_ignores_already_due_waiters() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let due = timer
        .after(Duration::from_secs(1))
        .expect("due deadline should register");
    clock
        .advance(Duration::from_secs(1))
        .expect("short manual advance should succeed");
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));

    let _future = timer
        .after(Duration::from_secs(2))
        .expect("future deadline should register");

    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("new future deadline should be ready");
    };
    assert_eq!(Duration::from_secs(3), deadline.elapsed_since_origin());
    drop(due);
}

#[test]
fn test_manual_deadline_future_wakes_when_deadline_registers() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let mut observer = Box::pin(clock.wait_for_next_deadline_async());
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));

    let _future = timer
        .after(Duration::from_secs(2))
        .expect("wake-triggering deadline should register");

    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert!(observer.as_mut().poll(&mut context).is_ready());
}

#[test]
fn test_manual_deadline_future_reuses_registered_waker() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut observer = Box::pin(clock.wait_for_next_deadline_async());

    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));
    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));
    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
}

#[test]
fn test_manual_deadline_future_replacement_drops_waker_outside_clock_lock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let mut observer = Box::pin(clock.wait_for_next_deadline_async());
    let (drop_completed, drop_observer) = sync_channel(1);
    {
        let waker = Waker::from(Arc::new(ReentrantDropWaker {
            clock: Arc::downgrade(&clock),
            drop_completed,
        }));
        let mut context = Context::from_waker(&waker);
        assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));
    }

    let replacement = thread::spawn(move || {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));
        observer
    });
    drop_observer
        .recv_timeout(Duration::from_secs(1))
        .expect("replaced observer waker should re-enter the unlocked clock");
    let observer = replacement
        .join()
        .expect("observer replacement poll should finish");
    drop(observer);
}

#[test]
fn test_manual_deadline_future_returns_earliest_deadline_at_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let _later = timer
        .after(Duration::from_secs(4))
        .expect("later deadline should register");
    let _earlier = timer
        .after(Duration::from_secs(1))
        .expect("earlier deadline should register");
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("an active future deadline should be ready");
    };
    assert_eq!(Duration::from_secs(1), deadline.elapsed_since_origin());
}

#[test]
fn test_manual_deadline_future_unregisters_on_drop() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut observer = Box::pin(clock.wait_for_next_deadline_async());
    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));
    drop(observer);

    let _future = timer
        .after(Duration::from_secs(2))
        .expect("post-drop deadline should register");

    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
}
