/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Shared monotonic timeline for mock time components.

use std::sync::{
    Arc,
    Condvar,
    Mutex,
    MutexGuard,
};
use std::time::{
    Duration,
    Instant,
};

#[cfg(feature = "tokio")]
use crate::sleep::AsyncSleepFuture;
#[cfg(feature = "tokio")]
use tokio::sync::watch;

use crate::{
    MockInstant,
    MockTimeError,
    MockWaiterKind,
};

/// Shared monotonic time source for deterministic tests.
#[derive(Clone, Debug)]
pub struct MockTimeline {
    shared: Arc<MockTimelineShared>,
    #[cfg(feature = "tokio")]
    async_event_sender: watch::Sender<u64>,
}

/// Shared state and condition variables for a mock timeline.
#[derive(Debug)]
struct MockTimelineShared {
    state: Mutex<MockTimelineState>,
    event_changed: Condvar,
    waiters_changed: Condvar,
}

/// Mutable mock timeline state.
#[derive(Debug)]
struct MockTimelineState {
    elapsed_nanos: u128,
    time_epoch: u64,
    event_epoch: u64,
    sleep_waiters: usize,
    deadline_waiters: usize,
}

/// Registration for a mock timeline waiter.
#[derive(Debug)]
struct MockTimelineWaiterRegistration {
    timeline: MockTimeline,
    kind: MockWaiterKind,
}

impl MockTimelineWaiterRegistration {
    /// Registers a waiter on a mock timeline.
    ///
    /// # Parameters
    /// - `timeline`: Timeline that owns the waiter count.
    /// - `kind`: Waiter group to increment.
    ///
    /// # Returns
    /// A registration that decrements the waiter count when dropped.
    fn new(timeline: MockTimeline, kind: MockWaiterKind) -> Self {
        {
            let mut state = timeline.lock_state();
            MockTimeline::increment_waiter(&mut state, kind);
        }
        timeline.shared.waiters_changed.notify_all();
        Self { timeline, kind }
    }
}

impl Drop for MockTimelineWaiterRegistration {
    /// Removes the registered waiter from the timeline.
    fn drop(&mut self) {
        {
            let mut state = self.timeline.lock_state();
            MockTimeline::decrement_waiter(&mut state, self.kind);
        }
        self.timeline.shared.waiters_changed.notify_all();
    }
}

