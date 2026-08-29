// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a timer driven by Tokio's time driver.

// qubit-style: allow coverage-cfg

use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::OnceLock;
#[cfg(coverage)]
use std::sync::atomic::AtomicBool;
#[cfg(coverage)]
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::time::Instant;
use tokio::time::sleep_until;

use crate::MonotonicClock;
use crate::MonotonicInstant;
use crate::TimeError;
use crate::Timer;
use crate::TimerFuture;
use crate::TimerUnavailableError;
use crate::TokioMonotonicClock;
use crate::TokioRuntimeError;
use crate::timer::internal::tokio_runtime_liveness::TokioRuntimeLiveness;
use crate::timer::internal::tokio_runtime_liveness_registry::TokioRuntimeLivenessRegistry;
use crate::timer::internal::tokio_timer_future::TokioTimerFuture;

#[cfg(coverage)]
static PANIC_NEXT_SLEEP_POLL: AtomicBool = AtomicBool::new(false);

/// Makes the next Tokio Timer sleep poll panic deterministically.
///
/// This coverage-only hook exercises the defensive path for an unexpected
/// Tokio sleep panic after the shared liveness check.
#[cfg(coverage)]
pub fn panic_next_tokio_timer_sleep_poll() {
    PANIC_NEXT_SLEEP_POLL.store(true, Ordering::Release);
}

/// Takes the coverage-only sleep-poll panic request.
#[cfg(coverage)]
pub(crate) fn take_tokio_timer_sleep_poll_panic() -> bool {
    PANIC_NEXT_SLEEP_POLL.swap(false, Ordering::AcqRel)
}

/// An asynchronous timer backed by one Tokio runtime time driver.
///
/// The timer retains the source clock's exact domain, origin, and runtime
/// capability. It enters that runtime briefly to sample time and create each
/// Tokio sleep, so registration does not depend on the caller's ambient
/// runtime. The returned future may be polled elsewhere, but the retained
/// runtime must remain alive and driven until the future completes or is
/// dropped. If it shuts down first, a pending future returns
/// [`TimerUnavailableError::RuntimeShuttingDown`].
///
/// # Resolution
///
/// Logical deadlines preserve the full [`Duration`], but Tokio drives pending
/// sleeps with millisecond-level scheduling granularity. This timer is not for
/// high-resolution timing, and platform scheduling may add further delay.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioTimer {
    /// Private handle retaining the source clock domain and Tokio origin.
    clock: TokioMonotonicClock,
    /// Lazily resolved liveness shared by timers on the retained runtime.
    liveness: OnceLock<Arc<TokioRuntimeLiveness>>,
}

