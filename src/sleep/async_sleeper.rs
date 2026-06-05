// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::time::Duration;

use crate::sleep::AsyncSleepFuture;

/// Provides asynchronous relative sleep operations.
pub trait AsyncSleeper: Send + Sync {
    /// Returns a future that completes after the specified duration.
    ///
    /// The duration is measured from the method call, not from the first poll
    /// of the returned future.
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative duration to sleep.
    ///
    /// # Returns
    ///
    /// A future that resolves after the duration has elapsed.
    fn sleep_for_async<'a>(
        &'a self,
        duration: Duration,
    ) -> AsyncSleepFuture<'a>;
}
