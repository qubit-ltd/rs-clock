// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines reasons why a timer cannot register a deadline.

use thiserror::Error;

/// Identifies the unavailable resource that prevented timer registration.
///
/// Callers should use [`BackendUnavailable`](Self::BackendUnavailable) for a
/// custom [`Timer`](crate::Timer) implementation that cannot provide a more
/// specific built-in reason.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TimerUnavailableReason {
    /// The standard timer could not spawn its shared scheduler worker.
    #[error("the scheduler worker thread could not be spawned")]
    WorkerThreadSpawnFailed,
    /// No asynchronous runtime was entered when a deadline was registered.
    #[error("no asynchronous runtime is entered")]
    RuntimeNotEntered,
    /// The entered asynchronous runtime has no enabled time driver.
    #[error("the asynchronous runtime time driver is disabled")]
    TimeDriverDisabled,
    /// A custom or otherwise unspecified timer backend is unavailable.
    #[error("the timer backend is unavailable")]
    BackendUnavailable,
}