impl TokioTimer {
    /// Creates a timer backed by an explicit runtime handle.
    ///
    /// This constructor does not depend on an ambient Tokio runtime.
    ///
    /// # Parameters
    ///
    /// * `runtime` - Runtime capability providing the clock and time driver.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain backed by `runtime`.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn from_handle(runtime: Handle) -> Self {
        Self {
            clock: TokioMonotonicClock::from_handle(runtime),
            liveness: OnceLock::new(),
        }
    }

    /// Creates a timer by capturing the currently entered Tokio runtime.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain retaining the current runtime's handle.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime is entered or all process-wide clock-domain
    /// identifiers are exhausted.
    #[must_use]
    #[track_caller]
    #[inline]
    pub fn current() -> Self {
        Self::try_current().unwrap_or_else(|error| panic!("cannot create Tokio timer: {error}"))
    }

    /// Tries to create a timer by capturing the current Tokio runtime.
    ///
    /// # Returns
    ///
    /// A timer with a new clock domain retaining the current runtime's handle.
    ///
    /// # Errors
    ///
    /// Returns [`TokioRuntimeError::NotEntered`] when no Tokio runtime is
    /// entered.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[inline]
    pub fn try_current() -> Result<Self, TokioRuntimeError> {
        TokioMonotonicClock::try_current().map(|clock| Self {
            clock,
            liveness: OnceLock::new(),
        })
    }

    /// Creates a timer sharing the supplied Tokio clock's exact domain.
    ///
    /// # Parameters
    ///
    /// * `clock` - Tokio clock whose domain, origin, and runtime capability
    ///   apply.
    ///
    /// # Returns
    ///
    /// A timer retaining an independent same-domain clock handle.
    #[must_use]
    #[inline]
    pub fn from_clock(clock: &TokioMonotonicClock) -> Self {
        Self {
            clock: clock.same_domain_handle(),
            liveness: OnceLock::new(),
        }
    }

    /// Converts a domain-scoped deadline to its native Tokio instant.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Absolute deadline in the source clock domain.
    ///
    /// # Returns
    ///
    /// The corresponding Tokio instant.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ClockDomainMismatch`] for a foreign deadline and
    /// [`TimeError::InstantOverflow`] when conversion overflows.
    fn native_deadline(&self, deadline: MonotonicInstant) -> Result<Instant, TimeError> {
        deadline.validate_domain(self.clock.domain())?;
        self.clock
            .origin()
            .checked_add(deadline.elapsed_since_origin())
            .ok_or(TimeError::InstantOverflow)
    }

    /// Returns runtime liveness without a reentrant `OnceLock` initializer.
    ///
    /// Tokio invokes task-spawn hooks synchronously. Publishing liveness in the
    /// registry may therefore re-enter this same timer while its first
    /// registration is still in progress.
    ///
    /// # Returns
    ///
    /// Liveness shared by timers retaining the same Tokio runtime.
    fn runtime_liveness(&self) -> Arc<TokioRuntimeLiveness> {
        if let Some(liveness) = self.liveness.get() {
            return Arc::clone(liveness);
        }
        let liveness = TokioRuntimeLivenessRegistry::current();
        let _ = self.liveness.set(liveness);
        Arc::clone(self.liveness.get().expect("Tokio timer liveness should be initialized"))
    }

    /// Creates the future for one native deadline while the target runtime is
    /// entered.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Fixed native Tokio deadline.
    /// * `now` - Single current-time sample used to detect reached deadlines.
    ///
    /// # Returns
    ///
    /// An immediately ready future or a Tokio sleep paired with shared
    /// retained-runtime liveness. Dropping the future cancels only its sleep
    /// and releases its liveness reference; the sentinel remains active while
    /// a timer or another pending future on that runtime retains it.
    ///
    /// # Errors
    ///
    /// Returns [`TimerUnavailableError::TimeDriverDisabled`] when a future
    /// deadline cannot be registered because Tokio time is disabled.
    fn schedule(&self, deadline: Instant, now: Instant) -> Result<TimerFuture, TimeError> {
        if deadline <= now {
            return Ok(Box::pin(std::future::ready(Ok(()))));
        }
        // Tokio 1.52 exposes no public query for whether a Handle has a time
        // driver. Catching the constructor panic preserves a typed error in
        // unwind builds, but the process panic hook runs first and panic=abort
        // cannot recover. Temporarily replacing the global hook would race
        // with application panic handling, so this library deliberately does
        // not attempt to suppress that observable side effect.
        let sleep =
            catch_unwind(AssertUnwindSafe(|| sleep_until(deadline))).map_err(|_| TimeError::TimerUnavailable {
                source: TimerUnavailableError::TimeDriverDisabled,
            })?;
        // Criterion's `tokio_timer` benchmark showed that 10,240 legacy
        // per-deadline sentinels retained 10,240 tasks and made registration
        // more than 20% slower than native sleeps. One lazy sentinel per
        // retained runtime preserves structured shutdown errors without that
        // scaling cost.
        let liveness = self.runtime_liveness();
        Ok(Box::pin(TokioTimerFuture::new(sleep, liveness)))
    }
}

impl Timer for TokioTimer {
    /// Returns the private same-domain Tokio clock handle.
    ///
    /// # Returns
    ///
    /// The monotonic clock driving this timer.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Creates a Tokio sleep with a fixed absolute deadline.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Deadline in this timer's clock domain.
    ///
    /// # Returns
    ///
    /// A future waiting for the fixed deadline, or an immediately ready future
    /// for a reached deadline in the retained runtime's time domain. If the
    /// retained runtime shuts down before a pending future completes, that
    /// future returns [`TimerUnavailableError::RuntimeShuttingDown`].
    ///
    /// # Errors
    ///
    /// Returns a domain mismatch or instant overflow before runtime access.
    /// Returns [`TimerUnavailableError::TimeDriverDisabled`] when a future
    /// deadline requires a time driver that the retained runtime did not
    /// enable. Reached deadlines do not require a time driver.
    /// In unwind builds Tokio's panic hook may observe this disabled-driver
    /// condition before it is converted into the structured error. In
    /// `panic = "abort"` builds Tokio aborts before conversion is possible.
    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        let deadline = self.native_deadline(deadline)?;
        self.clock.with_runtime(|| self.schedule(deadline, Instant::now()))
    }

    /// Registers a notification after a duration in the retained Tokio
    /// runtime.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration from the retained runtime's current instant.
    ///
    /// # Returns
    ///
    /// A future that becomes ready when the fixed deadline is reached.
    /// If the retained runtime shuts down before that happens, the future
    /// returns [`TimerUnavailableError::RuntimeShuttingDown`].
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] when the relative deadline cannot
    /// be represented, or [`TimerUnavailableError::TimeDriverDisabled`] when a
    /// nonzero future deadline requires a disabled time driver.
    /// In unwind builds Tokio's panic hook may observe this disabled-driver
    /// condition before it is converted into the structured error. In
    /// `panic = "abort"` builds Tokio aborts before conversion is possible.
    #[inline]
    fn after(&self, duration: Duration) -> Result<TimerFuture, TimeError> {
        self.clock.with_runtime(|| {
            let now = Instant::now();
            let deadline = now.checked_add(duration).ok_or(TimeError::InstantOverflow)?;
            self.schedule(deadline, now)
        })
    }
}
