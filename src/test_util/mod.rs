// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Test-only Timer implementations for deterministic failure injection.
//!
//! Enable the `test-util` Cargo feature from development dependencies. The
//! feature is disabled by default and does not require an asynchronous runtime.

mod fault_injecting_timer;
mod timer_failure_point;

#[cfg(all(loom, feature = "loom-model"))]
#[doc(hidden)]
pub mod loom;

pub use fault_injecting_timer::FaultInjectingTimer;
pub use timer_failure_point::TimerFailurePoint;
