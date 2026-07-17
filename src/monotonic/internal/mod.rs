// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Internal helpers for the manual monotonic clock.

mod advance_effects;
mod manual_monotonic_state;
mod manual_time_domain;
mod manual_waiter_registry;
mod panic_fanout;
mod waiter_registration_guard;

pub(crate) use advance_effects::AdvanceEffects;
pub(crate) use manual_monotonic_state::ManualMonotonicState;
pub(crate) use manual_time_domain::ManualTimeDomain;
pub(crate) use panic_fanout::PanicFanout;
pub(crate) use waiter_registration_guard::WaiterRegistrationGuard;
