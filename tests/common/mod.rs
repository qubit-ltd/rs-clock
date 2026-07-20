// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Exposes production synchronization state machines to Loom model tests.
//!
//! The concrete model assertions remain in their mirrored external test files.
//! This support module only makes the exact production implementations
//! available inside those integration-test crates.

#[allow(dead_code)]
#[path = "../../src/sleep/internal/notification_latch.rs"]
pub(crate) mod notification_latch;

#[allow(dead_code)]
#[path = "../../src/timer/internal/std_timer_waiter.rs"]
pub(crate) mod std_timer_waiter;

#[allow(dead_code)]
#[path = "../../src/timer/internal/std_timer_waiter_state.rs"]
pub(crate) mod std_timer_waiter_state;
