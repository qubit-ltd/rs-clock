// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Exposes two pure internal state machines to focused integration tests.
//!
//! The project keeps tests outside `src/`, while identifier exhaustion cannot
//! be reached through the process-global public APIs in a practical test. These
//! path modules are therefore a narrow exception used only for terminal
//! identifier transitions, waiter-observer latching, and impossible async
//! waiter lifecycle states. Production behavior is otherwise exercised through
//! the public `qubit_clock` API; do not add general implementation tests to
//! this support module.

#[allow(dead_code)]
#[path = "../../src/monotonic/clock_domain.rs"]
pub(crate) mod clock_domain;

#[allow(dead_code)]
#[path = "../../src/monotonic/internal/manual_waiter_registry.rs"]
pub(crate) mod manual_waiter_registry;
