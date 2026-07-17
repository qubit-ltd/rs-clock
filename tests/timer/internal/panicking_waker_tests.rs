// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a task waker that panics whenever a notification is delivered.

use std::sync::Arc;
use std::task::Wake;

/// Panic payload whose destructor also panics.
struct DestructorPanickingPayload;

impl Drop for DestructorPanickingPayload {
    /// Panics to exercise nested panic isolation.
    fn drop(&mut self) {
        panic!("intentional Timer panic-payload destructor panic");
    }
}

/// Panics when a Timer attempts to notify its registered task.
pub(crate) struct PanickingWaker;

impl Wake for PanickingWaker {
    /// Panics to exercise Timer notification isolation.
    ///
    /// # Panics
    ///
    /// Always panics with a stable test message.
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Panics to exercise Timer notification isolation.
    ///
    /// # Panics
    ///
    /// Always panics with a stable test message.
    fn wake_by_ref(self: &Arc<Self>) {
        panic!("intentional Timer waker panic");
    }
}

/// Panics with a payload whose destructor also panics.
pub(crate) struct DestructorPanickingWaker;

impl Wake for DestructorPanickingWaker {
    /// Panics with a destructor-panicking payload.
    ///
    /// # Panics
    ///
    /// Always panics with [`DestructorPanickingPayload`].
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Panics with a destructor-panicking payload.
    ///
    /// # Panics
    ///
    /// Always panics with [`DestructorPanickingPayload`].
    fn wake_by_ref(self: &Arc<Self>) {
        std::panic::panic_any(DestructorPanickingPayload);
    }
}
