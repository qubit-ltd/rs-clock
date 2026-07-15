// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Side effects collected while committing one manual time advance.

use super::manual_advance_registry::AdvanceCallback;
use std::task::Waker;

/// Side effects collected while committing one manual time advance.
pub(crate) struct AdvanceEffects {
    /// Task wakers whose deadlines were reached by the advance.
    pub(crate) due_wakers: Vec<Waker>,
    /// Persistent subscriber callbacks captured for this advance.
    pub(crate) advance_callbacks: Vec<AdvanceCallback>,
}
