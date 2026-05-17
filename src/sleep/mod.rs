/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Thread-blocking and asynchronous sleep abstractions.
//!
//! The sleep module intentionally models only relative sleeps. It does not
//! expose deadlines, notifications, or condition waits; those belong to
//! synchronization primitives such as monitors.
//! [`SystemSleeper`] and [`MockSleeper`] implement [`Sleeper`], and also
//! implement `AsyncSleeper` when the `tokio` feature is enabled.

#[cfg(feature = "tokio")]
mod async_sleep_future;
#[cfg(feature = "tokio")]
mod async_sleeper;
mod mock_sleeper;
mod sleeper;
mod system_sleeper;

#[cfg(feature = "tokio")]
pub use async_sleep_future::AsyncSleepFuture;
#[cfg(feature = "tokio")]
pub use async_sleeper::AsyncSleeper;
pub use mock_sleeper::MockSleeper;
pub use sleeper::Sleeper;
pub use system_sleeper::SystemSleeper;
