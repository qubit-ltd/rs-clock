// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines errors produced by time-domain operations.

use std::error::Error;
use std::fmt::{
    Display,
    Formatter,
};

/// Describes an invalid monotonic-time operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    /// Two monotonic instants belong to different clock domains.
    ClockDomainMismatch {
        /// Domain required by the receiving clock or instant.
        expected: u64,
        /// Domain carried by the supplied instant.
        actual: u64,
    },
    /// A monotonic instant cannot represent the requested result.
    InstantOverflow,
    /// A manual monotonic clock was asked to move backward.
    CannotMoveBackward,
    /// Duration was requested with an earlier instant after the current one.
    InvalidInstantOrder,
}

impl Display for TimeError {
    /// Formats a stable, human-readable error message.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClockDomainMismatch { expected, actual } => write!(
                formatter,
                "monotonic clock domain mismatch: expected {expected}, actual {actual}",
            ),
            Self::InstantOverflow => {
                formatter.write_str("monotonic instant overflow")
            }
            Self::CannotMoveBackward => formatter
                .write_str("manual monotonic time cannot move backward"),
            Self::InvalidInstantOrder => formatter.write_str(
                "earlier monotonic instant is later than the current instant",
            ),
        }
    }
}

impl Error for TimeError {}
