// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the Tokio monotonic clock implementation.

use crate::{
    ClockDomain,
    MonotonicClock,
    MonotonicInstant,
    Timer,
    TokioRuntimeError,
    TokioTimer,
};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::time::Instant;

/// A monotonic clock backed by Tokio's time driver.
///
/// It follows the pause and advance semantics of the Tokio runtime entered at
/// construction. The binding is permanent and validated before every sample;
/// moving tasks between worker threads of that runtime is supported, while
/// sampling from an independent runtime is rejected.
///
/// The type intentionally does not implement [`Clone`]; shared identity uses
/// `Arc<TokioMonotonicClock>`. Use [`Self::try_now`] when runtime-affinity
/// failures must be handled without a panic.
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[derive(Debug)]
pub struct TokioMonotonicClock {
    /// Domain carried by instants sampled from this clock.
    domain: ClockDomain,
    /// Native Tokio instant mapped to elapsed duration zero.
    origin: Instant,
    /// Tokio runtime capability retained for identity validation.
    runtime: Handle,
}

impl TokioMonotonicClock {
    /// Creates a Tokio clock bound to the currently entered runtime.
    ///
    /// # Returns
    ///
    /// A Tokio monotonic clock bound to the current runtime.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime is entered or all process-wide clock-domain
    /// identifiers are exhausted.
    #[must_use]
    #[track_caller]
    #[inline]
    pub fn current() -> Self {
        Self::try_current().unwrap_or_else(|error| {
            panic!("cannot create Tokio monotonic clock: {error}")
        })
    }

    /// Tries to create a Tokio clock bound to the currently entered runtime.
    ///
    /// # Returns
    ///
    /// A Tokio monotonic clock bound to the current runtime.
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
        let runtime = Handle::try_current()
            .map_err(|source| TokioRuntimeError::NotEntered { source })?;
        Ok(Self {
            domain: ClockDomain::new(),
            origin: Instant::now(),
            runtime,
        })
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

    /// Tries to sample the current instant from the bound Tokio runtime.
    ///
    /// # Returns
    ///
    /// The current elapsed duration represented in this clock's domain.
    ///
    /// # Errors
    ///
    /// Returns [`TokioRuntimeError::NotEntered`] when no Tokio runtime is
    /// entered, or [`TokioRuntimeError::Mismatch`] when a different runtime is
    /// entered.
    #[inline]
    pub fn try_now(&self) -> Result<MonotonicInstant, TokioRuntimeError> {
        self.ensure_current_runtime()?;
        Ok(MonotonicInstant::new(self.domain, self.origin.elapsed()))
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

    /// Verifies that the bound Tokio runtime is currently entered.
    ///
    /// # Errors
    ///
    /// Returns [`TokioRuntimeError::NotEntered`] when no Tokio runtime is
    /// entered, or [`TokioRuntimeError::Mismatch`] when the entered runtime
    /// differs from the bound runtime.
    #[inline]
    pub(crate) fn ensure_current_runtime(
        &self,
    ) -> Result<(), TokioRuntimeError> {
        let actual = Handle::try_current()
            .map_err(|source| TokioRuntimeError::NotEntered { source })?
            .id();
        let expected = self.runtime.id();
        if actual == expected {
            Ok(())
        } else {
            Err(TokioRuntimeError::Mismatch { expected, actual })
        }
    }
}

impl MonotonicClock for TokioMonotonicClock {
    /// Returns the current instant in this clock's domain.
    ///
    /// # Returns
    ///
    /// The current elapsed duration represented in this clock's domain.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime is entered or the entered runtime differs
    /// from the runtime bound at construction. Use [`Self::try_now`] to handle
    /// either condition without a panic.
    #[track_caller]
    #[inline]
    fn now(&self) -> MonotonicInstant {
        self.try_now().unwrap_or_else(|error| {
            panic!("cannot sample Tokio monotonic clock: {error}")
        })
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
