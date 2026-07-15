// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

pub(crate) use crate::support::clock_domain;
mod clock_domain_tests;
mod manual_advance_subscription_tests;
mod manual_monotonic_clock_tests;
mod manual_monotonic_state_tests;
mod manual_waiter_future_tests;
mod manual_waiter_registry_tests;
mod monotonic_clock_tests;
mod monotonic_instant_tests;
mod std_monotonic_clock_tests;

#[cfg(feature = "tokio")]
mod tokio_monotonic_clock_tests;
