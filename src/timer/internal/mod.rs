// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal timer future implementations.

pub(crate) mod manual_timer_future;
pub(crate) mod std_timer_future;
pub(crate) mod std_timer_registration;
pub(crate) mod std_timer_scheduler;
pub(crate) mod std_timer_scheduler_state;
pub(crate) mod std_timer_waiter;
pub(crate) mod std_timer_waiter_state;
pub(crate) mod std_timer_worker_guard;

#[cfg(feature = "tokio")]
pub(crate) mod tokio_runtime_liveness;
#[cfg(feature = "tokio")]
pub(crate) mod tokio_runtime_liveness_registry;
#[cfg(feature = "tokio")]
pub(crate) mod tokio_runtime_shutdown_guard;
#[cfg(feature = "tokio")]
pub(crate) mod tokio_runtime_shutdown_state;
#[cfg(feature = "tokio")]
pub(crate) mod tokio_timer_future;
