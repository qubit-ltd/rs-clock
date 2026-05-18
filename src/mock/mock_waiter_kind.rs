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
///
/// Waiter kinds are not part of timekeeping itself. They are a test
/// observability aid used by
/// [`MockTimeline::wait_for_blocked_waiters`](crate::MockTimeline::wait_for_blocked_waiters).
/// A test can wait until a sleeper or deadline waiter is registered before it
/// advances mock time, which avoids races where the test advances the timeline
/// before the worker has actually started waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MockWaiterKind {
    /// A waiter entered through [`crate::sleep::Sleeper`].
    Sleep,
    /// A generic deadline waiter entered directly through the mock timeline.
    Deadline,
}
