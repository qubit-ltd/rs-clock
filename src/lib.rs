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

pub use error::TimeError;
pub use error::TimerUnavailableError;
#[cfg(feature = "tokio")]
pub use error::TokioRuntimeError;
pub use monotonic::ClockDomain;
pub use monotonic::ManualDeadlineFuture;
pub use monotonic::ManualMonotonicClock;
pub use monotonic::ManualWaiterFuture;
pub use monotonic::MonotonicClock;
pub use monotonic::MonotonicInstant;
pub use monotonic::StdMonotonicClock;
#[cfg(feature = "tokio")]
pub use monotonic::TokioMonotonicClock;
pub use sleep::BlockingSleeper;
pub use timer::ManualTimer;
pub use timer::StdTimer;
pub use timer::Timer;
pub use timer::TimerFuture;
#[cfg(feature = "tokio")]
pub use timer::TokioTimer;
// qubit-style: allow coverage-cfg
#[doc(hidden)]
#[cfg(coverage)]
pub use timer::internal::std_timer_scheduler::fail_next_std_timer_worker_spawn;
#[doc(hidden)]
#[cfg(coverage)]
pub use timer::internal::std_timer_scheduler::panic_next_std_timer_worker;
#[doc(hidden)]
#[cfg(coverage)]
pub use timer::internal::std_timer_scheduler::reset_std_timer_worker_notification_count;
#[doc(hidden)]
#[cfg(coverage)]
pub use timer::internal::std_timer_scheduler::std_timer_worker_notification_count;
#[doc(hidden)]
#[cfg(all(coverage, feature = "tokio"))]
pub use timer::panic_next_tokio_timer_sleep_poll;
pub use wall::FixedWallClock;
pub use wall::ManualWallClock;
pub use wall::StdWallClock;
pub use wall::WallClock;
