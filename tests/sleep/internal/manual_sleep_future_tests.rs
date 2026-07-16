// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    AsyncSleeper,
    ManualAsyncSleeper,
    ManualMonotonicClock,
};
use std::sync::{
    Arc,
    Weak,
    atomic::{
        AtomicUsize,
        Ordering,
    },
    mpsc::{
        SyncSender,
        sync_channel,
    },
};
use std::task::{
    Context,
    Poll,
    Wake,
    Waker,
};
use std::time::Duration;

/// Re-enters its manual clock when the final registered waker is dropped.
struct ReentrantDropWaker {
    /// Clock whose state lock must already have been released.
    clock: Weak<ManualMonotonicClock>,
    /// Signals that the re-entrant destructor completed.
    drop_completed: SyncSender<()>,
}

#[allow(clippy::manual_noop_waker)]
impl Wake for ReentrantDropWaker {
    /// Ignores wake requests because these tests exercise only destruction.
    fn wake(self: Arc<Self>) {}
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

/// Counts task wake requests issued by the manual clock.
#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    /// Records one wake request.
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn test_manual_sleep_future_drop_unregisters_waiter() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let future = sleeper.sleep_for_async(Duration::from_secs(30));

    assert_eq!(1, clock.pending_waiters());
    drop(future);
    assert_eq!(0, clock.pending_waiters());
}

/// Verifies cancellation drops a registered custom waker outside the clock
/// state lock.
#[test]
fn test_manual_sleep_future_cancellation_drops_waker_outside_clock_lock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut future = sleeper.sleep_for_async(Duration::from_secs(30));
    let (drop_completed, drop_observer) = sync_channel(1);
    {
        let waker = Waker::from(Arc::new(ReentrantDropWaker {
            clock: Arc::downgrade(&clock),
            drop_completed,
        }));
        let mut context = Context::from_waker(&waker);
        assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    }

    let cancellation = std::thread::spawn(move || drop(future));
    drop_observer
        .recv_timeout(Duration::from_secs(1))
        .expect("custom waker drop should re-enter the unlocked clock");
    cancellation
        .join()
        .expect("future cancellation should finish");
    assert_eq!(0, clock.pending_waiters());
}

/// Verifies replacing a registered custom waker drops the old waker outside
/// the clock state lock.
#[test]
fn test_manual_sleep_future_replacement_drops_waker_outside_clock_lock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut future = sleeper.sleep_for_async(Duration::from_secs(30));
    let (drop_completed, drop_observer) = sync_channel(1);
    {
        let waker = Waker::from(Arc::new(ReentrantDropWaker {
            clock: Arc::downgrade(&clock),
            drop_completed,
        }));
        let mut context = Context::from_waker(&waker);
        assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    }

    let replacement = std::thread::spawn(move || {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
        future
    });
    drop_observer
        .recv_timeout(Duration::from_secs(1))
        .expect("replaced waker drop should re-enter the unlocked clock");
    let future = replacement.join().expect("replacement poll should finish");
    drop(future);
    assert_eq!(0, clock.pending_waiters());
}

#[test]
fn test_manual_sleep_future_reuses_registered_waker() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut future = sleeper.sleep_for_async(Duration::from_secs(30));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    assert_eq!(0, wake_counter.0.load(Ordering::Relaxed));
}

#[test]
fn test_manual_sleep_future_is_woken_once_after_deadline_is_reached() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut future = sleeper.sleep_for_async(Duration::from_secs(5));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending,));
    clock
        .advance(Duration::from_secs(5))
        .expect("deadline advance should succeed");
    assert_eq!(1, wake_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, clock.pending_waiters());

    clock
        .advance(Duration::from_secs(1))
        .expect("post-deadline advance should succeed");
    assert_eq!(1, wake_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, clock.pending_waiters());

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(Ok(())),
    ));
    assert_eq!(0, clock.pending_waiters());
}
