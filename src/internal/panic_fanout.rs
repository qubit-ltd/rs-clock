// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Attempts every Waker while retaining at most one panic payload.

use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::panic::resume_unwind;
use std::task::Waker;

/// Panic payload retained while a notification fanout attempts every target.
type PanicPayload = Box<dyn Any + Send + 'static>;

/// Attempts every Waker while retaining at most the first panic payload.
pub(crate) struct PanicFanout {
    /// First panic observed in notification order.
    first_panic: Option<PanicPayload>,
}

impl PanicFanout {
    /// Creates a fanout with no retained panic.
    ///
    /// # Returns
    ///
    /// An empty panic accumulator.
    #[must_use]
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self { first_panic: None }
    }

    /// Attempts every task wake and retains only the first panic payload.
    ///
    /// # Parameters
    ///
    /// * `wakers` - Task Wakers to invoke and destroy in iteration order.
    pub(crate) fn wake_all(&mut self, wakers: Vec<Waker>) {
        for waker in wakers {
            // Borrowing for the wake keeps the Waker's destructor out of the
            // wake panic's unwind path, preventing a double-panic abort.
            self.record(catch_unwind(AssertUnwindSafe(|| waker.wake_by_ref())));
            self.record(catch_unwind(AssertUnwindSafe(|| drop(waker))));
        }
    }

    /// Resumes the first retained panic after every target was attempted.
    ///
    /// # Panics
    ///
    /// Resumes the first retained panic when a prior fanout target panicked.
    #[inline]
    pub(crate) fn resume_first_panic(self) {
        if let Some(payload) = self.first_panic {
            resume_unwind(payload);
        }
    }

    /// Discards the retained panic without allowing its destructor to unwind.
    ///
    /// A detached background notifier has no caller to receive a Waker panic.
    /// The original payload is normally dropped; if that destructor panics,
    /// the secondary payload is deliberately leaked so notification threads
    /// remain alive.
    pub(crate) fn discard_panics(mut self) {
        let Some(payload) = self.first_panic.take() else {
            return;
        };
        if let Err(drop_panic) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
            std::mem::forget(drop_panic);
        }
    }

    /// Records `result` when it is the first panic in this fanout.
    ///
    /// # Parameters
    ///
    /// * `result` - Caught result from a Waker or its destructor.
    #[inline]
    fn record(&mut self, result: Result<(), PanicPayload>) {
        if let Err(payload) = result {
            if self.first_panic.is_none() {
                self.first_panic = Some(payload);
            } else {
                // A panic payload may itself panic when dropped. Leaking only
                // secondary payloads preserves the first panic and guarantees
                // that the remaining notification targets are attempted.
                std::mem::forget(payload);
            }
        }
    }
}
