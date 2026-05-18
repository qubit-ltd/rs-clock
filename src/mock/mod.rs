/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Unified mock time runtime.
//!
//! This module provides the deterministic test-time model used by mock clocks
//! and mock sleepers. The central type is [`MockTimeline`], a shared monotonic
//! elapsed-time source. [`MockClock`] maps that elapsed time onto UTC wall-clock
//! readings, while [`crate::sleep::MockSleeper`] turns relative sleeps into
//! waits on the same timeline. [`MockTime`] is the convenience facade for the
//! common case where one test needs a clock and a sleeper that advance together.
//!
//! All mock components are intentionally driven by explicit calls to
//! [`MockTimeline::advance`] or [`MockTime::advance`]. Real wall-clock time is
//! used only for test-observability guard timeouts, such as
//! [`MockTimeline::wait_for_blocked_waiters`], so tests can fail instead of
//! hanging forever.

mod mock_clock;
mod mock_instant;
mod mock_time;
mod mock_time_error;
mod mock_timeline;
mod mock_waiter_kind;

pub use mock_clock::MockClock;
pub use mock_instant::MockInstant;
pub use mock_time::MockTime;
pub use mock_time_error::MockTimeError;
pub use mock_timeline::MockTimeline;
pub use mock_waiter_kind::MockWaiterKind;
