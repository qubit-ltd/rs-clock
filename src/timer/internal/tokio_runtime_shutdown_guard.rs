// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Signals shutdown when Tokio drops the runtime-liveness sentinel.

use std::sync::Arc;

use crate::timer::internal::tokio_runtime_shutdown_state::TokioRuntimeShutdownState;

/// Sentinel-owned guard that publishes shutdown from [`Drop`].
#[derive(Debug)]
pub(crate) struct TokioRuntimeShutdownGuard {
    /// State notified when the sentinel task is dropped.
    shutdown: Arc<TokioRuntimeShutdownState>,
}

impl TokioRuntimeShutdownGuard {
    /// Creates a guard for one runtime shutdown state.
    ///
    /// # Parameters
    ///
    /// * `shutdown` - State notified when this guard is dropped.
    ///
    /// # Returns
    ///
    /// A guard retaining the supplied state.
    #[must_use]
    pub(crate) fn new(shutdown: Arc<TokioRuntimeShutdownState>) -> Self {
        Self { shutdown }
    }
}

impl Drop for TokioRuntimeShutdownGuard {
    /// Publishes shutdown when Tokio releases the sentinel task.
    fn drop(&mut self) {
        self.shutdown.signal();
    }
}
