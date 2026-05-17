/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

#[cfg(feature = "tokio")]
use tokio::sync::watch;

use crate::sleep::Sleeper;
#[cfg(feature = "tokio")]
use crate::sleep::{AsyncSleepFuture, AsyncSleeper};

/// A manually controlled elapsed-time sleeper for deterministic tests.
///
/// This type implements [`Sleeper`] by waiting until manually controlled mock
/// elapsed time reaches the requested target. When the `tokio` feature is
/// enabled, it also implements `AsyncSleeper` with the same mock elapsed-time
/// semantics.
///
/// # Testing guidance
///
/// Use this type when the code under test sleeps for retry, backoff, polling,
/// or timeout intervals. `MockSleeper` controls elapsed sleep time only; it
/// does not change the current time returned by [`crate::MockClock`]. If a
/// component depends on both current-time reads and sleep completion, inject a
/// `MockClock` and a `MockSleeper` separately and advance each one explicitly.
#[derive(Clone, Debug)]
pub struct MockSleeper {
    shared: Arc<MockSleeperShared>,
    #[cfg(feature = "tokio")]
    async_time_epoch_sender: watch::Sender<u64>,
}

/// Shared state and condition variable for cloned mock sleepers.
#[derive(Debug)]
struct MockSleeperShared {
    state: Mutex<MockSleeperState>,
    time_changed: Condvar,
}

/// Mutable mock sleeper state guarded by [`MockSleeperShared::state`].
#[derive(Debug)]
struct MockSleeperState {
    elapsed: Duration,
    time_epoch: u64,
}

/// Snapshot of mock sleeper state read under the internal lock.
#[derive(Clone, Copy, Debug)]
struct MockSleeperSnapshot {
    elapsed: Duration,
}

impl MockSleeper {
    /// Creates a mock sleeper whose elapsed time starts at zero.
    ///
    /// # Returns
    ///
    /// A new mock sleeper with zero elapsed time.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "tokio")]
        let (async_time_epoch_sender, _) = watch::channel(0);
        Self {
            shared: Arc::new(MockSleeperShared {
                state: Mutex::new(MockSleeperState {
                    elapsed: Duration::ZERO,
                    time_epoch: 0,
                }),
                time_changed: Condvar::new(),
            }),
            #[cfg(feature = "tokio")]
            async_time_epoch_sender,
        }
    }

    /// Returns the current mock elapsed time.
    ///
    /// # Returns
    ///
    /// The elapsed time observed by this mock sleeper.
    pub fn elapsed(&self) -> Duration {
        self.current_state().elapsed
    }

    /// Sets the current mock elapsed time.
    ///
    /// This wakes all blocking and asynchronous sleepers so they can recheck
    /// their target elapsed time.
    ///
    /// # Arguments
    ///
    /// * `elapsed` - The new elapsed time.
    pub fn set_elapsed(&self, elapsed: Duration) {
        let time_epoch = {
            let mut state = self.lock_state();
            state.elapsed = elapsed;
            state.time_epoch = state.time_epoch.wrapping_add(1);
            state.time_epoch
        };
        self.shared.time_changed.notify_all();
        self.notify_async_time_changed(time_epoch);
    }

    /// Advances the mock elapsed time by a duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The amount to add to the current elapsed time.
    pub fn advance(&self, duration: Duration) {
        let time_epoch = {
            let mut state = self.lock_state();
            state.elapsed = state.elapsed.saturating_add(duration);
            state.time_epoch = state.time_epoch.wrapping_add(1);
            state.time_epoch
        };
        self.shared.time_changed.notify_all();
        self.notify_async_time_changed(time_epoch);
    }

    /// Resets the mock elapsed time to zero.
    pub fn reset(&self) {
        self.set_elapsed(Duration::ZERO);
    }

    /// Reads a state snapshot under the sleeper lock.
    ///
    /// # Returns
    ///
    /// A snapshot of current elapsed time.
    fn current_state(&self) -> MockSleeperSnapshot {
        let state = self.lock_state();
        Self::snapshot(&state)
    }

    /// Creates an immutable snapshot from locked state.
    ///
    /// # Arguments
    ///
    /// * `state` - The locked state to snapshot.
    ///
    /// # Returns
    ///
    /// The corresponding immutable snapshot.
    fn snapshot(state: &MockSleeperState) -> MockSleeperSnapshot {
        MockSleeperSnapshot {
            elapsed: state.elapsed,
        }
    }

    /// Locks the mock sleeper state.
    ///
    /// # Returns
    ///
    /// A guard for the sleeper state.
    fn lock_state(&self) -> MutexGuard<'_, MockSleeperState> {
        self.shared
            .state
            .lock()
            .expect("mock sleeper state should not be poisoned")
    }

    /// Notifies asynchronous sleepers about a time change.
    ///
    /// # Arguments
    ///
    /// * `time_epoch` - The new time epoch value.
    #[cfg(feature = "tokio")]
    fn notify_async_time_changed(&self, time_epoch: u64) {
        let _ = self.async_time_epoch_sender.send(time_epoch);
    }

    /// No-op when asynchronous sleeper support is disabled.
    #[cfg(not(feature = "tokio"))]
    fn notify_async_time_changed(&self, _time_epoch: u64) {}
}

impl Default for MockSleeper {
    /// Creates a new mock sleeper.
    fn default() -> Self {
        Self::new()
    }
}

impl Sleeper for MockSleeper {
    /// Blocks until mock elapsed time has advanced by `duration`.
    fn sleep_for(&self, duration: Duration) {
        let target_elapsed = self.current_state().elapsed.saturating_add(duration);
        let mut state = self.lock_state();
        while state.elapsed < target_elapsed {
            state = self
                .shared
                .time_changed
                .wait(state)
                .expect("mock sleeper state should not be poisoned");
        }
    }
}

#[cfg(feature = "tokio")]
impl AsyncSleeper for MockSleeper {
    /// Returns a future that completes when mock elapsed time reaches the target.
    fn sleep_for_async<'a>(&'a self, duration: Duration) -> AsyncSleepFuture<'a> {
        let snapshot = self.current_state();
        let target_elapsed = snapshot.elapsed.saturating_add(duration);
        let mut time_receiver = self.async_time_epoch_sender.subscribe();
        Box::pin(async move {
            if self.current_state().elapsed >= target_elapsed {
                return;
            }

            loop {
                if self.current_state().elapsed >= target_elapsed {
                    return;
                }
                time_receiver
                    .changed()
                    .await
                    .expect("mock sleeper sender should live while the sleeper is borrowed");
            }
        })
    }
}
