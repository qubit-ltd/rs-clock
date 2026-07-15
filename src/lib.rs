// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
#![cfg_attr(docsrs, feature(doc_cfg))]
//! Injectable wall clocks, monotonic clocks, and deterministic sleepers.
//!
//! Wall time and monotonic time are deliberately separate. Long-lived
//! services can inject [`WallClock`] for business timestamps, while timeout
//! and delay code injects [`MonotonicClock`], [`BlockingSleeper`], or
//! [`AsyncSleeper`]. Manual implementations allow tests to advance logical
//! time without waiting for real time to pass.
//!
//! # Examples
//!
//! A manual clock can drive a blocking sleep without waiting for real time:
//!
//! ```
//! use qubit_clock::{
//!     BlockingSleeper, ManualBlockingSleeper, ManualMonotonicClock,
//! };
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! let clock = Arc::new(ManualMonotonicClock::new());
//! let sleeper = ManualBlockingSleeper::from_clock(Arc::clone(&clock));
//! let worker = std::thread::spawn(move || {
//!     sleeper
//!         .sleep_for(Duration::from_secs(5))
//!         .expect("manual sleep should complete");
//! });
//!
//! assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
//! clock
//!     .advance(Duration::from_secs(5))
//!     .expect("manual time should advance");
//! worker.join().expect("sleeping thread should finish");
//! ```

pub mod error;
pub mod monotonic;
pub mod sleep;
pub mod wall;

pub use error::TimeError;
pub use monotonic::{
    ClockDomain,
    ManualAdvanceSubscription,
    ManualMonotonicClock,
    ManualWaiterFuture,
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
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
