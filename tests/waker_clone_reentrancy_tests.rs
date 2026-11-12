// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::future::Future;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::Context;
use std::task::RawWaker;
use std::task::RawWakerVTable;
use std::task::Waker;
use std::time::Duration;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::Timer;

const CHILD_MARKER: &str = "QUBIT_CLOCK_REENTRANT_WAKER_CHILD";
const CHILD_TEST: &str = "test_clone_callback_reenters_manual_clock_in_child";

struct CallbackData {
    clock: Weak<ManualMonotonicClock>,
    clones: AtomicUsize,
    drops: AtomicUsize,
}

impl CallbackData {
    fn reenter(&self) {
        if let Some(clock) = self.clock.upgrade() {
            let _ = clock.now();
            let _ = clock.pending_waiters();
        }
    }
}

unsafe fn clone_raw(data: *const ()) -> RawWaker {
    // SAFETY: `data` is an Arc pointer created by `reentrant_waker`; the
    // ManuallyDrop preserves the original RawWaker's strong reference.
    let data = std::mem::ManuallyDrop::new(unsafe { Arc::from_raw(data.cast::<CallbackData>()) });
    data.reenter();
    data.clones.fetch_add(1, Ordering::SeqCst);
    let cloned = Arc::clone(&data);
    RawWaker::new(Arc::into_raw(cloned).cast(), &VTABLE)
}

unsafe fn wake_raw(data: *const ()) {
    // SAFETY: wake consumes exactly the strong reference transferred by RawWaker.
    unsafe { drop_raw(data) };
}

unsafe fn wake_by_ref_raw(data: *const ()) {
    // SAFETY: the caller guarantees a live RawWaker reference for this callback.
    let data = unsafe { &*data.cast::<CallbackData>() };
    data.reenter();
}

unsafe fn drop_raw(data: *const ()) {
    // SAFETY: `data` is the Arc pointer owned by this RawWaker reference.
    let data = unsafe { Arc::from_raw(data.cast::<CallbackData>()) };
    data.reenter();
    data.drops.fetch_add(1, Ordering::SeqCst);
    drop(data);
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_raw, wake_raw, wake_by_ref_raw, drop_raw);

fn reentrant_waker(clock: &Arc<ManualMonotonicClock>) -> (Waker, Arc<CallbackData>) {
    let data = Arc::new(CallbackData {
        clock: Arc::downgrade(clock),
        clones: AtomicUsize::new(0),
        drops: AtomicUsize::new(0),
    });
    // SAFETY: every vtable operation follows RawWaker's Arc ownership contract.
    let waker = unsafe { Waker::from_raw(RawWaker::new(Arc::into_raw(Arc::clone(&data)).cast(), &VTABLE)) };
    (waker, data)
}

#[test]
fn test_clone_callback_reenters_manual_clock_in_child() {
    if std::env::var_os(CHILD_MARKER).is_none() {
        let output = Command::new(std::env::current_exe().expect("test executable path"))
            .arg("--exact")
            .arg(CHILD_TEST)
            .arg("--nocapture")
            .env(CHILD_MARKER, "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("child test should start");
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut child = output;
        loop {
            if let Some(status) = child.try_wait().expect("child status should be readable") {
                let output = child.wait_with_output().expect("child output should be collected");
                assert!(
                    status.success(),
                    "child failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                    "child test was not confirmed: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                return;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().expect("timed out child should be killed");
                let output = child.wait_with_output().expect("killed child should be reaped");
                panic!(
                    "child deadlocked during Waker callback: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut sleep = Box::pin(timer.after(Duration::from_secs(1)).expect("timer should register"));
    let mut waiter_observer = Box::pin(clock.wait_for_waiters_async(2));
    let mut deadline_observer = Box::pin(clock.wait_for_next_deadline_async());
    let (waker, data) = reentrant_waker(&clock);
    let mut context = Context::from_waker(&waker);

    assert!(sleep.as_mut().poll(&mut context).is_pending());
    // Polling again with the same task waker exercises the will_wake hit path;
    // the unused candidate must still be dropped after releasing the clock lock.
    assert!(sleep.as_mut().poll(&mut context).is_pending());
    assert!(waiter_observer.as_mut().poll(&mut context).is_pending());
    assert!(deadline_observer.as_mut().poll(&mut context).is_ready());
    assert!(data.clones.load(Ordering::SeqCst) >= 2);

    let empty_clock = ManualMonotonicClock::new_shared();
    let mut pending_deadline = Box::pin(empty_clock.wait_for_next_deadline_async());
    let (pending_waker, _) = reentrant_waker(&empty_clock);
    let mut pending_context = Context::from_waker(&pending_waker);
    assert!(pending_deadline.as_mut().poll(&mut pending_context).is_pending());
    drop(pending_deadline);
    drop(pending_waker);

    drop(deadline_observer);
    drop(waiter_observer);
    drop(sleep);
    drop(waker);
    assert!(data.drops.load(Ordering::SeqCst) > 0);
}

unsafe fn panic_clone(_: *const ()) -> RawWaker {
    panic!("intentional RawWaker clone panic");
}

unsafe fn noop_wake(_: *const ()) {}
unsafe fn noop_drop(_: *const ()) {}

static PANICKING_VTABLE: RawWakerVTable = RawWakerVTable::new(panic_clone, noop_wake, noop_wake, noop_drop);

fn panicking_clone_waker() -> Waker {
    // SAFETY: the vtable's no-op consume/drop callbacks own no data, and the
    // clone callback is permitted to panic before producing a new RawWaker.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &PANICKING_VTABLE)) }
}

#[test]
fn test_clone_panic_leaves_manual_and_standard_timer_waiters_usable() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let mut manual = Box::pin(
        timer
            .after(Duration::from_secs(1))
            .expect("manual timer should register"),
    );
    let panic_waker = panicking_clone_waker();
    let mut panic_context = Context::from_waker(&panic_waker);
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = manual.as_mut().poll(&mut panic_context);
    }));
    assert!(panicked.is_err());
    let _ = clock.now();
    let noop = Waker::noop();
    let mut context = Context::from_waker(noop);
    assert!(manual.as_mut().poll(&mut context).is_pending());
    clock
        .advance(Duration::from_secs(1))
        .expect("manual clock should advance");
    assert!(manual.as_mut().poll(&mut context).is_ready());

    let timer = qubit_clock::StdTimer::new();
    let mut standard = Box::pin(
        timer
            .after(Duration::from_millis(20))
            .expect("standard timer should register"),
    );
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = standard.as_mut().poll(&mut panic_context);
    }));
    assert!(panicked.is_err());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("test runtime should build");
    runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(10), standard).await })
        .expect("standard timer should complete within the test guard")
        .expect("standard timer should remain usable after clone panic");
}
