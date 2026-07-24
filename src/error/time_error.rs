// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines errors produced by time-domain operations.

use super::TimerUnavailableError;
use crate::ClockDomain;
use std::time::Duration;
use thiserror::Error;

/// Describes an invalid monotonic-time operation.
///
/// Callers must include a wildcard arm when matching this enum so future
/// versions can add errors without breaking downstream code:
///
/// ```
/// use qubit_clock::TimeError;
///
/// fn message(error: TimeError) -> &'static str {
///     match error {
///         TimeError::ClockDomainMismatch { .. } => "domain mismatch",
///         TimeError::InstantOverflow => "overflow",
///         TimeError::CannotMoveBackward { .. } => "backward move",
///         TimeError::InvalidInstantOrder { .. } => "invalid order",
///         TimeError::TimerUnavailable { .. } => "timer unavailable",
///         _ => "other time error",
///     }
/// }
///
/// assert_eq!("overflow", message(TimeError::InstantOverflow));
/// ```
///
/// An exhaustive match is rejected outside this crate because additional
/// variants may be introduced in a compatible release:
///
/// ```compile_fail
/// use qubit_clock::TimeError;
///
/// fn exhaustive_message(error: TimeError) -> &'static str {
///     match error {
///         TimeError::ClockDomainMismatch { .. } => "domain mismatch",
///         TimeError::InstantOverflow => "overflow",
///         TimeError::CannotMoveBackward { .. } => "backward move",
///         TimeError::InvalidInstantOrder { .. } => "invalid order",
///         TimeError::TimerUnavailable { .. } => "timer unavailable",
///     }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TimeError {
    /// Two monotonic instants belong to different clock domains.
    #[error("monotonic clock domain mismatch: expected {expected}, actual {actual}")]
    ClockDomainMismatch {
        /// Domain required by the receiving clock or instant.
        expected: ClockDomain,
        /// Domain carried by the supplied instant.
        actual: ClockDomain,
    },
    /// A monotonic instant cannot represent the requested result.
    #[error("monotonic instant overflow")]
    InstantOverflow,
    /// A manual monotonic clock was asked to move backward.
    #[error(
        "manual monotonic time cannot move backward from {current_elapsed:?} to \
         {requested_elapsed:?}"
    )]
    CannotMoveBackward {
        /// Current elapsed duration in the manual clock domain.
        current_elapsed: Duration,
        /// Earlier elapsed duration requested by the caller.
        requested_elapsed: Duration,
    },
    /// Duration was requested with an earlier instant after the current one.
    #[error(
        "instant at {earlier_elapsed:?} cannot be earlier than current instant \
         at {current_elapsed:?}"
    )]
    InvalidInstantOrder {
        /// Elapsed duration carried by the receiving current instant.
        current_elapsed: Duration,
        /// Elapsed duration carried by the supplied earlier instant.
        earlier_elapsed: Duration,
    },
    /// A timer could not register or complete a requested deadline.
    #[error("monotonic timer is unavailable: {source}")]
    TimerUnavailable {
        /// Backend error that prevented timer registration or completion.
        #[source]
        source: TimerUnavailableError,
    },
}

impl From<TimerUnavailableError> for TimeError {
    /// Wraps a timer-backend failure as a monotonic time error.
    ///
    /// # Parameters
    ///
    /// * `source` - Timer-backend failure to wrap.
    ///
    /// # Returns
    ///
    /// A monotonic time error retaining `source`.
    #[inline(always)]
    fn from(source: TimerUnavailableError) -> Self {
        Self::TimerUnavailable { source }
    }
}
