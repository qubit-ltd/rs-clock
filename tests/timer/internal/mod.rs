// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Provides task wakers used by Timer integration tests.

mod panicking_waker;
mod thread_waker;

pub(super) use panicking_waker::PanickingWaker;
pub(super) use thread_waker::block_on_timer_future;
