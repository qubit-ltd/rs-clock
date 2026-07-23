// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Retains one liveness sentinel for a Tokio runtime.

use crate::timer::internal::tokio_runtime_shutdown_guard::TokioRuntimeShutdownGuard;
use crate::timer::internal::tokio_runtime_shutdown_state::TokioRuntimeShutdownState;
use std::sync::Arc;
use tokio::{
    sync::futures::OwnedNotified,
    task::AbortHandle,
};

/// Runtime-liveness sentinel shared by timers retaining one runtime.
#[derive(Debug)]
pub(crate) struct TokioRuntimeLiveness {
    /// State signaled when the retained runtime drops the sentinel task.
    shutdown: Arc<TokioRuntimeShutdownState>,
    /// Handle used to release the sentinel after its final consumer is gone.
    sentinel: AbortHandle,
}

impl TokioRuntimeLiveness {
    /// Spawns one task whose cancellation publishes runtime shutdown.
    ///
    /// # Returns
    ///
    /// Shared liveness state for futures registered on the entered runtime.
    #[must_use]
    pub(crate) fn new() -> Self {
        let shutdown = Arc::new(TokioRuntimeShutdownState::new());
        let shutdown_guard =
            TokioRuntimeShutdownGuard::new(Arc::clone(&shutdown));
        let sentinel = tokio::spawn(async move {
            let _shutdown_guard = shutdown_guard;
            std::future::pending::<()>().await;
        });
        let sentinel = sentinel.abort_handle();
        Self { shutdown, sentinel }
    }

    /// Reports whether the sentinel's runtime has shut down.
    ///
    /// # Returns
    ///
    /// `true` after the runtime closes the shutdown channel.
    #[must_use]
    #[inline]
    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.is_shutdown()
    }

    /// Creates an owned notification for retained-runtime shutdown.
    ///
    /// # Returns
    ///
    /// A future that becomes ready after the sentinel guard publishes shutdown.
    pub(crate) fn shutdown_notification(&self) -> OwnedNotified {
        self.shutdown.notification()
    }
}

impl Drop for TokioRuntimeLiveness {
    /// Aborts the sentinel after the timer and all its futures release it.
    fn drop(&mut self) {
        self.sentinel.abort();
    }
}
