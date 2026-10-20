// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines errors caused by an unavailable ambient Tokio runtime.

use thiserror::Error;
use tokio::runtime::TryCurrentError;

/// Describes why a Tokio-backed type cannot capture the current runtime.
///
/// This error occurs only in `try_current` constructors. Once constructed, a
/// Tokio clock or timer uses its retained runtime handle and does not depend on
/// the caller's ambient runtime context. The enum is non-exhaustive; callers
/// must retain a fallback arm when matching it.
///
/// # Examples
///
/// ```
/// use qubit_clock::{TokioRuntimeError, TokioTimer};
///
/// match TokioTimer::try_current() {
///     Err(TokioRuntimeError::NotEntered { source }) => {
///         eprintln!("Tokio runtime is unavailable: {source}");
///     }
///     Err(error) => eprintln!("could not capture Tokio runtime: {error}"),
///     Ok(_timer) => {}
/// }
/// ```
#[non_exhaustive]
#[must_use]
#[derive(Debug, Error)]
pub enum TokioRuntimeError {
    /// No Tokio runtime is entered on the current thread.
    #[error("no Tokio runtime is entered: {source}")]
    NotEntered {
        /// Runtime lookup error reported by Tokio.
        #[source]
        source: TryCurrentError,
    },
}
