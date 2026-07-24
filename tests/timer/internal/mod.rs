// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Provides task wakers used by Timer integration tests.

mod manual_timer_future_tests;
mod panicking_waker_tests;
mod std_timer_future_tests;
mod std_timer_registration_tests;
mod std_timer_scheduler_state_tests;
mod std_timer_waiter_state_tests;
mod std_timer_waiter_tests;
mod std_timer_worker_guard_tests;

#[cfg(feature = "tokio")]
mod tokio_runtime_liveness_registry_tests;
#[cfg(feature = "tokio")]
mod tokio_runtime_liveness_tests;
#[cfg(feature = "tokio")]
mod tokio_runtime_shutdown_guard_tests;
#[cfg(feature = "tokio")]
mod tokio_runtime_shutdown_state_tests;
#[cfg(feature = "tokio")]
mod tokio_timer_future_tests;

pub(super) use panicking_waker_tests::{DestructorPanickingWaker, PanickingWaker};
