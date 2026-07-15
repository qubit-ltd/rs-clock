// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Exposes internal source modules to focused integration behavior tests.

#[allow(dead_code)]
#[path = "../../src/monotonic/clock_domain.rs"]
pub(crate) mod clock_domain;

#[allow(dead_code)]
#[path = "../../src/monotonic/internal/manual_waiter_registry.rs"]
pub(crate) mod manual_waiter_registry;
