// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Side effects collected while committing one manual time advance.

use std::task::Waker;

/// Side effects collected while committing one manual time advance.
#[must_use = "collected advance effects must be delivered after unlocking"]
pub(crate) struct AdvanceEffects {
    /// Task wakers whose deadlines were reached by the advance.
    pub(crate) due_wakers: Vec<Waker>,
}
