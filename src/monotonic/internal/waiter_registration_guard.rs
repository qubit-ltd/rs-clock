// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Removes a timer registration if control unwinds before ownership transfers.

use crate::monotonic::manual_monotonic_clock::ManualMonotonicClock;

/// Guards one newly registered manual timer waiter during notification fanout.
#[must_use = "dropping the guard unregisters the timer waiter"]
pub(crate) struct WaiterRegistrationGuard<'a> {
    /// Clock whose shared time domain owns the registration.
    clock: &'a ManualMonotonicClock,
    /// Registration still owned by this guard.
    waiter_id: Option<u64>,
}

impl<'a> WaiterRegistrationGuard<'a> {
    /// Guards a newly registered timer waiter.
    ///
    /// # Parameters
    ///
    /// * `clock` - Manual clock whose domain owns the waiter.
    /// * `waiter_id` - Identifier of the timer registration.
    ///
    /// # Returns
    ///
    /// A guard that unregisters the waiter unless ownership is transferred.
    #[inline(always)]
    pub(crate) const fn new(
        clock: &'a ManualMonotonicClock,
        waiter_id: u64,
    ) -> Self {
        Self {
            clock,
            waiter_id: Some(waiter_id),
        }
    }

    /// Transfers the timer registration to its returned future.
    ///
    /// # Returns
    ///
    /// The identifier of the transferred timer waiter.
    #[must_use = "the transferred waiter identifier must be retained"]
    #[inline]
    pub(crate) fn into_waiter_id(mut self) -> u64 {
        self.waiter_id
            .take()
            .expect("registration guard must own a timer waiter")
    }
}

impl Drop for WaiterRegistrationGuard<'_> {
    /// Removes a timer registration still owned during unwinding.
    #[inline]
    fn drop(&mut self) {
        if let Some(waiter_id) = self.waiter_id.take() {
            self.clock.unregister_timer_waiter(waiter_id);
        }
    }
}
