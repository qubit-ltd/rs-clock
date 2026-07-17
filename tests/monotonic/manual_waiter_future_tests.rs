// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{
    BlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    Timer,
};
use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use std::sync::{
    Arc,
    Weak,
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

/// Panics whenever the manual clock attempts to wake its task.
struct PanicWaker;

impl Wake for PanicWaker {
    /// Simulates a task whose custom waker panics.
    fn wake(self: Arc<Self>) {
        panic!("observer waker panic");
    }
}

#[test]
fn test_manual_waiter_future_is_ready_for_zero_expected_count() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let mut waiter = pin!(clock.wait_for_waiters_async(0));
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
}

#[test]
fn test_manual_waiter_future_is_ready_when_count_is_already_satisfied() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");
    let mut waiter = pin!(clock.wait_for_waiters_async(1));
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
    drop(sleep);
}

#[test]
fn test_manual_waiter_future_observes_blocking_adapter_registration() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));

    let worker = thread::spawn(move || {
        sleeper
            .sleep_for(Duration::from_secs(1))
            .expect("manual blocking sleep should complete");
    });
    assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
    clock
        .advance(Duration::from_secs(1))
        .expect("short manual advance should succeed");
    worker.join().expect("blocking waiter should finish");

    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
}

#[test]
fn test_manual_waiter_future_wakes_when_expected_waiter_registers() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let mut waiter = pin!(clock.wait_for_waiters_async(1));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));

    let sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");

    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
    drop(sleep);
}

#[test]
fn test_manual_waiter_future_replaces_registered_waker() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let first_counter = Arc::new(WakeCounter::default());
    let second_counter = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_counter));
    let second_waker = Waker::from(Arc::clone(&second_counter));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut first_context));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut second_context));

    let sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");

    assert_eq!(0, first_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(1, second_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut second_context));
    drop(sleep);
}

/// Verifies replacing an observer waker drops the old waker outside the clock
/// state lock.
#[test]
fn test_manual_waiter_future_replacement_drops_waker_outside_clock_lock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    let (drop_completed, drop_observer) = sync_channel(1);
    {
        let waker = Waker::from(Arc::new(ReentrantDropWaker {
            clock: Arc::downgrade(&clock),
            drop_completed,
        }));
        let mut context = Context::from_waker(&waker);
        assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    }

    let replacement = thread::spawn(move || {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
        waiter
    });
    drop_observer
        .recv_timeout(Duration::from_secs(1))
        .expect("replaced observer waker should re-enter the unlocked clock");
    let waiter = replacement
        .join()
        .expect("observer replacement poll should finish");
    drop(waiter);
}

/// Verifies that polling with the same waker keeps the observer pending
/// without replacing its registration.
#[test]
fn test_manual_waiter_future_reuses_registered_waker() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));

    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
}

#[test]
fn test_manual_waiter_future_unregisters_on_drop() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    drop(waiter);

    let sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");

    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
    drop(sleep);
}

/// Verifies observer cancellation drops its registered waker outside the clock
/// state lock.
#[test]
fn test_manual_waiter_future_cancellation_drops_waker_outside_clock_lock() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    let (drop_completed, drop_observer) = sync_channel(1);
    {
        let waker = Waker::from(Arc::new(ReentrantDropWaker {
            clock: Arc::downgrade(&clock),
            drop_completed,
        }));
        let mut context = Context::from_waker(&waker);
        assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));
    }

    let cancellation = thread::spawn(move || drop(waiter));
    drop_observer
        .recv_timeout(Duration::from_secs(1))
        .expect("observer waker drop should re-enter the unlocked clock");
    cancellation
        .join()
        .expect("observer cancellation should finish");
}

#[test]
fn test_manual_waiter_future_latches_reached_count_before_waiter_drops() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut waiter = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(Poll::Pending, waiter.as_mut().poll(&mut context));

    let sleep = timer
        .after(Duration::from_secs(1))
        .expect("timer deadline should register");
    drop(sleep);

    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(Poll::Ready(()), waiter.as_mut().poll(&mut context));
}

#[test]
fn test_timer_waiter_registration_cleans_up_after_observer_waker_panics() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();
    let panic_waker = Waker::from(Arc::new(PanicWaker));
    let wake_counter = Arc::new(WakeCounter::default());
    let counting_waker = Waker::from(Arc::clone(&wake_counter));
    let mut panic_context = Context::from_waker(&panic_waker);
    let mut counting_context = Context::from_waker(&counting_waker);
    let mut panic_observer = Box::pin(clock.wait_for_waiters_async(1));
    let mut counting_observer = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(
        Poll::Pending,
        panic_observer.as_mut().poll(&mut panic_context),
    );
    assert_eq!(
        Poll::Pending,
        counting_observer.as_mut().poll(&mut counting_context),
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        timer.after(Duration::from_secs(1))
    }));

    assert!(result.is_err());
    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(0, clock.pending_waiters());
    assert_eq!(
        Poll::Ready(()),
        panic_observer.as_mut().poll(&mut panic_context),
    );
    assert_eq!(
        Poll::Ready(()),
        counting_observer.as_mut().poll(&mut counting_context),
    );
}

#[test]
fn test_blocking_adapter_registration_cleans_up_after_observer_waker_panics() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let panic_waker = Waker::from(Arc::new(PanicWaker));
    let wake_counter = Arc::new(WakeCounter::default());
    let counting_waker = Waker::from(Arc::clone(&wake_counter));
    let mut panic_context = Context::from_waker(&panic_waker);
    let mut counting_context = Context::from_waker(&counting_waker);
    let mut panic_observer = Box::pin(clock.wait_for_waiters_async(1));
    let mut counting_observer = Box::pin(clock.wait_for_waiters_async(1));
    assert_eq!(
        Poll::Pending,
        panic_observer.as_mut().poll(&mut panic_context),
    );
    assert_eq!(
        Poll::Pending,
        counting_observer.as_mut().poll(&mut counting_context),
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sleeper.sleep_for(Duration::from_secs(1))
    }));

    assert!(result.is_err());
    assert_eq!(1, wake_counter.wakes.load(Ordering::SeqCst));
    assert_eq!(0, clock.pending_waiters());
    assert_eq!(
        Poll::Ready(()),
        panic_observer.as_mut().poll(&mut panic_context),
    );
    assert_eq!(
        Poll::Ready(()),
        counting_observer.as_mut().poll(&mut counting_context),
    );
}
