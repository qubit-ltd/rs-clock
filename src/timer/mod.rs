/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Monotonic timer domains, deadlines, and mockable waiting.
//!
//! A timer domain owns a monotonic time axis whose zero point is the moment that
//! timer domain was created. [`TimerInstant`] values are therefore only
//! meaningful inside their original domain. APIs in this module reject foreign
//! instants instead of comparing unrelated elapsed durations.

#[cfg(feature = "tokio")]
mod async_sleeper;
#[cfg(feature = "tokio")]
mod async_timer;
#[cfg(feature = "tokio")]
mod async_timer_result;
#[cfg(feature = "tokio")]
mod async_waiter;
mod blocking_sleeper;
mod blocking_timer;
mod blocking_waiter;
mod mock_timer;
mod system_timer;
mod timer_domain;
mod timer_error;
mod timer_instant;
mod timer_result;
mod timer_wait_outcome;
mod wait_notifier;

#[cfg(feature = "tokio")]
pub use async_sleeper::AsyncSleeper;
#[cfg(feature = "tokio")]
pub use async_timer::AsyncTimer;
#[cfg(feature = "tokio")]
pub use async_timer_result::AsyncTimerResult;
#[cfg(feature = "tokio")]
pub use async_waiter::AsyncWaiter;
pub use blocking_sleeper::BlockingSleeper;
pub use blocking_timer::BlockingTimer;
pub use blocking_waiter::BlockingWaiter;
pub use mock_timer::MockTimer;
pub use system_timer::SystemTimer;
pub use timer_domain::TimerDomain;
pub(crate) use timer_domain::next_timer_domain_id;
pub use timer_error::TimerError;
pub use timer_instant::TimerInstant;
pub use timer_result::TimerResult;
pub use timer_wait_outcome::TimerWaitOutcome;
pub use wait_notifier::WaitNotifier;
