// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shares Tokio runtime liveness across independent timers.

use crate::timer::internal::tokio_runtime_liveness::TokioRuntimeLiveness;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex, Weak},
};
use tokio::runtime::{Handle, Id};

/// Process-wide runtime-liveness registry.
static REGISTRY: LazyLock<TokioRuntimeLivenessRegistry> =
    LazyLock::new(TokioRuntimeLivenessRegistry::default);

/// Weakly indexes one liveness sentinel per running Tokio runtime.
#[derive(Debug, Default)]
pub(crate) struct TokioRuntimeLivenessRegistry {
    /// Runtime identifiers mapped to non-owning liveness references.
    entries: Mutex<HashMap<Id, Weak<TokioRuntimeLiveness>>>,
}

impl TokioRuntimeLivenessRegistry {
    /// Returns shared liveness for the currently entered Tokio runtime.
    ///
    /// # Returns
    ///
    /// Existing live state for the current runtime, or newly spawned state.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime is entered.
    #[must_use]
    pub(crate) fn current() -> Arc<TokioRuntimeLiveness> {
        REGISTRY.get_or_create(Handle::current().id())
    }

    /// Returns live state for one runtime identifier.
    ///
    /// Closed state is never reused because Tokio may recycle identifiers after
    /// a runtime completes.
    ///
    /// # Parameters
    ///
    /// * `runtime_id` - Identifier of the currently entered runtime.
    ///
    /// # Returns
    ///
    /// Shared live state for `runtime_id`.
    fn get_or_create(&self, runtime_id: Id) -> Arc<TokioRuntimeLiveness> {
        let (liveness, release_notification) = {
            let mut entries = self
                .entries
                .lock()
                .expect("Tokio runtime-liveness registry lock should not be poisoned");
            entries.retain(|_, liveness| liveness.strong_count() != 0);
            if let Some(liveness) = entries.get(&runtime_id).and_then(Weak::upgrade)
                && !liveness.is_shutdown()
            {
                return liveness;
            }
            let (liveness, release_notification) = TokioRuntimeLiveness::new();
            let liveness = Arc::new(liveness);
            entries.insert(runtime_id, Arc::downgrade(&liveness));
            (liveness, release_notification)
        };
        liveness.start(release_notification);
        liveness
    }
}
