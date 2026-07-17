// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Internal timer future implementations.

pub(crate) mod manual_timer_future;
pub(crate) mod std_timer_future;
pub(crate) mod std_timer_scheduler;
pub(crate) mod std_timer_waiter;
pub(crate) mod std_timer_worker_guard;
