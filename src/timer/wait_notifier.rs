/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::timer::TimerDomain;

/// Broadcasts notifications to waiters registered in a timer domain.
///
/// Notifications are completion signals for `wait_*` operations. They are not
/// completion signals for `sleep_*` operations, which continue until their
/// deadline is reached.
pub trait WaitNotifier: TimerDomain {
    /// Wakes all current waiters without advancing time.
    ///
    /// Wait operations return a notification outcome when their deadline has not
    /// been reached. Sleep operations are not completed by this signal.
    fn notify_all_waiters(&self);
}
