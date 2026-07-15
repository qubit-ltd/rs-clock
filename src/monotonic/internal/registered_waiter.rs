// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Kind and identifier of one waiter whose registration needs cleanup.

/// Kind and identifier of one waiter whose registration needs cleanup.
pub(crate) enum RegisteredWaiter {
    /// Blocking waiter registration.
    Blocking(u64),
    /// Async waiter registration.
    Async(u64),
}
