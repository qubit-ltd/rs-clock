// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow coverage-cfg

#[cfg(coverage)]
use qubit_clock::{
    StdMonotonicClock,
    StdTimer,
    Timer,
    panic_next_std_timer_worker,
};
#[cfg(coverage)]
use qubit_clock::{
    TimeError,
    TimerUnavailableError,
};
#[cfg(coverage)]
use std::sync::{
    Arc,
    mpsc::{
        SyncSender,
        sync_channel,
    },
};
#[cfg(coverage)]
use std::task::{
    Context,
    Poll,
    Wake,
    Waker,
};
#[cfg(coverage)]
use std::time::Duration;

/// Maximum time allowed for a failed worker to notify its waiter.
#[cfg(coverage)]
const FAILURE_GUARD: Duration = Duration::from_secs(2);

/// Maximum time allowed for a replacement worker to complete a deadline.
#[cfg(coverage)]
const RECOVERY_GUARD: Duration = Duration::from_secs(2);

/// Unparks the thread currently polling a standard Timer future.
#[cfg(coverage)]
struct ThreadUnparker {
    /// Thread to unpark when its registered Waker is invoked.
    thread: std::thread::Thread,
}

#[cfg(coverage)]
impl Wake for ThreadUnparker {
    /// Unparks the polling thread.
    fn wake(self: Arc<Self>) {
        self.thread.unpark();
    }

    /// Unparks the polling thread without consuming the shared Waker state.
    fn wake_by_ref(self: &Arc<Self>) {
        self.thread.unpark();
    }
}

/// Polls a standard Timer future until it completes.
///
/// # Parameters
///
/// * `future` - Registered standard Timer future to drive.
/// * `pending_sender` - Optional signal emitted after the first pending poll.
///
/// # Returns
///
/// The Timer completion result.
#[cfg(coverage)]
fn poll_until_terminal(
    mut future: qubit_clock::TimerFuture,
    mut pending_sender: Option<SyncSender<()>>,
) -> Result<(), TimeError> {
    let thread_waker = Arc::new(ThreadUnparker {
        thread: std::thread::current(),
    });
    let waker = Waker::from(thread_waker);
    let mut context = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
            return result;
        }
        if let Some(sender) = pending_sender.take() {
            let _ = sender.send(());
        }
        std::thread::park();
    }
}

/// Verifies worker failure reaches existing waiters and permits a replacement.
#[cfg(coverage)]
#[test]
fn test_std_timer_worker_failure_fails_waiter_and_recovers_next_generation() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let failed_future = timer
        .after(Duration::from_secs(30))
        .expect("worker failure registration should be accepted");
    let (pending_sender, pending_receiver) = sync_channel(1);
    let (failure_sender, failure_receiver) = sync_channel(1);
    let failure_waiter = std::thread::spawn(move || {
        let failure = poll_until_terminal(failed_future, Some(pending_sender));
        let _ = failure_sender.send(failure);
    });
    pending_receiver
        .recv_timeout(FAILURE_GUARD)
        .expect("future should register its Waker before worker failure");

    panic_next_std_timer_worker();

    let failure = failure_receiver
        .recv_timeout(FAILURE_GUARD)
        .expect("failed worker must not leave its waiter pending");
    assert!(matches!(
        failure,
        Err(TimeError::TimerUnavailable {
            source: TimerUnavailableError::SchedulerWorkerTerminated,
        })
    ));
    failure_waiter
        .join()
        .expect("failure-observing thread should finish");

    let recovered_future = timer
        .after(Duration::from_millis(10))
        .expect("new worker generation should register");
    let (recovery_sender, recovery_receiver) = sync_channel(1);
    let recovery_waiter = std::thread::spawn(move || {
        let recovery = poll_until_terminal(recovered_future, None);
        let _ = recovery_sender.send(recovery);
    });
    recovery_receiver
        .recv_timeout(RECOVERY_GUARD)
        .expect("new worker generation should make progress")
        .expect("recovered timer should complete successfully");
    recovery_waiter
        .join()
        .expect("recovery-observing thread should finish");
}
