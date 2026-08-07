// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Timer capabilities for asynchronous deadline notification.

// qubit-style: allow coverage-cfg

pub(crate) mod internal;
mod manual_timer;
mod std_timer;
#[allow(
    clippy::module_inception,
    reason = "the file name follows the public Timer type"
)]
mod timer;
mod timer_future;

#[cfg(feature = "tokio")]
mod tokio_timer;

pub use manual_timer::ManualTimer;
pub use std_timer::StdTimer;
pub use timer::Timer;
pub use timer_future::TimerFuture;
#[cfg(feature = "tokio")]
pub use tokio_timer::TokioTimer;
#[doc(hidden)]
#[cfg(all(coverage, feature = "tokio"))]
pub use tokio_timer::panic_next_tokio_timer_sleep_poll;
