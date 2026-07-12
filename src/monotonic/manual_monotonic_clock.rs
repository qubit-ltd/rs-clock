// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines an explicitly advanced monotonic clock for deterministic tests.

use crate::monotonic::manual_monotonic_state::{
    AdvanceCallback,
    ManualMonotonicState,
};
use crate::{
    ManualAdvanceSubscription,
    ManualWaiterFuture,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    allocate_clock_domain_id,
};
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
    resume_unwind,
};
use std::sync::{
    Arc,
    Condvar,
    Mutex,
    MutexGuard,
};
use std::task::{
    Context,
    Poll,
    Waker,
};
use std::time::Duration;
use std::time::Instant;

type TimeChangeNotifications = (Vec<Waker>, Vec<AdvanceCallback>);

/// A monotonic clock that advances only when explicitly instructed.
///
/// The type intentionally does not implement [`Clone`]. Components that must
/// observe one shared manual clock use `Arc<ManualMonotonicClock>` explicitly.
pub struct ManualMonotonicClock {
    domain_id: u64,
    state: Mutex<ManualMonotonicState>,
    changed: Condvar,
    waiters_changed: Condvar,
}

impl ManualMonotonicClock {
    /// Creates a new manual clock at its zero-duration origin.
    #[must_use]
    pub fn new() -> Self {
        Self {
            domain_id: allocate_clock_domain_id(),
            state: Mutex::new(ManualMonotonicState::new()),
            changed: Condvar::new(),
            waiters_changed: Condvar::new(),
        }
    }

    /// Advances this clock by `duration` and notifies time observers.
    ///
    /// Returns [`TimeError::InstantOverflow`] when the resulting elapsed time
    /// cannot be represented. A zero duration succeeds as a no-op and does not
    /// notify observers.
    pub fn advance(&self, duration: Duration) -> Result<(), TimeError> {
        if duration.is_zero() {
            return Ok(());
        }
        let notifications = {
            let mut state = self.lock_state();
            state.elapsed = state
                .elapsed
                .checked_add(duration)
                .ok_or(TimeError::InstantOverflow)?;
            Self::collect_time_change_notifications(&state)
        };
        self.notify_time_changed(notifications);
        Ok(())
    }

    /// Advances this clock to an absolute instant in the same domain.
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign instant and
    /// [`TimeError::CannotMoveBackward`] when `target` precedes the current
    /// instant.
    pub fn advance_to(
        &self,
        target: MonotonicInstant,
    ) -> Result<(), TimeError> {
        target.ensure_domain(self.domain_id)?;
        let target_elapsed = target.elapsed_since_origin();
        let notifications = {
            let mut state = self.lock_state();
            if target_elapsed < state.elapsed {
                return Err(TimeError::CannotMoveBackward);
            }
            if target_elapsed == state.elapsed {
                return Ok(());
            }
            state.elapsed = target_elapsed;
            Self::collect_time_change_notifications(&state)
        };
        self.notify_time_changed(notifications);
        Ok(())
    }

    /// Subscribes to successful forward changes of this manual clock.
    ///
    /// This concrete-only hook lets test doubles such as a mock lock monitor
    /// signal their own condition variable or task wakers whenever logical
    /// time advances. The callback executes synchronously outside the clock
    /// mutex and may overlap callbacks from concurrent advances. It must be
    /// idempotent and should do no more than signal the subscriber's own
    /// waiting primitive. Callback order is unspecified. If callbacks panic,
    /// every callback collected for that advance is still attempted before
    /// the first panic is resumed on the advancing thread.
    ///
    /// Dropping the returned handle prevents registration in later advances;
    /// a callback already collected by an in-flight advance may still run once.
    /// A callback that locks another synchronization object must establish a
    /// consistent lock order: callers must not advance this clock while they
    /// hold that same lock.
    ///
    /// The `self: &Arc<Self>` receiver makes shared-clock identity explicit and
    /// lets the returned subscription keep only a weak reference to the clock.
    pub fn subscribe_advances<F>(
        self: &Arc<Self>,
        callback: F,
    ) -> ManualAdvanceSubscription
    where
        F: Fn() + Send + Sync + 'static,
    {
        let subscriber_id = {
            let mut state = self.lock_state();
            let subscriber_id = state.next_advance_subscriber_id;
            state.next_advance_subscriber_id = state
                .next_advance_subscriber_id
                .checked_add(1)
                .expect("manual advance subscriber identifiers exhausted");
            state
                .advance_subscribers
                .insert(subscriber_id, Arc::new(callback));
            subscriber_id
        };
        ManualAdvanceSubscription::new(Arc::downgrade(self), subscriber_id)
    }

    /// Unregisters an advance subscriber when its registration is dropped.
    pub(crate) fn unregister_advance_subscriber(&self, subscriber_id: u64) {
        self.lock_state().advance_subscribers.remove(&subscriber_id);
    }

    /// Returns the number of blocking and asynchronous deadline waiters.
    #[must_use]
    pub fn pending_waiters(&self) -> usize {
        self.lock_state().waiter_count()
    }

