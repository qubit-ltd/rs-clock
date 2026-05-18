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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockTimeError {
    /// The operation would reset a timeline while waiters are still registered.
    ActiveWaiters,
}

impl Display for MockTimeError {
    /// Formats the mock time error.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ActiveWaiters => write!(f, "mock timeline has active waiters"),
        }
    }
}

impl Error for MockTimeError {}
