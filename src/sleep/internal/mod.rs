// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal synchronization helpers for blocking sleep.

pub(crate) mod notification_latch;
mod thread_waker;

pub(super) use thread_waker::ThreadWaker;
