// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the sleep module.

mod async_sleeper_tests;
mod blocking_sleeper_tests;
mod manual_async_sleeper_tests;
mod manual_blocking_sleeper_tests;
mod manual_sleep_future_tests;
mod sleep_future_tests;
mod std_blocking_sleeper_tests;
#[cfg(feature = "tokio")]
mod tokio_async_sleeper_tests;
