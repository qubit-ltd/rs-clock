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

use parking_lot::{
    Condvar,
    Mutex,
    MutexGuard,
};
use std::sync::Arc;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
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

/// Next globally unique mock timeline id.
static NEXT_MOCK_TIMELINE_ID: AtomicU64 = AtomicU64::new(1);

/// Shared monotonic time source for deterministic tests.
///
/// `MockTimeline` is the single authority for elapsed mock time. Cloned
/// timelines share the same internal state, so a clock, sleeper, and future
/// timeout-aware primitive can all observe the same logical time progression.
/// The timeline starts at elapsed zero and advances only when a test calls
/// [`advance`](Self::advance). It never follows wall-clock time by itself.
///
/// The timeline maintains two related notions of progress:
///
/// - **elapsed time** is the monotonic duration since the timeline origin.
///   [`MockClock`](crate::MockClock) and
///   [`crate::sleep::MockSleeper`] derive their behavior from this value.
/// - **event epoch** is a notification counter incremented when time advances
///   or when [`notify_external_change`](Self::notify_external_change) is
///   called. It lets monitor-like primitives wait for either a state change or
///   a later deadline without inventing a second mock clock.
///
/// Blocking waiters use condition variables, and async waiters use a Tokio
/// watch channel when the `tokio` feature is enabled. Waiter counts are tracked
/// by [`MockWaiterKind`] so tests can wait until a thread or future has really
/// entered a mock wait before advancing time. Reset is rejected while waiters
/// are active, because rewinding a timeline under a blocked waiter would make
/// deadline semantics ambiguous. Deadlines also carry the id of the timeline
/// that created them. Passing a deadline from one timeline into another
/// timeline is rejected with [`MockTimeError::MismatchedTimeline`].
///
/// `MockTimeline` uses non-poisoning synchronization primitives internally.
/// A panic in one test thread does not permanently poison the timeline for the
/// rest of the test process.
#[derive(Clone, Debug)]
pub struct MockTimeline {
    id: u64,
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
            id: next_mock_timeline_id(),
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

    /// Returns the globally unique id of this timeline.
    ///
    /// Clones of the same timeline return the same id. Independently created
    /// timelines receive different ids, which lets deadline APIs reject
    /// [`MockInstant`] values from the wrong timeline.
    ///
    /// # Returns
    /// The timeline id.
    #[inline]
    pub const fn id(&self) -> u64 {
        self.id
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
        MockInstant::from_nanos_since_origin(self.id, self.elapsed_nanos())
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
    ///
    /// # Returns
    /// `Ok(())` when the wait completes.
    ///
    /// # Errors
    /// Returns [`MockTimeError::MismatchedTimeline`] if `deadline` was created
    /// by a different timeline.
    #[inline]
    pub fn wait_until(&self, deadline: MockInstant) -> Result<(), MockTimeError> {
        self.wait_until_with_kind(deadline, MockWaiterKind::Deadline)
    }

    /// Blocks until `duration` has elapsed on the mock timeline.
    ///
    /// # Parameters
    /// - `duration`: Relative mock duration to wait.
    #[inline]
    pub fn wait_for(&self, duration: Duration) {
        self.wait_until(self.now().saturating_add(duration))
            .expect("relative waits should create deadlines on the same timeline");
    }

    /// Blocks until the event epoch changes after `observed_epoch`.
    ///
    /// # Parameters
    /// - `observed_epoch`: Event epoch already observed by the caller.
    pub fn wait_for_event_after(&self, observed_epoch: u64) {
        let mut state = self.lock_state();
        while state.event_epoch == observed_epoch {
            self.shared.event_changed.wait(&mut state);
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
    pub fn wait_for_blocked_waiters(&self, kind: MockWaiterKind, count: usize, real_timeout: Duration) -> bool {
        let Some(deadline) = Instant::now().checked_add(real_timeout) else {
            return false;
        };
        let mut state = self.lock_state();
        while Self::waiter_count(&state, kind) < count {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let wait_result = self.shared.waiters_changed.wait_for(&mut state, remaining);
            if wait_result.timed_out() && Self::waiter_count(&state, kind) < count {
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
    pub(crate) fn wait_until_with_kind(
        &self,
        deadline: MockInstant,
        kind: MockWaiterKind,
    ) -> Result<(), MockTimeError> {
        self.ensure_own_instant(deadline)?;
        let mut state = self.lock_state();
        if state.elapsed_nanos >= deadline.nanos_since_origin() {
            return Ok(());
        }
        Self::increment_waiter(&mut state, kind);
        self.shared.waiters_changed.notify_all();
        while state.elapsed_nanos < deadline.nanos_since_origin() {
            self.shared.event_changed.wait(&mut state);
        }
        Self::decrement_waiter(&mut state, kind);
        self.shared.waiters_changed.notify_all();
        Ok(())
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
    ) -> Result<AsyncSleepFuture<'a>, MockTimeError> {
        self.ensure_own_instant(deadline)?;
        if self.elapsed_nanos() >= deadline.nanos_since_origin() {
            return Ok(Box::pin(async {}));
        }
        let registration = MockTimelineWaiterRegistration::new(self.clone(), kind);
        let mut event_receiver = self.async_event_sender.subscribe();
        Ok(Box::pin(async move {
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
        }))
    }

    /// Ensures an instant belongs to this timeline.
    ///
    /// # Parameters
    /// - `instant`: Instant to validate.
    ///
    /// # Returns
    /// `Ok(())` when the instant belongs to this timeline.
    ///
    /// # Errors
    /// Returns [`MockTimeError::MismatchedTimeline`] when the instant belongs
    /// to a different timeline.
    fn ensure_own_instant(&self, instant: MockInstant) -> Result<(), MockTimeError> {
        if instant.timeline_id() == self.id {
            Ok(())
        } else {
            Err(MockTimeError::MismatchedTimeline {
                expected: self.id,
                actual: instant.timeline_id(),
            })
        }
    }

    /// Locks timeline state.
    ///
    /// # Returns
    /// A guard for timeline state.
    #[inline]
    fn lock_state(&self) -> MutexGuard<'_, MockTimelineState> {
        self.shared.state.lock()
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
    #[inline]
    fn notify_async_waiters(&self, event_epoch: u64) {
        let _ = self.async_event_sender.send(event_epoch);
    }

    /// No-op when async support is disabled.
    ///
    /// # Parameters
    /// - `_event_epoch`: New event epoch.
    #[cfg(not(feature = "tokio"))]
    #[inline]
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
    #[inline]
    fn waiter_count(state: &MockTimelineState, kind: MockWaiterKind) -> usize {
        match kind {
            MockWaiterKind::Sleep => state.sleep_waiters,
            MockWaiterKind::Deadline => state.deadline_waiters,
        }
    }
}

impl Default for MockTimeline {
    /// Creates a zero-elapsed mock timeline.
    #[inline]
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

/// Allocates a new globally unique mock timeline id.
///
/// # Returns
/// A non-zero timeline id.
///
/// # Panics
/// Panics if all `u64` timeline ids have been exhausted.
fn next_mock_timeline_id() -> u64 {
    loop {
        let current = NEXT_MOCK_TIMELINE_ID.load(Ordering::Relaxed);
        assert_ne!(current, u64::MAX, "mock timeline id space exhausted");
        if NEXT_MOCK_TIMELINE_ID
            .compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return current;
        }
    }
}
