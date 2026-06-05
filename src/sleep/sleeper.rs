// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::time::Duration;

/// Provides blocking relative sleep operations.
pub trait Sleeper: Send + Sync {
    /// Blocks the current thread for the specified duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The relative duration to sleep.
    fn sleep_for(&self, duration: Duration);
}
