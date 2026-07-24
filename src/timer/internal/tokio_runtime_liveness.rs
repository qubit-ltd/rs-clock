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
use tokio::sync::{futures::OwnedNotified, oneshot};

/// Runtime-liveness sentinel shared by timers retaining one runtime.
#[derive(Debug)]
pub(crate) struct TokioRuntimeLiveness {
    /// State signaled when the retained runtime drops the sentinel task.
    shutdown: Arc<TokioRuntimeShutdownState>,
    /// Sender whose drop releases the sentinel after its final consumer.
    _sentinel_release: oneshot::Sender<()>,
}

impl TokioRuntimeLiveness {
    /// Creates liveness state before its sentinel task is spawned.
    ///
    /// # Returns
    ///
    /// Unstarted shared liveness state and its sentinel release receiver.
    #[must_use]
    pub(crate) fn new() -> (Self, oneshot::Receiver<()>) {
        let (sentinel_release, release_notification) = oneshot::channel();
        let liveness = Self {
            shutdown: Arc::new(TokioRuntimeShutdownState::new()),
            _sentinel_release: sentinel_release,
        };
        (liveness, release_notification)
    }

    /// Spawns the task whose cancellation publishes runtime shutdown.
    ///
    /// The registry publishes this liveness value before calling this method,
    /// allowing synchronous Tokio task hooks to reuse it without recursively
    /// spawning another sentinel.
    ///
    /// # Parameters
    ///
    /// * `release_notification` - Receiver completed when the final liveness
    ///   consumer drops its sender.
    pub(crate) fn start(&self, release_notification: oneshot::Receiver<()>) {
        let shutdown_guard = TokioRuntimeShutdownGuard::new(Arc::clone(&self.shutdown));
        tokio::spawn(async move {
            let _shutdown_guard = shutdown_guard;
            let _ = release_notification.await;
        });
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
