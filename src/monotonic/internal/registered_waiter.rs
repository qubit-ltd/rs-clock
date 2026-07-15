// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Kind and identifier of one waiter whose registration needs cleanup.

/// Kind and identifier of one waiter whose registration needs cleanup.
pub(crate) enum RegisteredWaiter {
    /// Blocking waiter registration.
    Blocking(
        /// Identifier allocated by the blocking waiter registry.
        u64,
    ),
    /// Async waiter registration.
    Async(
        /// Identifier allocated by the async waiter registry.
        u64,
    ),
}
