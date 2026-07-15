// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Removes a waiter registration if control unwinds before ownership transfers.

use super::registered_waiter::RegisteredWaiter;
use crate::monotonic::manual_monotonic_clock::ManualMonotonicClock;

/// Removes a waiter registration if control unwinds before ownership is
/// transferred to its normal blocking or async lifetime.
pub(crate) struct WaiterRegistrationGuard<'a> {
    /// Clock that owns the registration.
    clock: &'a ManualMonotonicClock,
    /// Registration still owned by this guard.
    waiter: Option<RegisteredWaiter>,
}

impl<'a> WaiterRegistrationGuard<'a> {
    /// Guards a newly registered blocking waiter.
    #[inline(always)]
    pub(crate) fn blocking(clock: &'a ManualMonotonicClock, waiter_id: u64) -> Self {
        Self {
            clock,
            waiter: Some(RegisteredWaiter::Blocking(waiter_id)),
        }
    }

    /// Guards a newly registered async waiter.
    #[inline(always)]
    pub(crate) fn asynchronous(clock: &'a ManualMonotonicClock, waiter_id: u64) -> Self {
        Self {
            clock,
            waiter: Some(RegisteredWaiter::Async(waiter_id)),
        }
    }

    /// Transfers an async registration to the returned future.
    #[inline]
    pub(crate) fn into_async_waiter_id(mut self) -> u64 {
        let Some(RegisteredWaiter::Async(waiter_id)) = self.waiter.take()
        else {
            unreachable!("only async registration guards can be transferred");
        };
        waiter_id
    }
}

impl Drop for WaiterRegistrationGuard<'_> {
    /// Removes a registration still owned by this guard.
    #[inline]
    fn drop(&mut self) {
        match self.waiter.take() {
            Some(RegisteredWaiter::Blocking(waiter_id)) => {
                self.clock.unregister_blocking_waiter(waiter_id);
            }
            Some(RegisteredWaiter::Async(waiter_id)) => {
                self.clock.unregister_async_waiter(waiter_id);
            }
            None => {}
        }
    }
}
