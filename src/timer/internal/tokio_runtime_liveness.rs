// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shares runtime-shutdown notification across Tokio timer futures.

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
};
use tokio::{
    sync::watch,
    task::AbortHandle,
};

/// Runtime-liveness sentinel shared by every pending future of one timer.
#[derive(Debug)]
pub(crate) struct TokioRuntimeLiveness {
    /// Receiver closed when the retained runtime drops the sentinel task.
    shutdown: watch::Receiver<()>,
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
        let (shutdown_sender, shutdown) = watch::channel(());
        let sentinel = tokio::spawn(async move {
            let _shutdown_sender = shutdown_sender;
            std::future::pending::<()>().await;
        });
        let sentinel = sentinel.abort_handle();
        Self { shutdown, sentinel }
    }

    /// Creates a future that completes when the retained runtime shuts down.
    ///
    /// The future retains this state so dropping the originating timer cannot
    /// abort the sentinel while a deadline still needs shutdown detection.
    ///
    /// # Parameters
    ///
    /// * `liveness` - Shared liveness state to retain and observe.
    ///
    /// # Returns
    ///
    /// A future that remains pending until the sentinel sender is dropped.
    #[must_use]
    pub(crate) fn shutdown_future(
        liveness: &Arc<Self>,
    ) -> Pin<Box<impl Future<Output = ()> + Send + 'static>> {
        let mut shutdown = liveness.shutdown.clone();
        let retained_liveness = Arc::clone(liveness);
        Box::pin(async move {
            let _retained_liveness = retained_liveness;
            let _ = shutdown.changed().await;
        })
    }
}

impl Drop for TokioRuntimeLiveness {
    /// Aborts the sentinel after the timer and all its futures release it.
    fn drop(&mut self) {
        self.sentinel.abort();
    }
}
