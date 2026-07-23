// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines an explicitly advanced monotonic clock for deterministic tests.

use crate::internal::PanicFanout;
use crate::monotonic::internal::{
    AdvanceEffects,
    ManualTimeDomain,
    WaiterRegistrationGuard,
};
use crate::{
    ClockDomain,
    ManualDeadlineFuture,
    ManualTimer,
    ManualWaiterFuture,
    ManualWallClock,
    MonotonicClock,
    MonotonicInstant,
    TimeError,
    Timer,
};
use std::sync::Arc;
use std::task::{
    Context,
    Poll,
};
use std::time::{
    Duration,
    SystemTime,
};

/// A monotonic clock that advances only when explicitly instructed.
///
/// The type intentionally does not implement [`Clone`]. Components that must
/// observe one shared manual clock use `Arc<ManualMonotonicClock>` explicitly.
pub struct ManualMonotonicClock {
    /// The identifier of the originating monotonic clock domain.
    domain: ClockDomain,
    /// Shared mutable state of this manual time domain.
    time_domain: Arc<ManualTimeDomain>,
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
            time_domain: Arc::new(ManualTimeDomain::new()),
        }
    }

    /// Creates a shared manual clock at its zero-duration origin.
    ///
    /// This is the preferred constructor when timers, wall clocks, or test
    /// drivers must share its timeline.
    ///
    /// # Returns
    ///
    /// A reference-counted manual clock with a newly allocated domain.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Creates a private handle retaining this exact manual time domain.
    ///
    /// This operation deliberately does not expose [`Clone`] publicly. It is
    /// used by concrete time-domain components that must outlive the clock
    /// value supplied to their constructors.
    ///
    /// # Returns
    ///
    /// A clock handle with the same domain identifier and shared state.
    #[must_use]
    #[inline]
    pub(crate) fn same_domain_handle(&self) -> Self {
        Self {
            domain: self.domain,
            time_domain: Arc::clone(&self.time_domain),
        }
    }

    /// Creates a shared wall clock projected from this clock's timeline.
    ///
    /// # Parameters
    ///
    /// * `wall_time` - Wall-clock value assigned to the current manual instant.
    ///
    /// # Returns
    ///
    /// A wall clock anchored to `wall_time` and driven by this exact clock.
    #[must_use]
    #[inline]
    pub fn new_wall_clock(
        self: &Arc<Self>,
        wall_time: SystemTime,
    ) -> Arc<ManualWallClock> {
        Arc::new(ManualWallClock::from_clock(wall_time, Arc::clone(self)))
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
    /// Panics if waking a registered task panics. Every waker collected for
    /// this advance is attempted before the first panic is resumed. The
    /// logical-time update is already committed and is not rolled back during
    /// unwinding.
    pub fn advance(&self, duration: Duration) -> Result<(), TimeError> {
        if let Some(effects) = self.time_domain.advance(duration)? {
            Self::notify_time_changed(effects);
        }
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
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign target.
    /// Returns [`TimeError::CannotMoveBackward`] for an earlier target,
    /// retaining both the current and requested elapsed durations.
    ///
    /// # Panics
    ///
    /// Panics if waking a registered task panics. Every waker collected for
    /// this advance is attempted before the first panic is resumed. The
    /// logical-time update is already committed and is not rolled back during
    /// unwinding.
    pub fn advance_to(
        &self,
        target: MonotonicInstant,
    ) -> Result<(), TimeError> {
        target.validate_domain(self.domain)?;
        let target_elapsed = target.elapsed_since_origin();
        if let Some(effects) = self.time_domain.advance_to(target_elapsed)? {
            Self::notify_time_changed(effects);
        }
        Ok(())
    }

    /// Returns the number of registered timer deadline waiters.
    ///
    /// A reached timer waiter remains registered and continues contributing to
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
        self.time_domain.waiter_count()
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
        self.time_domain
            .next_future_deadline()
            .map(|elapsed| MonotonicInstant::new(self.domain, elapsed))
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
    #[inline(always)]
    pub fn wait_for_next_deadline(
        &self,
        real_timeout: Duration,
    ) -> Option<MonotonicInstant> {
        self.time_domain
            .wait_for_next_deadline(real_timeout)
            .map(|elapsed| MonotonicInstant::new(self.domain, elapsed))
    }

    /// Returns a future that observes the earliest active future deadline.
    ///
    /// The observer is registered before this method returns, so a waiter
    /// created immediately afterward can wake the observing task. Registration
    /// does not latch a particular waiter or deadline. Each poll examines the
    /// clock's current waiter state and returns the earliest deadline strictly
    /// later than the current manual time. Cancelled registrations and
    /// registrations that are already due are ignored. When no active future
    /// deadline exists, the future stores the current task waker and remains
    /// pending.
    ///
    /// The returned instant is a snapshot selected while the clock state is
    /// locked. Another task may register an earlier deadline after that poll.
    /// Test drivers should therefore use
    /// [`advance_to_next_deadline()`](Self::advance_to_next_deadline) to choose
    /// the deadline atomically, rather than blindly advancing to the observed
    /// value.
    ///
    /// # Examples
    ///
    /// Coordinate an asynchronous producer with a manual-time driver:
    ///
    /// ```
    /// use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
    /// use std::time::Duration;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let clock = ManualMonotonicClock::new_shared();
    /// let timer = clock.new_timer();
    /// let task = tokio::spawn(async move {
    ///     timer
    ///         .after(Duration::from_secs(5))
    ///         .expect("timer deadline should register")
    ///         .await
    ///         .expect("timer should complete");
    /// });
    ///
    /// let observed = clock.wait_for_next_deadline_async().await;
    /// assert_eq!(Duration::from_secs(5), observed.elapsed_since_origin());
    /// let reached = clock
    ///     .advance_to_next_deadline()
    ///     .expect("the observed waiter should remain active");
    /// assert_eq!(observed, reached);
    /// task.await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Returns
    ///
    /// A cancellation-safe state observer that is already registered.
    ///
    /// # Panics
    ///
    /// Panics if the waiter-observer identifier space is exhausted.
    #[must_use]
    #[inline(always)]
    pub fn wait_for_next_deadline_async(
        self: &Arc<Self>,
    ) -> ManualDeadlineFuture {
        ManualDeadlineFuture::new(Arc::clone(self))
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
    /// Panics if waking a registered task panics. Every waker collected for
    /// this advance is attempted before the first panic is resumed. The
    /// logical-time update is already committed and is not rolled back during
    /// unwinding.
    pub fn advance_to_next_deadline(&self) -> Option<MonotonicInstant> {
        let (target_elapsed, effects) =
            self.time_domain.advance_to_next_deadline()?;
        let target = MonotonicInstant::new(self.domain, target_elapsed);
        Self::notify_time_changed(effects);
        Some(target)
    }

    /// Waits for enough timer waiters and advances to the earliest deadline.
    ///
    /// The current waiter count, earliest future deadline, and logical-time
    /// update are selected atomically under one clock-state lock. This avoids
    /// the cancellation gap created by separately calling
    /// [`wait_for_waiters()`](Self::wait_for_waiters) and
    /// [`advance_to_next_deadline()`](Self::advance_to_next_deadline).
    /// Registrations whose deadlines are already due may contribute to the
    /// count, but the clock advances only when a future deadline exists. A zero
    /// `expected_count` removes the count requirement without removing the
    /// future-deadline requirement. An already satisfied call advances before
    /// `real_timeout` is checked for representability.
    ///
    /// # Parameters
    ///
    /// * `expected_count` - Minimum active timer waiter count.
    /// * `real_timeout` - Maximum real time spent waiting for the count and a
    ///   future deadline.
    ///
    /// # Returns
    ///
    /// The same-domain instant reached by the clock. Returns `None` when the
    /// conditions remain unsatisfied until the real-time guard expires or the
    /// guard cannot be represented.
    ///
    /// # Panics
    ///
    /// Panics if waking a registered task panics. Every waker collected for
    /// this advance is attempted before the first panic is resumed. The
    /// logical-time update is already committed and is not rolled back during
    /// unwinding.
    #[inline]
    pub fn advance_to_next_deadline_after_waiters(
        &self,
        expected_count: usize,
        real_timeout: Duration,
    ) -> Option<MonotonicInstant> {
        let (target_elapsed, effects) =
            self.time_domain.advance_to_next_deadline_after_waiters(
                expected_count,
                real_timeout,
            )?;
        let target = MonotonicInstant::new(self.domain, target_elapsed);
        Self::notify_time_changed(effects);
        Some(target)
    }

    /// Waits for and advances to the earliest registered future deadline.
    ///
    /// Observation begins when the returned future is first polled. If the
    /// observed deadline is cancelled before the clock can advance, this
    /// method waits again. A concurrently registered earlier deadline may be
    /// selected because
    /// [`advance_to_next_deadline()`](Self::advance_to_next_deadline) performs
    /// selection and advancement atomically.
    ///
    /// Cancelling the returned future does not advance the clock or alter any
    /// timer registration.
    ///
    /// # Returns
    ///
    /// The same-domain instant actually reached by the clock.
    ///
    /// # Panics
    ///
    /// Panics if waiter-observer identifiers are exhausted or if waking a
    /// registered task panics.
    pub async fn advance_to_next_deadline_async(
        self: &Arc<Self>,
    ) -> MonotonicInstant {
        loop {
            let _ = self.wait_for_next_deadline_async().await;
            if let Some(deadline) = self.advance_to_next_deadline() {
                return deadline;
            }
        }
    }

    /// Returns a future that completes after enough waiters are registered.
    ///
    /// Timer deadline waiters contribute to the count. Reaching the count is
    /// latched even if waiters unregister before
    /// the returned future is polled again. The `self: &Arc<Self>` receiver
    /// ensures the future keeps this exact clock instance alive. The observer
    /// is registered before this method returns, rather than on the first poll.
    /// Reached timer waiters remain counted until their futures are polled or
    /// dropped, so a due registration can satisfy `expected_count`. Use
    /// [`wait_for_next_deadline_async()`](Self::wait_for_next_deadline_async)
    /// when coordinating a later stage that specifically requires an active
    /// future deadline instead of a historical count threshold.
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
    /// Timer registrations contribute to `expected_count`. Reaching the count
    /// is latched even if waiters
    /// unregister before this thread reacquires the clock state. `real_timeout`
    /// is only a test guard and never advances logical time. Returns `true`
    /// when the count is reached, including when it is already satisfied before
    /// an unrepresentable guard is evaluated. Returns `false` when an
    /// unsatisfied wait reaches a guard that expires or cannot be represented.
    /// Reached timer waiters remain counted until their futures are polled or
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
    #[inline(always)]
    pub fn wait_for_waiters(
        &self,
        expected_count: usize,
        real_timeout: Duration,
    ) -> bool {
        self.time_domain
            .wait_for_waiters(expected_count, real_timeout)
    }

    /// Registers a timer deadline at future creation time.
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
    pub(crate) fn register_timer_waiter(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<Option<u64>, TimeError> {
        deadline.validate_domain(self.domain)?;
        let deadline_elapsed = deadline.elapsed_since_origin();
        let Some((waiter_id, observer_wakers)) =
            self.time_domain.register_timer_waiter(deadline_elapsed)
        else {
            return Ok(None);
        };
        let registration = WaiterRegistrationGuard::new(self, waiter_id);
        self.time_domain.notify_waiters_changed();
        let mut fanout = PanicFanout::new();
        fanout.wake_all(observer_wakers);
        fanout.resume_first_panic();
        Ok(Some(registration.into_waiter_id()))
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
        self.time_domain.register_waiter_observer(expected_count)
    }

    /// Registers an asynchronous observer of the next future deadline.
    ///
    /// # Returns
    ///
    /// The nonzero identifier assigned to the deadline observer.
    ///
    /// # Panics
    ///
    /// Panics when the observer identifier space is exhausted.
    #[must_use]
    #[inline]
    pub(crate) fn register_deadline_observer(&self) -> u64 {
        self.time_domain.register_deadline_observer()
    }

    /// Polls an asynchronous observer of the next future deadline.
    ///
    /// # Parameters
    ///
    /// * `observer_id` - Identifier of the deadline observer to poll.
    /// * `context` - Task context whose waker is retained while pending.
    ///
    /// # Returns
    ///
    /// [`Poll::Ready`] with the observed deadline, otherwise
    /// [`Poll::Pending`].
    ///
    /// # Panics
    ///
    /// Panics if the observer is unexpectedly missing or, after releasing the
    /// clock state lock, if destroying a replaced custom task waker panics.
    #[inline]
    pub(crate) fn poll_deadline_observer(
        &self,
        observer_id: u64,
        context: &Context<'_>,
    ) -> Poll<MonotonicInstant> {
        let (poll_result, replaced_waker) = self
            .time_domain
            .poll_deadline_observer(observer_id, context);
        drop(replaced_waker);
        poll_result.map(|elapsed| MonotonicInstant::new(self.domain, elapsed))
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
        let (poll_result, replaced_waker) =
            self.time_domain.poll_waiter_observer(observer_id, context);
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
        let removed_waker =
            self.time_domain.unregister_waiter_observer(observer_id);
        drop(removed_waker);
    }

    /// Polls a registered timer waiter against current manual time.
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
    /// # Panics
    ///
    /// Panics if `waiter_id` no longer identifies a registered timer waiter or
    /// if destroying a replaced custom task waker panics after unlocking.
    pub(crate) fn poll_timer_waiter(
        &self,
        waiter_id: u64,
        context: &Context<'_>,
    ) -> Poll<()> {
        let (poll_result, replaced_waker) =
            self.time_domain.poll_timer_waiter(waiter_id, context);
        drop(replaced_waker);
        poll_result
    }

    /// Removes a timer waiter after completion or future cancellation.
    ///
    /// # Parameters
    ///
    /// * `waiter_id` - Identifier of the timer waiter to remove.
    ///
    /// # Panics
    ///
    /// Panics after releasing the clock state lock if destroying the waiter's
    /// custom task waker panics.
    #[inline]
    pub(crate) fn unregister_timer_waiter(&self, waiter_id: u64) {
        let removed_waiter =
            self.time_domain.unregister_timer_waiter(waiter_id);
        drop(removed_waiter);
    }

    /// Wakes time observers outside the state lock.
    ///
    /// # Parameters
    ///
    /// * `effects` - Due task wakers to notify.
    ///
    /// # Panics
    ///
    /// Resumes the first panic raised by a waker or its destructor after every
    /// collected target has been attempted.
    fn notify_time_changed(effects: AdvanceEffects) {
        let AdvanceEffects { due_wakers } = effects;
        let mut fanout = PanicFanout::new();
        fanout.wake_all(due_wakers);
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
        MonotonicInstant::new(self.domain, self.time_domain.elapsed())
    }

    /// Creates a timer retaining this exact manual time domain.
    ///
    /// # Returns
    ///
    /// A shared timer driven by the same explicitly advanced timeline.
    #[inline]
    fn new_timer(&self) -> Arc<dyn Timer> {
        Arc::new(ManualTimer::from_clock(self))
    }
}
