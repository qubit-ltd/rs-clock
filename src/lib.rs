// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Injectable wall clocks, monotonic clocks, and deterministic sleepers.
//!
//! Wall time and monotonic time are deliberately separate. Long-lived
//! services can inject [`WallClock`] for business timestamps, while timeout
//! and delay code injects [`MonotonicClock`], [`BlockingSleeper`], or
//! [`AsyncSleeper`]. Manual implementations allow tests to advance logical
//! time without waiting for real time to pass.

pub mod error;
pub mod monotonic;
pub mod sleep;
pub mod wall;

pub use error::TimeError;
pub use monotonic::{
    ManualAdvanceSubscription,
    ManualMonotonicClock,
    ManualWaiterFuture,
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
    allocate_clock_domain_id,
};
pub use sleep::{
    AsyncSleeper,
    BlockingSleeper,
    ManualAsyncSleeper,
    ManualBlockingSleeper,
    SleepFuture,
    StdBlockingSleeper,
};
pub use wall::{
    FixedWallClock,
    ManualWallClock,
    StdWallClock,
    WallClock,
};

#[cfg(feature = "tokio")]
pub use monotonic::TokioMonotonicClock;
#[cfg(feature = "tokio")]
pub use sleep::TokioAsyncSleeper;
