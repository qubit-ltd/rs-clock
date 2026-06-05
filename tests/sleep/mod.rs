// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the sleep module.

#[cfg(feature = "tokio")]
mod async_sleeper_tests;
mod mock_sleeper_tests;
mod system_sleeper_tests;
