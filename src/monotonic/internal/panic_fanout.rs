// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Attempts every waker and callback before resuming the first panic.

use super::manual_advance_registry::AdvanceCallback;
use std::any::Any;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
    resume_unwind,
};
use std::task::Waker;

/// Panic payload retained while a notification fanout attempts every target.
type PanicPayload = Box<dyn Any + Send + 'static>;

/// Attempts every waker and callback before resuming the first panic.
pub(crate) struct PanicFanout {
    /// First panic observed in notification order.
    first_panic: Option<PanicPayload>,
}

impl PanicFanout {
    /// Creates a fanout with no retained panic.
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self { first_panic: None }
    }

    /// Attempts every task wake and retains only the first panic payload.
    pub(crate) fn wake_all(&mut self, wakers: Vec<Waker>) {
        for waker in wakers {
            // Borrowing for the wake keeps the waker's destructor out of the
            // wake panic's unwind path, preventing a double-panic abort.
            self.record(catch_unwind(AssertUnwindSafe(|| waker.wake_by_ref())));
            self.record(catch_unwind(AssertUnwindSafe(|| drop(waker))));
        }
    }

    /// Attempts every advance callback and retains only the first panic
    /// payload across both waker and callback phases.
    pub(crate) fn call_all(&mut self, callbacks: Vec<AdvanceCallback>) {
        for callback in callbacks {
            self.record(catch_unwind(AssertUnwindSafe(|| callback())));
            self.record(catch_unwind(AssertUnwindSafe(|| drop(callback))));
        }
    }

    /// Resumes the first retained panic after every target was attempted.
    #[inline]
    pub(crate) fn resume_first_panic(self) {
        if let Some(payload) = self.first_panic {
            resume_unwind(payload);
        }
    }

    /// Records `result` when it is the first panic in this fanout.
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
