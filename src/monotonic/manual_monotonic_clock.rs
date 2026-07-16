// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines an explicitly advanced monotonic clock for deterministic tests.

use crate::monotonic::internal::{
    AdvanceEffects,
    ManualMonotonicState,
    PanicFanout,
    WaiterRegistrationGuard,
};
use crate::{
    ClockDomain,
    ManualAdvanceSubscription,
    ManualWaiterFuture,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
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
};
use std::time::Duration;
use std::time::Instant;

/// A monotonic clock that advances only when explicitly instructed.
///
/// The type intentionally does not implement [`Clone`]. Components that must
/// observe one shared manual clock use `Arc<ManualMonotonicClock>` explicitly.
pub struct ManualMonotonicClock {
    /// The identifier of the originating monotonic clock domain.
    domain: ClockDomain,
    /// The mutable state of the manual clock.
    state: Mutex<ManualMonotonicState>,
    /// The condition variable used to notify time changes.
    changed: Condvar,
    /// The condition variable used to notify waiter changes.
    waiters_changed: Condvar,
}

impl ManualMonotonicClock {
    /// Creates a new manual clock at its zero-duration origin.
    ///
    /// # Returns
    ///
    /// A manual monotonic clock with a newly allocated domain.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            domain: ClockDomain::new(),
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
    ///
    /// # Parameters
    ///
    /// * `duration` - Logical duration to add to the current clock reading.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the advance or zero-duration no-op completes.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when the resulting elapsed time
    /// cannot be represented.
    ///
    /// # Panics
    ///
    /// Panics if waking a registered task or invoking an advance subscriber
    /// panics. Every waker and subscriber collected for this advance is
    /// attempted before the first panic is resumed. The logical-time update is
    /// already committed and is not rolled back during unwinding.
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
            Self::collect_advance_effects(&mut state)
        };
        self.notify_time_changed(notifications);
        Ok(())
    }

    /// Advances this clock to an absolute instant in the same domain.
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign instant and
    /// [`TimeError::CannotMoveBackward`] when `target` precedes the current
    /// instant.
    ///
    /// # Parameters
    ///
    /// * `target` - Same-domain instant that becomes the new clock reading.
    ///
    /// # Returns
    ///
    /// `Ok(())` after reaching `target`, including a no-op at the current time.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign target and
    /// [`TimeError::CannotMoveBackward`] for an earlier target.
    ///
    /// # Panics
    ///
    /// Panics if waking a registered task or invoking an advance subscriber
    /// panics. Every waker and subscriber collected for this advance is
    /// attempted before the first panic is resumed. The logical-time update is
    /// already committed and is not rolled back during unwinding.
    pub fn advance_to(
        &self,
        target: MonotonicInstant,
    ) -> Result<(), TimeError> {
        target.ensure_domain(self.domain)?;
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
            Self::collect_advance_effects(&mut state)
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
    ///
    /// # Parameters
    ///
    /// * `callback` - Thread-safe notification callback invoked after advances.
    ///
    /// # Returns
    ///
    /// A handle that unregisters the callback when dropped.
    ///
    /// # Panics
    ///
    /// Panics if the advance-subscriber identifier space is exhausted.
    pub fn subscribe_advances<F>(
        self: &Arc<Self>,
        callback: F,
    ) -> ManualAdvanceSubscription
    where
        F: Fn() + Send + Sync + 'static,
    {
        let subscriber_id = {
            let mut state = self.lock_state();
            state.advances.register(Arc::new(callback))
        };
        ManualAdvanceSubscription::new(Arc::downgrade(self), subscriber_id)
    }

    /// Returns the number of registered blocking and async deadline waiters.
    ///
    /// A reached async waiter remains registered and continues contributing to
    /// this count until its future is polled again or dropped. Use
    /// [`next_deadline()`](Self::next_deadline) when only future deadlines are
    /// relevant.
    ///
    /// # Returns
    ///
    /// The total number of waiter registrations awaiting cleanup or completion.
    #[must_use]
    #[inline(always)]
    pub fn pending_waiters(&self) -> usize {
        self.lock_state().waiter_count()
    }

    /// Returns the earliest registered deadline that has not yet been reached.
    ///
    /// # Returns
    ///
    /// The earliest future deadline, or `None` when every registration is due
    /// or no waiter is registered.
    #[must_use]
    #[inline]
    pub fn next_deadline(&self) -> Option<MonotonicInstant> {
        let state = self.lock_state();
        self.next_future_deadline(&state)
    }

    /// Blocks in real time until a future deadline is registered.
    ///
    /// Existing registrations whose deadlines have already been reached are
    /// ignored. This allows a test driver to wait for the next stage of a
    /// repeated operation even while the previous waiter is still cleaning up.
    /// `real_timeout` is only a test guard and never advances logical time.
    /// An existing future deadline is returned before the guard is checked for
    /// representability.
    ///
    /// # Parameters
    ///
    /// * `real_timeout` - Maximum real time spent waiting for a future
    ///   deadline.
    ///
    /// # Returns
    ///
    /// The earliest registered future deadline. Returns `None` when no such
    /// deadline exists and the real-time guard expires or cannot be
    /// represented.
    #[must_use]
    pub fn wait_for_next_deadline(
        &self,
        real_timeout: Duration,
    ) -> Option<MonotonicInstant> {
        let mut state = self.lock_state();
        if let Some(deadline) = self.next_future_deadline(&state) {
            return Some(deadline);
        }
        let real_deadline = Instant::now().checked_add(real_timeout)?;
        loop {
            let remaining =
                real_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (next_state, wait_result) = self
                .waiters_changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next_state;
            if let Some(deadline) = self.next_future_deadline(&state) {
                return Some(deadline);
            }
            if wait_result.timed_out() {
                return None;
            }
        }
    }

    /// Advances to the earliest registered future deadline.
    ///
    /// Returns `Some` with the reached instant, or `None` when no future
    /// deadline is registered. Due registrations awaiting cleanup are ignored.
    ///
    /// # Returns
    ///
    /// The reached same-domain instant, or `None` when no future deadline is
    /// registered.
    ///
    /// # Panics
    ///
    /// Panics if waking a registered task or invoking an advance subscriber
    /// panics. Every waker and subscriber collected for this advance is
    /// attempted before the first panic is resumed. The logical-time update is
    /// already committed and is not rolled back during unwinding.
    pub fn advance_to_next_deadline(&self) -> Option<MonotonicInstant> {
        let (target, notifications) = {
            let mut state = self.lock_state();
            let target_elapsed =
                state.waiters.next_future_deadline(state.elapsed)?;
            state.elapsed = target_elapsed;
            let target = MonotonicInstant::new(self.domain, target_elapsed);
            (target, Self::collect_advance_effects(&mut state))
        };
        self.notify_time_changed(notifications);
        Some(target)
    }

    /// Returns a future that completes after enough waiters are registered.
    ///
    /// Blocking and asynchronous deadline waiters both contribute to the
    /// count. Reaching the count is latched even if waiters unregister before
    /// the returned future is polled again. The `self: &Arc<Self>` receiver
    /// ensures the future keeps this exact clock instance alive. The observer
    /// is registered before this method returns, rather than on the first poll.
    /// Reached async waiters remain counted until their futures are polled or
    /// dropped, so a due registration can satisfy `expected_count`. Use
    /// [`wait_for_next_deadline()`](Self::wait_for_next_deadline) to coordinate
    /// a later stage that specifically requires a future deadline.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Registration count that completes the future.
    ///
    /// # Returns
    ///
    /// A future with its waiter-count observer already registered.
    ///
    /// # Panics
    ///
    /// Panics if the waiter-observer identifier space is exhausted.
    #[must_use]
    #[inline(always)]
    pub fn wait_for_waiters_async(
        self: &Arc<Self>,
        expected_count: usize,
    ) -> ManualWaiterFuture {
        ManualWaiterFuture::new(Arc::clone(self), expected_count)
    }

    /// Blocks in real time until enough deadline waiters are registered.
    ///
    /// Blocking and asynchronous sleeper registrations both contribute to
    /// `expected_count`. Reaching the count is latched even if waiters
    /// unregister before this thread reacquires the clock state. `real_timeout`
    /// is only a test guard and never advances logical time. Returns `true`
    /// when the count is reached, including when it is already satisfied before
    /// an unrepresentable guard is evaluated. Returns `false` when an
    /// unsatisfied wait reaches a guard that expires or cannot be represented.
    /// Reached async waiters remain counted until their futures are polled or
    /// dropped, so a due registration can satisfy `expected_count`.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Registration count that completes the wait.
    /// * `real_timeout` - Maximum real time spent waiting for that count.
    ///
    /// # Returns
    ///
    /// `true` when the count is already satisfied or becomes reached. Returns
    /// `false` when the count remains unsatisfied and the real-time guard
    /// expires or cannot be represented.
    ///
    /// # Panics
    ///
    /// Panics if the waiter-observer identifier space is exhausted.
    #[must_use]
    pub fn wait_for_waiters(
        &self,
        expected_count: usize,
        real_timeout: Duration,
    ) -> bool {
        let mut state = self.lock_state();
        let count = state.waiter_count();
        if count >= expected_count {
            return true;
        }
        let Some(real_deadline) = Instant::now().checked_add(real_timeout)
        else {
            return false;
        };
        let Some(observer_id) =
            state.waiters.register_observer(expected_count, count)
        else {
            return true;
        };
        loop {
            let remaining =
                real_deadline.saturating_duration_since(Instant::now());
            let (next_state, wait_result) = self
                .waiters_changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next_state;
            if !state.waiters.contains_observer(observer_id) {
                return true;
            }
            if wait_result.timed_out() {
                let removed_waker =
                    state.waiters.unregister_observer(observer_id);
                drop(state);
                drop(removed_waker);
                return false;
            }
        }
    }

    /// Unregisters an advance subscriber when its registration is dropped.
    ///
    /// # Parameters
    ///
    /// * `subscriber_id` - Identifier of the callback to unregister.
    ///
    /// # Panics
    ///
    /// Panics after releasing the clock state lock if destroying the callback
    /// or one of its captured values panics.
    #[inline]
    pub(crate) fn unregister_advance_subscriber(&self, subscriber_id: u64) {
        let removed_callback = {
            let mut state = self.lock_state();
            state.advances.unregister(subscriber_id)
        };
        drop(removed_callback);
    }

    /// Blocks until manual time reaches `deadline`.
    ///
    /// The deadline must already have been validated against this clock. If a
    /// reached waiter-count observer waker panics, all reached wakers are
    /// attempted before the first panic is resumed and this registration is
    /// removed during unwinding.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Domain-scoped instant to wait for.
    ///
    /// # Returns
    ///
    /// `Ok(())` after manual time reaches `deadline`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline.
    ///
    /// # Panics
    ///
    /// Panics after attempting every reached observer waker if one panics.
    pub(crate) fn wait_until_blocking(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<(), TimeError> {
        deadline.ensure_domain(self.domain)?;
        let deadline_elapsed = deadline.elapsed_since_origin();
        let mut state = self.lock_state();
        if state.elapsed >= deadline_elapsed {
            return Ok(());
        }
        let waiter_id = state.waiters.register_blocking(deadline_elapsed);
        let registration = WaiterRegistrationGuard::blocking(self, waiter_id);
        let observer_wakers = state.waiters.reached_observer_wakers();
        drop(state);
        self.waiters_changed.notify_all();
        let mut fanout = PanicFanout::new();
        fanout.wake_all(observer_wakers);
        fanout.resume_first_panic();
        let mut state = self.lock_state();
        while state.elapsed < deadline_elapsed {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        drop(state);
        drop(registration);
        Ok(())
    }

    /// Registers an async deadline at future creation time.
    ///
    /// Returns `Ok(None)` when the deadline has already been reached and a
    /// registration ID otherwise. A foreign deadline returns a domain error.
    /// If a reached observer waker panics, all reached wakers are attempted
    /// before the first panic is resumed and the new registration is removed.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Domain-scoped instant to register.
    ///
    /// # Returns
    ///
    /// `Ok(Some(id))` for a new waiter or `Ok(None)` when the deadline has
    /// already been reached.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline.
    ///
    /// # Panics
    ///
    /// Panics when waiter identifiers are exhausted or, after attempting all
    /// reached observer wakers, if one of those wakers panics.
    pub(crate) fn register_async_waiter(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<Option<u64>, TimeError> {
        deadline.ensure_domain(self.domain)?;
        let deadline_elapsed = deadline.elapsed_since_origin();
        let mut state = self.lock_state();
        if state.elapsed >= deadline_elapsed {
            return Ok(None);
        }
        let waiter_id = state.waiters.register_async(deadline_elapsed);
        let registration =
            WaiterRegistrationGuard::asynchronous(self, waiter_id);
        let observer_wakers = state.waiters.reached_observer_wakers();
        drop(state);
        self.waiters_changed.notify_all();
        let mut fanout = PanicFanout::new();
        fanout.wake_all(observer_wakers);
        fanout.resume_first_panic();
        Ok(Some(registration.into_async_waiter_id()))
    }

    /// Registers an asynchronous observer of the total waiter count.
    ///
    /// Panics if the observer identifier space is exhausted.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Registration count that satisfies the observer.
    ///
    /// # Returns
    ///
    /// A new observer identifier, or `None` when the count is already
    /// satisfied.
    ///
    /// # Panics
    ///
    /// Panics when the observer identifier space is exhausted.
    #[inline]
    pub(crate) fn register_waiter_observer(
        &self,
        expected_count: usize,
    ) -> Option<u64> {
        let mut state = self.lock_state();
        let count = state.waiter_count();
        state.waiters.register_observer(expected_count, count)
    }

    /// Polls an asynchronous observer of the total waiter count.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Identifier of the observer to poll.
    /// * `context` - Task context whose waker is retained while pending.
    ///
    /// # Returns
    ///
    /// [`Poll::Ready`] after the count is reached, otherwise [`Poll::Pending`].
    ///
    /// # Panics
    ///
    /// Panics after releasing the clock state lock if destroying a replaced
    /// custom task waker panics.
    #[inline]
    pub(crate) fn poll_waiter_observer(
        &self,
        observer_id: u64,
        context: &Context<'_>,
    ) -> Poll<()> {
        let (poll_result, replaced_waker) = {
            let mut state = self.lock_state();
            let count = state.waiter_count();
            state.waiters.poll_observer(observer_id, count, context)
        };
        drop(replaced_waker);
        poll_result
    }

    /// Removes an incomplete asynchronous waiter-count observer.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Identifier of the observer to remove.
    ///
    /// # Panics
    ///
    /// Panics after releasing the clock state lock if destroying the
    /// observer's custom task waker panics.
    #[inline]
    pub(crate) fn unregister_waiter_observer(&self, observer_id: u64) {
        let removed_waker = {
            let mut state = self.lock_state();
            state.waiters.unregister_observer(observer_id)
        };
        drop(removed_waker);
    }

    /// Polls a registered async waiter against current manual time.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Identifier returned when the waiter was registered.
    /// * `context` - Task context used to update the registered waker.
    ///
    /// # Returns
    ///
    /// [`Poll::Ready`] after the registered deadline is reached, or
    /// [`Poll::Pending`] while manual time remains before that deadline.
    ///
    /// # Errors
    ///
    /// The ready result is currently always `Ok(())`; registration errors are
    /// resolved before a waiter identifier is returned.
    ///
    /// # Panics
    ///
    /// Panics if `waiter_id` no longer identifies a registered async waiter or
    /// if destroying a replaced custom task waker panics after unlocking.
    pub(crate) fn poll_async_waiter(
        &self,
        waiter_id: u64,
        context: &Context<'_>,
    ) -> Poll<Result<(), TimeError>> {
        let (poll_result, replaced_waker) = {
            let mut state = self.lock_state();
            let elapsed = state.elapsed;
            state.waiters.poll_async(waiter_id, elapsed, context)
        };
        drop(replaced_waker);
        if poll_result.is_ready() {
            self.waiters_changed.notify_all();
            return Poll::Ready(Ok(()));
        }
        Poll::Pending
    }

    /// Removes an async waiter after completion or future cancellation.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Identifier of the async waiter to remove.
    ///
    /// # Panics
    ///
    /// Panics after releasing the clock state lock if destroying the waiter's
    /// custom task waker panics.
    #[inline]
    pub(crate) fn unregister_async_waiter(&self, waiter_id: u64) {
        let removed_waiter = {
            let mut state = self.lock_state();
            state.waiters.unregister_async(waiter_id)
        };
        let was_registered = removed_waiter.is_some();
        drop(removed_waiter);
        if was_registered {
            self.waiters_changed.notify_all();
        }
    }

    /// Removes a blocking waiter after completion or unwinding.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Identifier of the blocking waiter to remove.
    #[inline]
    pub(super) fn unregister_blocking_waiter(&self, waiter_id: u64) {
        self.lock_state().waiters.unregister_blocking(waiter_id);
        self.waiters_changed.notify_all();
    }

    /// Returns the earliest future deadline represented in this clock domain.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked manual-clock state to inspect.
    ///
    /// # Returns
    ///
    /// The earliest future deadline in this clock's domain, or `None`.
    #[inline]
    fn next_future_deadline(
        &self,
        state: &ManualMonotonicState,
    ) -> Option<MonotonicInstant> {
        state
            .waiters
            .next_future_deadline(state.elapsed)
            .map(|elapsed| MonotonicInstant::new(self.domain, elapsed))
    }

    /// Locks mutable state, recovering the inner value after poisoning.
    ///
    /// # Returns
    ///
    /// A guard granting mutable access to the manual clock state.
    #[inline]
    fn lock_state(&self) -> MutexGuard<'_, ManualMonotonicState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Collects due task wakers and current advance callbacks under the lock.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked manual-clock state from which effects are collected.
    ///
    /// # Returns
    ///
    /// Owned notification effects that can be processed after unlocking.
    #[inline]
    fn collect_advance_effects(
        state: &mut ManualMonotonicState,
    ) -> AdvanceEffects {
        let elapsed = state.elapsed;
        let due_wakers = state.waiters.take_due_async_wakers(elapsed);
        let advance_callbacks = state.advances.callbacks();
        AdvanceEffects {
            due_wakers,
            advance_callbacks,
        }
    }

    /// Wakes time observers and invokes every collected subscriber outside the
    /// state lock, resuming the first panic after the full fanout completes.
    ///
    /// # Parameters
    ///
    /// * `effects` - Due task wakers and advance callbacks to notify.
    ///
    /// # Panics
    ///
    /// Resumes the first panic raised by a waker, callback, or their
    /// destructors after every collected target has been attempted.
    fn notify_time_changed(&self, effects: AdvanceEffects) {
        let AdvanceEffects {
            due_wakers,
            advance_callbacks,
        } = effects;
        self.changed.notify_all();
        let mut fanout = PanicFanout::new();
        fanout.wake_all(due_wakers);
        fanout.call_all(advance_callbacks);
        fanout.resume_first_panic();
    }
}

impl std::fmt::Debug for ManualMonotonicClock {
    /// Formats this clock's domain without locking its mutable time state.
    ///
    /// # Parameters
    ///
    /// * `formatter` - The destination formatter.
    ///
    /// # Returns
    ///
    /// `Ok(())` when formatting succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`std::fmt::Error`] when the formatter cannot accept the
    /// generated output.
    #[inline]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualMonotonicClock")
            .field("domain", &self.domain)
            .finish_non_exhaustive()
    }
}

impl Default for ManualMonotonicClock {
    /// Creates a new independent manual clock domain.
    ///
    /// # Returns
    ///
    /// A manual monotonic clock at elapsed duration zero.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for ManualMonotonicClock {
    /// Returns the current instant in this clock's domain.
    ///
    /// # Returns
    ///
    /// The current logical elapsed duration represented in this clock's domain.
    #[inline]
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::new(self.domain, self.lock_state().elapsed)
    }
}
