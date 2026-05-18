/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Errors returned by mock time controls.

use std::error::Error;
use std::fmt::{
    Display,
    Formatter,
};

/// Error returned when a mock time control operation is rejected.
///
/// Mock time controls are usually infallible because tests advance time
/// explicitly. Resetting elapsed time while a sleeper or deadline waiter is
/// active would rewind the timeline underneath a blocked operation. Passing an
/// instant from one timeline into another timeline would make a deadline refer
/// to the wrong logical time source. Both cases are rejected instead of
/// silently producing inconsistent deadline behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockTimeError {
    /// The operation would reset a timeline while waiters are still registered.
    ActiveWaiters,
    /// The operation received an instant created by a different timeline.
    MismatchedTimeline {
        /// Timeline id expected by the operation.
        expected: u64,
        /// Timeline id carried by the provided instant.
        actual: u64,
    },
}

impl Display for MockTimeError {
    /// Formats the mock time error.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ActiveWaiters => write!(f, "mock timeline has active waiters"),
            Self::MismatchedTimeline { expected, actual } => write!(
                f,
                "mock instant belongs to timeline {actual}, but timeline {expected} was expected",
            ),
        }
    }
}

impl Error for MockTimeError {}
