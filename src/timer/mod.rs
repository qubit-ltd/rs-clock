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
mod async_timer;
mod blocking_timer;
mod mock_timer;
mod monotonic_timer;
mod system_timer;
mod timer_domain_id;
mod timer_error;
mod timer_instant;
mod timer_wait_outcome;

#[cfg(feature = "tokio")]
pub use async_timer::AsyncTimer;
pub use blocking_timer::BlockingTimer;
pub use mock_timer::MockTimer;
pub use monotonic_timer::MonotonicTimer;
pub use system_timer::SystemTimer;
pub use timer_domain_id::TimerDomainId;
pub use timer_error::TimerError;
pub use timer_instant::TimerInstant;
pub use timer_wait_outcome::TimerWaitOutcome;
