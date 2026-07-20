// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines errors caused by an unavailable or incompatible Tokio runtime.

use thiserror::Error;
use tokio::runtime::{
    Id,
    TryCurrentError,
};

/// Describes why a Tokio-backed clock cannot use the current runtime context.
///
/// A Tokio clock is permanently bound to the runtime in which it was created.
/// Callers can therefore distinguish a missing runtime context from an
/// independently running, incompatible runtime.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TokioRuntimeError {
    /// No Tokio runtime is entered on the current thread.
    #[error("no Tokio runtime is entered: {source}")]
    NotEntered {
        /// Runtime lookup error reported by Tokio.
        #[source]
        source: TryCurrentError,
    },
    /// The current Tokio runtime differs from the clock's bound runtime.
    #[error("Tokio runtime mismatch: expected {expected}, actual {actual}")]
    Mismatch {
        /// Runtime identity retained by the Tokio clock.
        expected: Id,
        /// Runtime identity entered on the current thread.
        actual: Id,
    },
}
