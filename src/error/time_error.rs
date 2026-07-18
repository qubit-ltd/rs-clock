// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines errors produced by time-domain operations.

use super::TimerUnavailableReason;
use crate::ClockDomain;
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
///         TimeError::CannotMoveBackward => "backward move",
///         TimeError::InvalidInstantOrder => "invalid order",
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
///         TimeError::CannotMoveBackward => "backward move",
///         TimeError::InvalidInstantOrder => "invalid order",
///         TimeError::TimerUnavailable { .. } => "timer unavailable",
///     }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TimeError {
    /// Two monotonic instants belong to different clock domains.
    #[error(
        "monotonic clock domain mismatch: expected {expected}, actual {actual}"
    )]
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
    #[error("manual monotonic time cannot move backward")]
    CannotMoveBackward,
    /// Duration was requested with an earlier instant after the current one.
    #[error("earlier monotonic instant is later than the current instant")]
    InvalidInstantOrder,
    /// A timer could not register a requested deadline.
    #[error("monotonic timer is unavailable: {reason}")]
    TimerUnavailable {
        /// Resource or backend condition that prevented registration.
        reason: TimerUnavailableReason,
    },
}