    /// Returns the earliest registered deadline that has not yet been reached.
    #[must_use]
    pub fn next_deadline(&self) -> Option<MonotonicInstant> {
        let state = self.lock_state();
        state
            .blocking_waiters
            .values()
            .chain(state.async_waiters.values().map(|(deadline, _)| deadline))
            .filter(|deadline| **deadline > state.elapsed)
            .min()
            .copied()
            .map(|elapsed| MonotonicInstant::new(self.domain_id, elapsed))
    }

    /// Advances to the earliest registered future deadline.
    ///
    /// Returns `Some` with the reached instant, or `None` when no future
    /// deadline is registered. Due registrations awaiting cleanup are ignored.
    pub fn advance_to_next_deadline(&self) -> Option<MonotonicInstant> {
        let (target, notifications) = {
            let mut state = self.lock_state();
            let target_elapsed = state
                .blocking_waiters
                .values()
                .chain(
                    state.async_waiters.values().map(|(deadline, _)| deadline),
                )
                .filter(|deadline| **deadline > state.elapsed)
                .min()
                .copied()?;
            state.elapsed = target_elapsed;
            let target = MonotonicInstant::new(self.domain_id, target_elapsed);
            (target, Self::collect_time_change_notifications(&state))
        };
        self.notify_time_changed(notifications);
        Some(target)
    }

    /// Returns a future that completes after enough waiters are registered.
    ///
    /// Blocking and asynchronous deadline waiters both contribute to the
    /// count. Reaching the count is latched even if waiters unregister before
    /// the returned future is polled again. The `self: &Arc<Self>` receiver
    /// ensures the future keeps this exact clock instance alive.
    #[must_use]
    pub fn wait_for_waiters_async(
        self: &Arc<Self>,
        expected_count: usize,
    ) -> ManualWaiterFuture {
        ManualWaiterFuture::new(Arc::clone(self), expected_count)
    }

    /// Blocks in real time until enough deadline waiters are registered.
    ///
    /// Blocking and asynchronous sleeper registrations both contribute to
    /// `expected_count`. `real_timeout` is only a test guard and never advances
    /// logical time. Returns `true` when the count is reached and `false` when
    /// the real-time guard expires first or cannot be represented.
    #[must_use]
    pub fn wait_for_waiters(
        &self,
        expected_count: usize,
        real_timeout: Duration,
    ) -> bool {
        let Some(real_deadline) = Instant::now().checked_add(real_timeout)
        else {
            return false;
        };
        let mut state = self.lock_state();
        while state.waiter_count() < expected_count {
            let remaining =
                real_deadline.saturating_duration_since(Instant::now());
            let (next_state, wait_result) = self
                .waiters_changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next_state;
            if wait_result.timed_out() && state.waiter_count() < expected_count
            {
                return false;
            }
        }
        true
    }

