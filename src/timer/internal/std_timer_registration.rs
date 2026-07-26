// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stores one active standard Timer registration.

use super::std_timer_waiter::StdTimerWaiter;
use std::sync::Arc;
use std::time::Instant;

/// Associates a native deadline with its completion waiter.
pub(super) struct StdTimerRegistration {
    /// Native deadline indexed by the scheduler.
    deadline: Instant,
    /// Completion latch shared with the returned future.
    waiter: Arc<StdTimerWaiter>,
}

impl StdTimerRegistration {
    /// Creates a registration for `waiter` at `deadline`.
    ///
    /// # Parameters
    ///
    /// * `deadline` - Native standard-library deadline.
    /// * `waiter` - Completion latch shared with the returned future.
    ///
    /// # Returns
    ///
    /// A registration ready to be inserted into scheduler state.
    #[must_use]
    #[inline(always)]
    pub(super) const fn new(deadline: Instant, waiter: Arc<StdTimerWaiter>) -> Self {
        Self { deadline, waiter }
    }

    /// Returns the native deadline indexed for this registration.
    ///
    /// # Returns
    ///
    /// The registration's standard-library deadline.
    #[must_use]
    #[inline(always)]
    pub(super) const fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Consumes this registration and returns its completion waiter.
    ///
    /// # Returns
    ///
    /// The waiter formerly owned by this active registration.
    #[must_use]
    #[inline(always)]
    pub(super) fn into_waiter(self) -> Arc<StdTimerWaiter> {
        self.waiter
    }
}
