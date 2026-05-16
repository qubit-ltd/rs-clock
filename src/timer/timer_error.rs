/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::error::Error;
use std::fmt;

use crate::timer::TimerDomainId;

/// Error returned when timer-domain operations cannot be completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerError {
    /// The operation received an instant from another timer domain.
    TimerDomainMismatch {
        /// The timer domain required by the operation.
        expected: TimerDomainId,
        /// The timer domain carried by the provided instant.
        actual: TimerDomainId,
    },
}

impl TimerError {
    /// Creates a timer-domain mismatch error.
    pub(crate) fn timer_domain_mismatch(expected: TimerDomainId, actual: TimerDomainId) -> Self {
        Self::TimerDomainMismatch { expected, actual }
    }
}

impl fmt::Display for TimerError {
    /// Formats the error with enough domain context for diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimerDomainMismatch { expected, actual } => write!(
                formatter,
                "timer domain mismatch: expected domain {expected}, got domain {actual}",
            ),
        }
    }
}

impl Error for TimerError {}
