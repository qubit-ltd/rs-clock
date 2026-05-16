/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
/// Describes why a timer wait operation returned.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TimerWaitOutcome {
    /// The requested deadline has been reached.
    ///
    /// The timer's monotonic time is at or past the waited-for instant.
    DeadlineReached,
    /// The wait was woken by an explicit notification before the deadline.
    ///
    /// Returned when a blocking or asynchronous wait is interrupted by
    /// [`notify_all_waiters`](crate::timer::BlockingTimer::notify_all_waiters) before the
    /// deadline is reached.
    Notified,
}
