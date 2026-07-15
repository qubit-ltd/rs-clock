// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Blocking and asynchronous sleep capabilities based on monotonic clocks.

mod async_sleeper;
mod blocking_sleeper;
mod internal;
mod manual_async_sleeper;
mod manual_blocking_sleeper;
mod sleep_future;
mod std_blocking_sleeper;

#[cfg(feature = "tokio")]
mod tokio_async_sleeper;

pub use async_sleeper::AsyncSleeper;
pub use blocking_sleeper::BlockingSleeper;
pub use manual_async_sleeper::ManualAsyncSleeper;
pub use manual_blocking_sleeper::ManualBlockingSleeper;
pub use sleep_future::SleepFuture;
pub use std_blocking_sleeper::StdBlockingSleeper;

#[cfg(feature = "tokio")]
pub use tokio_async_sleeper::TokioAsyncSleeper;
