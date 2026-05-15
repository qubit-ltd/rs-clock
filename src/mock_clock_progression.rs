/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Progression mode for controllable mock clocks.

/// Controls whether a mock clock stays frozen or progresses with monotonic time.
///
/// [`Frozen`](MockClockProgression::Frozen) is the default because deterministic
/// tests usually need the clock to change only when explicitly advanced.
/// [`Monotonic`](MockClockProgression::Monotonic) keeps the current logical
/// reading anchored to an internal monotonic time source, so subsequent reads
/// naturally progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MockClockProgression {
    /// Keep the logical time frozen until explicitly advanced.
    #[default]
    Frozen,
    /// Progress the logical time using an internal monotonic time source.
    Monotonic,
}

impl MockClockProgression {
    /// Returns `true` when this mode uses monotonic progression.
    #[inline]
    pub const fn is_monotonic(self) -> bool {
        matches!(self, Self::Monotonic)
    }
}