    /// Blocks until manual time reaches `deadline`.
    ///
    /// The deadline must already have been validated against this clock.
    pub(crate) fn wait_until_blocking(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<(), TimeError> {
        deadline.ensure_domain(self.domain_id)?;
        let deadline_elapsed = deadline.elapsed_since_origin();
        let mut state = self.lock_state();
        if state.elapsed >= deadline_elapsed {
            return Ok(());
        }
        let waiter_id = state.next_blocking_waiter_id;
        state.next_blocking_waiter_id = state
            .next_blocking_waiter_id
            .checked_add(1)
            .expect("manual blocking waiter identifiers exhausted");
        state.blocking_waiters.insert(waiter_id, deadline_elapsed);
        let observer_wakers =
            Self::collect_reached_waiter_observer_wakers(&mut state);
        drop(state);
        self.waiters_changed.notify_all();
        for waker in observer_wakers {
            waker.wake();
        }
        let mut state = self.lock_state();
        while state.elapsed < deadline_elapsed {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        state.blocking_waiters.remove(&waiter_id);
        drop(state);
        self.waiters_changed.notify_all();
        Ok(())
    }

    /// Registers an async deadline at future creation time.
    ///
    /// Returns `Ok(None)` when the deadline has already been reached and a
    /// registration ID otherwise. A foreign deadline returns a domain error.
    pub(crate) fn register_async_waiter(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<Option<u64>, TimeError> {
        deadline.ensure_domain(self.domain_id)?;
        let deadline_elapsed = deadline.elapsed_since_origin();
        let mut state = self.lock_state();
        if state.elapsed >= deadline_elapsed {
            return Ok(None);
        }
        let waiter_id = state.next_async_waiter_id;
        state.next_async_waiter_id = state
            .next_async_waiter_id
            .checked_add(1)
            .expect("manual async waiter identifiers exhausted");
        state
            .async_waiters
            .insert(waiter_id, (deadline_elapsed, None));
        let observer_wakers =
            Self::collect_reached_waiter_observer_wakers(&mut state);
        drop(state);
        self.waiters_changed.notify_all();
        for waker in observer_wakers {
            waker.wake();
        }
        Ok(Some(waiter_id))
    }

    /// Registers an asynchronous observer of the total waiter count.
    pub(crate) fn register_waiter_observer(
        &self,
        expected_count: usize,
    ) -> Option<u64> {
        let mut state = self.lock_state();
        if state.waiter_count() >= expected_count {
            return None;
        }
        let observer_id = state.next_waiter_observer_id;
        state.next_waiter_observer_id = state
            .next_waiter_observer_id
            .checked_add(1)
            .expect("manual waiter observer identifiers exhausted");
        state
            .waiter_observers
            .insert(observer_id, (expected_count, None));
        Some(observer_id)
    }

    /// Polls an asynchronous observer of the total waiter count.
    pub(crate) fn poll_waiter_observer(
        &self,
        observer_id: u64,
        context: &Context<'_>,
    ) -> Poll<()> {
        let mut state = self.lock_state();
        let Some((expected_count, _)) =
            state.waiter_observers.get(&observer_id)
        else {
            return Poll::Ready(());
        };
        if state.waiter_count() >= *expected_count {
            state.waiter_observers.remove(&observer_id);
            return Poll::Ready(());
        }
        if let Some((_, registered_waker)) =
            state.waiter_observers.get_mut(&observer_id)
            && registered_waker
                .as_ref()
                .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            *registered_waker = Some(context.waker().clone());
        }
        Poll::Pending
    }

    /// Removes an incomplete asynchronous waiter-count observer.
    pub(crate) fn unregister_waiter_observer(&self, observer_id: u64) {
        self.lock_state().waiter_observers.remove(&observer_id);
    }

    /// Polls a registered async waiter against current manual time.
    pub(crate) fn poll_async_waiter(
        &self,
        waiter_id: u64,
        deadline: MonotonicInstant,
        context: &Context<'_>,
    ) -> Poll<Result<(), TimeError>> {
        let mut state = self.lock_state();
        if state.elapsed >= deadline.elapsed_since_origin() {
            state.async_waiters.remove(&waiter_id);
            drop(state);
            self.waiters_changed.notify_all();
            return Poll::Ready(Ok(()));
        }
        if let Some((_, registered_waker)) =
            state.async_waiters.get_mut(&waiter_id)
            && registered_waker
                .as_ref()
                .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            *registered_waker = Some(context.waker().clone());
        }
        Poll::Pending
    }

    /// Removes an async waiter after completion or future cancellation.
    pub(crate) fn unregister_async_waiter(&self, waiter_id: u64) {
        let removed =
            self.lock_state().async_waiters.remove(&waiter_id).is_some();
        if removed {
            self.waiters_changed.notify_all();
        }
    }

    /// Locks mutable state, recovering the inner value after poisoning.
    fn lock_state(&self) -> MutexGuard<'_, ManualMonotonicState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Collects due task wakers and current advance subscribers under the lock.
    fn collect_time_change_notifications(
        state: &ManualMonotonicState,
    ) -> TimeChangeNotifications {
        let due_wakers = state
            .async_waiters
            .values()
            .filter(|(deadline, _)| *deadline <= state.elapsed)
            .filter_map(|(_, waker)| waker.clone())
            .collect();
        let subscribers = state.advance_subscribers.values().cloned().collect();
        (due_wakers, subscribers)
    }

    /// Removes reached observers and collects their registered task wakers.
    ///
    /// Removing an observer latches the reached state: its future treats a
    /// missing registration as complete even if waiters unregister before the
    /// future is polled again.
    fn collect_reached_waiter_observer_wakers(
        state: &mut ManualMonotonicState,
    ) -> Vec<Waker> {
        let waiter_count = state.waiter_count();
        let mut wakers = Vec::new();
        state.waiter_observers.retain(|_, (expected_count, waker)| {
            if *expected_count <= waiter_count {
                if let Some(waker) = waker.take() {
                    wakers.push(waker);
                }
                false
            } else {
                true
            }
        });
        wakers
    }

    /// Wakes time observers and invokes every collected subscriber outside the
    /// state lock, resuming the first subscriber panic after fanout completes.
    fn notify_time_changed(
        &self,
        (due_wakers, subscribers): TimeChangeNotifications,
    ) {
        self.changed.notify_all();
        for waker in due_wakers {
            waker.wake();
        }
        let mut first_panic = None;
        for subscriber in subscribers {
            if let Err(payload) =
                catch_unwind(AssertUnwindSafe(|| subscriber()))
                && first_panic.is_none()
            {
                first_panic = Some(payload);
            }
        }
        if let Some(payload) = first_panic {
            resume_unwind(payload);
        }
    }
}

impl std::fmt::Debug for ManualMonotonicClock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualMonotonicClock")
            .field("domain_id", &self.domain_id)
            .finish_non_exhaustive()
    }
}

impl Default for ManualMonotonicClock {
    /// Creates a new independent manual clock domain.
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for ManualMonotonicClock {
    /// Returns this clock's stable domain identifier.
    fn domain_id(&self) -> u64 {
        self.domain_id
    }

    /// Returns current logical elapsed time without advancing it.
    fn elapsed_since_origin(&self) -> Duration {
        self.lock_state().elapsed
    }
}
