/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Mock timeline waiter categories.

/// Identifies waiter groups tracked by [`crate::MockTimeline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MockWaiterKind {
    /// A waiter entered through [`crate::sleep::Sleeper`].
    Sleep,
    /// A generic deadline waiter entered directly through the mock timeline.
    Deadline,
}