impl MockTimeline {
    /// Creates a new timeline at elapsed zero.
    ///
    /// # Returns
    /// A mock timeline with no elapsed time.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "tokio")]
        let (async_event_sender, _) = watch::channel(0);
        Self {
            shared: Arc::new(MockTimelineShared {
                state: Mutex::new(MockTimelineState {
                    elapsed_nanos: 0,
                    time_epoch: 0,
                    event_epoch: 0,
                    sleep_waiters: 0,
                    deadline_waiters: 0,
                }),
                event_changed: Condvar::new(),
                waiters_changed: Condvar::new(),
            }),
            #[cfg(feature = "tokio")]
            async_event_sender,
        }
    }

    /// Returns elapsed mock time as a standard duration.
    ///
    /// # Returns
    /// Elapsed monotonic time since the timeline origin.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        duration_from_nanos_saturating(self.elapsed_nanos())
    }

    /// Returns elapsed mock time in nanoseconds.
    ///
    /// # Returns
    /// Elapsed monotonic nanoseconds since the timeline origin.
    #[inline]
    pub fn elapsed_nanos(&self) -> u128 {
        self.lock_state().elapsed_nanos
    }

    /// Returns the current mock instant.
    ///
    /// # Returns
    /// Current instant on this timeline.
    #[inline]
    pub fn now(&self) -> MockInstant {
        MockInstant::from_nanos_since_origin(self.elapsed_nanos())
    }

    /// Returns the current event epoch.
    ///
    /// # Returns
    /// Epoch incremented by time advances and external notifications.
    #[inline]
    pub fn event_epoch(&self) -> u64 {
        self.lock_state().event_epoch
    }

    /// Advances mock time and wakes all timeline waiters.
    ///
    /// # Parameters
    /// - `duration`: Non-negative duration to add.
    pub fn advance(&self, duration: Duration) {
        let event_epoch = {
            let mut state = self.lock_state();
            state.elapsed_nanos = state.elapsed_nanos.saturating_add(duration.as_nanos());
            state.time_epoch = state.time_epoch.wrapping_add(1);
            state.event_epoch = state.event_epoch.wrapping_add(1);
            state.event_epoch
        };
        self.notify_waiters(event_epoch);
    }

    /// Resets the timeline to elapsed zero when no waiters are active.
    ///
    /// # Returns
    /// `Ok(())` when reset succeeds.
    ///
    /// # Errors
    /// Returns [`MockTimeError::ActiveWaiters`] when timeline waiters are active.
    pub fn reset(&self) -> Result<(), MockTimeError> {
        let event_epoch = {
            let mut state = self.lock_state();
            if state.sleep_waiters != 0 || state.deadline_waiters != 0 {
                return Err(MockTimeError::ActiveWaiters);
            }
            state.elapsed_nanos = 0;
            state.time_epoch = state.time_epoch.wrapping_add(1);
            state.event_epoch = state.event_epoch.wrapping_add(1);
            state.event_epoch
        };
        self.notify_waiters(event_epoch);
        Ok(())
    }

    /// Wakes waiters without changing elapsed time.
    ///
    /// This is intended for synchronization primitives that combine state-change
    /// notifications with timeout deadlines.
    pub fn notify_external_change(&self) {
        let event_epoch = {
            let mut state = self.lock_state();
            state.event_epoch = state.event_epoch.wrapping_add(1);
            state.event_epoch
        };
        self.notify_waiters(event_epoch);
    }

    /// Blocks until the current mock instant reaches `deadline`.
    ///
    /// # Parameters
    /// - `deadline`: Mock instant at which the wait should complete.
    pub fn wait_until(&self, deadline: MockInstant) {
        self.wait_until_with_kind(deadline, MockWaiterKind::Deadline);
    }

    /// Blocks until `duration` has elapsed on the mock timeline.
    ///
    /// # Parameters
    /// - `duration`: Relative mock duration to wait.
    pub fn wait_for(&self, duration: Duration) {
        self.wait_until(self.now().saturating_add(duration));
    }

    /// Blocks until the event epoch changes after `observed_epoch`.
    ///
    /// # Parameters
    /// - `observed_epoch`: Event epoch already observed by the caller.
    pub fn wait_for_event_after(&self, observed_epoch: u64) {
        let mut state = self.lock_state();
        while state.event_epoch == observed_epoch {
            state = self
                .shared
                .event_changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// Blocks until a registered waiter count is observed or real timeout expires.
    ///
    /// # Parameters
    /// - `kind`: Waiter group to inspect.
    /// - `count`: Minimum number of waiters expected.
    /// - `real_timeout`: Real wall-clock limit used only to keep tests from
    ///   hanging forever.
    ///
    /// # Returns
    /// `true` when enough waiters are observed before the real timeout.
    pub fn wait_for_blocked_waiters(
        &self,
        kind: MockWaiterKind,
        count: usize,
        real_timeout: Duration,
    ) -> bool {
        let Some(deadline) = Instant::now().checked_add(real_timeout) else {
            return false;
        };
        let mut state = self.lock_state();
        while Self::waiter_count(&state, kind) < count {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let wait_result = self.shared.waiters_changed.wait_timeout(state, remaining);
            let (next_state, timeout_result) = match wait_result {
                Ok(result) => result,
                Err(poisoned) => poisoned.into_inner(),
            };
            state = next_state;
            if timeout_result.timed_out() && Self::waiter_count(&state, kind) < count {
                return false;
            }
        }
        true
    }

    /// Blocks until a deadline with the specified waiter kind is reached.
    ///
    /// # Parameters
    /// - `deadline`: Mock instant at which the wait should complete.
    /// - `kind`: Waiter group used for test observability.
    pub(crate) fn wait_until_with_kind(&self, deadline: MockInstant, kind: MockWaiterKind) {
        let mut state = self.lock_state();
        if state.elapsed_nanos >= deadline.nanos_since_origin() {
            return;
        }
        Self::increment_waiter(&mut state, kind);
        self.shared.waiters_changed.notify_all();
        while state.elapsed_nanos < deadline.nanos_since_origin() {
            state = self
                .shared
                .event_changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        Self::decrement_waiter(&mut state, kind);
        self.shared.waiters_changed.notify_all();
    }

    /// Returns a future that completes once the deadline is reached.
    ///
    /// # Parameters
    /// - `deadline`: Mock instant at which the future should resolve.
    /// - `kind`: Waiter group used for test observability.
    ///
    /// # Returns
    /// A future resolving after the mock deadline is reached.
    #[cfg(feature = "tokio")]
    pub(crate) fn wait_until_async_with_kind<'a>(
        &'a self,
        deadline: MockInstant,
        kind: MockWaiterKind,
    ) -> AsyncSleepFuture<'a> {
        if self.elapsed_nanos() >= deadline.nanos_since_origin() {
            return Box::pin(async {});
        }
        let registration = MockTimelineWaiterRegistration::new(self.clone(), kind);
        let mut event_receiver = self.async_event_sender.subscribe();
        Box::pin(async move {
            let _registration = registration;
            loop {
                if self.elapsed_nanos() >= deadline.nanos_since_origin() {
                    return;
                }
                event_receiver
                    .changed()
                    .await
                    .expect("mock timeline sender should live while timeline is borrowed");
            }
        })
    }

    /// Locks timeline state and recovers from poisoning.
    ///
    /// # Returns
    /// A guard for timeline state.
    fn lock_state(&self) -> MutexGuard<'_, MockTimelineState> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Wakes blocking and async waiters after an event-epoch change.
    ///
    /// # Parameters
    /// - `event_epoch`: New event epoch to publish to async waiters.
    fn notify_waiters(&self, event_epoch: u64) {
        self.shared.event_changed.notify_all();
        self.shared.waiters_changed.notify_all();
        self.notify_async_waiters(event_epoch);
    }

    /// Publishes an event change to async waiters.
    ///
    /// # Parameters
    /// - `event_epoch`: New event epoch.
    #[cfg(feature = "tokio")]
    fn notify_async_waiters(&self, event_epoch: u64) {
        let _ = self.async_event_sender.send(event_epoch);
    }

    /// No-op when async support is disabled.
    ///
    /// # Parameters
    /// - `_event_epoch`: New event epoch.
    #[cfg(not(feature = "tokio"))]
    fn notify_async_waiters(&self, _event_epoch: u64) {}

    /// Increments a waiter count.
    ///
    /// # Parameters
    /// - `state`: Timeline state to mutate.
    /// - `kind`: Waiter group to increment.
    fn increment_waiter(state: &mut MockTimelineState, kind: MockWaiterKind) {
        match kind {
            MockWaiterKind::Sleep => {
                state.sleep_waiters = state.sleep_waiters.saturating_add(1);
            }
            MockWaiterKind::Deadline => {
                state.deadline_waiters = state.deadline_waiters.saturating_add(1);
            }
        }
    }

    /// Decrements a waiter count.
    ///
    /// # Parameters
    /// - `state`: Timeline state to mutate.
    /// - `kind`: Waiter group to decrement.
    fn decrement_waiter(state: &mut MockTimelineState, kind: MockWaiterKind) {
        match kind {
            MockWaiterKind::Sleep => {
                state.sleep_waiters = state.sleep_waiters.saturating_sub(1);
            }
            MockWaiterKind::Deadline => {
                state.deadline_waiters = state.deadline_waiters.saturating_sub(1);
            }
        }
    }

    /// Returns the waiter count for a group.
    ///
    /// # Parameters
    /// - `state`: Timeline state to inspect.
    /// - `kind`: Waiter group to read.
    ///
    /// # Returns
    /// Number of registered waiters in the group.
    fn waiter_count(state: &MockTimelineState, kind: MockWaiterKind) -> usize {
        match kind {
            MockWaiterKind::Sleep => state.sleep_waiters,
            MockWaiterKind::Deadline => state.deadline_waiters,
        }
    }
}

impl Default for MockTimeline {
    /// Creates a zero-elapsed mock timeline.
    fn default() -> Self {
        Self::new()
    }
}

/// Converts nanoseconds to [`Duration`], saturating at `Duration::MAX`.
///
/// # Parameters
/// - `nanos`: Nanoseconds to convert.
///
/// # Returns
/// A standard duration.
fn duration_from_nanos_saturating(nanos: u128) -> Duration {
    let secs = nanos / 1_000_000_000;
    let sub_nanos = (nanos % 1_000_000_000) as u32;
    let secs = match u64::try_from(secs) {
        Ok(secs) => secs,
        Err(_) => return Duration::MAX,
    };
    Duration::new(secs, sub_nanos)
}
