// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Internal helpers for the manual monotonic clock.

mod advance_effects;
mod manual_advance_registry;
mod manual_monotonic_state;
mod manual_waiter_registry;
mod panic_fanout;
mod registered_waiter;
mod waiter_registration_guard;

pub(crate) use advance_effects::AdvanceEffects;
pub(crate) use manual_monotonic_state::ManualMonotonicState;
pub(crate) use panic_fanout::PanicFanout;
pub(crate) use waiter_registration_guard::WaiterRegistrationGuard;
