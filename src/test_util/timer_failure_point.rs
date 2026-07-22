// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the Timer lifecycle point where a test failure is injected.

/// Selects which stage of a Timer operation reports the configured failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TimerFailurePoint {
    /// `Timer::at` rejects the deadline without returning a future.
    Registration,
    /// `Timer::at` succeeds and its returned future reports the failure.
    Completion,
}
