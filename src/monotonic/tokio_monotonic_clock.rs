// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the Tokio monotonic clock implementation.

use crate::{ClockDomain, MonotonicClock, MonotonicInstant, Timer, TokioRuntimeError, TokioTimer};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::time::Instant;

/// Runs one synchronous operation in the target runtime context.
///
/// Tokio task hooks execute while the runtime context is already borrowed.
/// Re-entering that same runtime from such a hook panics, so an existing
/// matching context is reused.
///
/// # Parameters
///
/// * `runtime` - Runtime context required by `operation`.
/// * `operation` - Synchronous operation to execute.
///
/// # Returns
///
/// The value returned by `operation`.
#[inline]
fn within_runtime<R>(runtime: &Handle, operation: impl FnOnce() -> R) -> R {
    let is_current = Handle::try_current().is_ok_and(|current| current.id() == runtime.id());
    if is_current {
        return operation();
    }
    let _runtime_guard = runtime.enter();
    operation()
}

/// A monotonic clock backed by Tokio's time driver.
///
/// The clock retains a [`Handle`] and enters that runtime briefly whenever it
/// samples Tokio time. It therefore follows the retained runtime's pause and
/// advance semantics without requiring callers to enter that runtime. The
/// runtime owner must remain alive while the clock or a derived timer is used.
/// A pending derived timer reports
/// [`TimerUnavailableError::RuntimeShuttingDown`](crate::TimerUnavailableError::RuntimeShuttingDown)
/// if that runtime shuts down before its future completes.
///
/// The type intentionally does not implement [`Clone`]; shared identity uses
/// `Arc<TokioMonotonicClock>`. Derived timers retain an independent handle with
/// the same clock domain and Tokio origin.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioMonotonicClock {
    /// Domain carried by instants sampled from this clock.
    domain: ClockDomain,
    /// Native Tokio instant mapped to elapsed duration zero.
    origin: Instant,
    /// Tokio runtime capability used for time sampling and timer registration.
    runtime: Handle,
}

impl TokioMonotonicClock {
    /// Creates a Tokio clock backed by an explicit runtime handle.
    ///
    /// This constructor does not depend on an ambient Tokio runtime. Clock
    /// samples and derived timers use `runtime` even when called or polled from
    /// another runtime context.
    ///
    /// # Parameters
    ///
    /// * `runtime` - Runtime capability providing the Tokio time source.
    ///
    /// # Returns
    ///
    /// A Tokio monotonic clock with a new clock domain.
    ///
    /// # Panics
    ///
    /// Panics if all process-wide clock-domain identifiers are exhausted.
    #[must_use]
    #[inline]
    pub fn from_handle(runtime: Handle) -> Self {
        let domain = ClockDomain::new();
        let origin = within_runtime(&runtime, Instant::now);
        Self {
            domain,
            origin,
            runtime,
        }
    }

    /// Creates a Tokio clock by capturing the currently entered runtime.
    ///
    /// # Returns
    ///
    /// A Tokio monotonic clock retaining the current runtime's handle.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime is entered or all process-wide clock-domain
    /// identifiers are exhausted.
    #[must_use]
    #[track_caller]
    #[inline]
    pub fn current() -> Self {
        Self::try_current()
            .unwrap_or_else(|error| panic!("cannot create Tokio monotonic clock: {error}"))
    }

    /// Tries to create a Tokio clock by capturing the current runtime.
    ///
    /// # Returns
    ///
    /// A Tokio monotonic clock retaining the current runtime's handle.
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
        let runtime =
            Handle::try_current().map_err(|source| TokioRuntimeError::NotEntered { source })?;
        Ok(Self::from_handle(runtime))
    }

    /// Creates a private handle retaining this exact Tokio clock domain.
    ///
    /// # Returns
    ///
    /// A clock handle with the same domain identifier and Tokio origin.
    #[must_use]
    #[inline]
    pub(crate) fn same_domain_handle(&self) -> Self {
        Self {
            domain: self.domain,
            origin: self.origin,
            runtime: self.runtime.clone(),
        }
    }

    /// Returns the Tokio origin used by the paired timer.
    ///
    /// # Returns
    ///
    /// The Tokio instant mapped to elapsed duration zero.
    #[must_use]
    #[inline(always)]
    pub(crate) const fn origin(&self) -> Instant {
        self.origin
    }

    /// Returns this concrete clock's domain without sampling Tokio time.
    ///
    /// # Returns
    ///
    /// This clock's process-unique domain.
    #[inline(always)]
    pub(crate) const fn domain(&self) -> ClockDomain {
        self.domain
    }

    /// Runs one synchronous operation inside the retained runtime context.
    ///
    /// The runtime guard is dropped before this method returns and must never
    /// escape into an asynchronous future.
    ///
    /// # Parameters
    ///
    /// * `operation` - Synchronous operation requiring Tokio runtime context.
    ///
    /// # Returns
    ///
    /// The value returned by `operation`.
    #[inline]
    pub(crate) fn with_runtime<R>(&self, operation: impl FnOnce() -> R) -> R {
        within_runtime(&self.runtime, operation)
    }
}

impl MonotonicClock for TokioMonotonicClock {
    /// Returns the current instant in this clock's domain.
    ///
    /// # Returns
    ///
    /// The current elapsed duration represented in this clock's domain.
    #[inline]
    fn now(&self) -> MonotonicInstant {
        let elapsed = self.with_runtime(|| self.origin.elapsed());
        MonotonicInstant::new(self.domain, elapsed)
    }

    /// Creates a timer retaining this exact Tokio clock domain and origin.
    ///
    /// # Returns
    ///
    /// A shared timer bound to the same Tokio runtime.
    #[inline]
    fn new_timer(&self) -> Arc<dyn Timer> {
        Arc::new(TokioTimer::from_clock(self))
    }
}
