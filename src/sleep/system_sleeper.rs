/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::thread;
use std::time::Duration;

use crate::sleep::Sleeper;
#[cfg(feature = "tokio")]
use crate::sleep::{
    AsyncSleepFuture,
    AsyncSleeper,
};

/// A real elapsed-time sleeper.
///
/// This type implements [`Sleeper`] using [`std::thread::sleep`]. When the
/// `tokio` feature is enabled, it also implements `AsyncSleeper` using
/// Tokio timers.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemSleeper;

impl SystemSleeper {
    /// Creates a new system sleeper.
    ///
    /// # Returns
    ///
    /// A sleeper that waits using real elapsed time.
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl Sleeper for SystemSleeper {
    /// Blocks the current thread using [`std::thread::sleep`].
    fn sleep_for(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

#[cfg(feature = "tokio")]
impl AsyncSleeper for SystemSleeper {
    /// Returns a Tokio sleep future for the requested duration.
    fn async_sleep_for<'a>(&'a self, duration: Duration) -> AsyncSleepFuture<'a> {
        Box::pin(tokio::time::sleep(duration))
    }
}
