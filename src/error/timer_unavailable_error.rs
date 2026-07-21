// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines errors reported when a timer backend is unavailable.

use std::{
    error::Error as StdError,
    io,
};

use thiserror::Error;

/// Describes a backend failure that prevented timer registration or completion.
///
/// Each variant preserves the most specific stable source exposed by its
/// backend. Custom [`Timer`](crate::Timer) implementations should use
/// [`BackendUnavailable`](Self::BackendUnavailable) to retain their own error
/// rather than reducing it to display text. The enum is non-exhaustive; callers
/// must retain a fallback arm when matching it.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TimerUnavailableError {
    /// The standard timer could not spawn its shared scheduler worker.
    #[error("the scheduler worker thread could not be spawned: {source}")]
    WorkerThreadSpawnFailed {
        /// I/O error returned by the native thread builder.
        #[source]
        source: io::Error,
    },
    /// The standard timer scheduler worker exited before the deadline.
    #[error("the scheduler worker thread terminated unexpectedly")]
    SchedulerWorkerTerminated,
    /// The target asynchronous runtime has no enabled time driver.
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    #[error("the asynchronous runtime time driver is disabled")]
    TimeDriverDisabled,
    /// The target asynchronous runtime shut down before a pending timer future
    /// completed.
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    #[error(
        "the asynchronous runtime shut down before the timer future completed"
    )]
    RuntimeShuttingDown,
    /// A custom timer backend is unavailable.
    #[error("timer backend '{backend}' is unavailable: {source}")]
    BackendUnavailable {
        /// Stable name identifying the custom backend.
        backend: &'static str,
        /// Backend-specific error that prevented timer registration or
        /// completion.
        #[source]
        source: Box<dyn StdError + Send + Sync + 'static>,
    },
}
