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

/// Error returned when timer-domain operations cannot be completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerError {
    /// The operation received an instant from another timer domain.
    TimerDomainMismatch {
        /// The timer domain ID required by the operation.
        expected_domain_id: u64,
        /// The timer domain ID carried by the provided instant.
        actual_domain_id: u64,
    },
}

impl TimerError {
    /// Creates a timer-domain mismatch error.
    ///
    /// # Arguments
    ///
    /// * `expected_domain_id` - The timer domain ID required by the operation.
    /// * `actual_domain_id` - The timer domain ID carried by the provided instant.
    ///
    /// # Returns
    ///
    /// A [`TimerError::TimerDomainMismatch`] value containing both domains.
    pub(crate) fn timer_domain_mismatch(expected_domain_id: u64, actual_domain_id: u64) -> Self {
        Self::TimerDomainMismatch {
            expected_domain_id,
            actual_domain_id,
        }
    }
}

impl fmt::Display for TimerError {
    /// Formats the error with enough domain context for diagnostics.
    ///
    /// # Arguments
    ///
    /// * `formatter` - The destination formatter.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or the formatter's error otherwise.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimerDomainMismatch {
                expected_domain_id,
                actual_domain_id,
            } => write!(
                formatter,
                "timer domain mismatch: expected domain {expected_domain_id}, got domain {actual_domain_id}",
            ),
        }
    }
}

impl Error for TimerError {}
