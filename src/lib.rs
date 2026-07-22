// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![cfg_attr(docsrs, feature(doc_cfg))]
//! Injectable wall clocks, monotonic clocks, and deterministic timers.
//!
//! Wall time and monotonic time are deliberately separate. Long-lived
//! services can inject [`WallClock`] for business timestamps, while timeout
//! and delay code injects [`MonotonicClock`] or [`Timer`]. A
//! [`BlockingSleeper`] can adapt the same timer when synchronous code must
//! block, provided the timer backend can progress while the calling thread is
//! parked. Manual implementations allow tests to advance logical time without
//! waiting for real time to pass.
//!
//! # Examples
//!
//! A manual timer can drive a blocking sleep without waiting for real time:
//!
//! ```
//! use qubit_clock::{
//!     BlockingSleeper, ManualMonotonicClock, MonotonicClock,
//! };
//! use std::time::Duration;
//!
//! let clock = ManualMonotonicClock::new_shared();
//! let sleeper = BlockingSleeper::new(clock.new_timer());
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

pub(crate) mod internal;

pub mod error;
pub mod monotonic;
pub mod sleep;
#[cfg(feature = "test-util")]
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
pub mod test_util;
pub mod timer;
pub mod wall;

#[cfg(feature = "tokio")]
pub use error::TokioRuntimeError;
pub use error::{
    TimeError,
    TimerUnavailableError,
};
pub use monotonic::{
    ClockDomain,
    ManualDeadlineFuture,
    ManualMonotonicClock,
    ManualWaiterFuture,
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
};
pub use sleep::BlockingSleeper;
pub use timer::{
    ManualTimer,
    StdTimer,
    Timer,
    TimerFuture,
};
pub use wall::{
    FixedWallClock,
    ManualWallClock,
    StdWallClock,
    WallClock,
};

// qubit-style: allow coverage-cfg
#[doc(hidden)]
#[cfg(coverage)]
pub use timer::internal::std_timer_scheduler::{
    fail_next_std_timer_worker_spawn,
    panic_next_std_timer_worker,
    reset_std_timer_worker_notification_count,
    std_timer_worker_notification_count,
};

#[doc(hidden)]
#[cfg(all(coverage, feature = "tokio"))]
pub use timer::panic_next_tokio_timer_sleep_poll;

#[cfg(feature = "tokio")]
pub use monotonic::TokioMonotonicClock;
#[cfg(feature = "tokio")]
pub use timer::TokioTimer;
