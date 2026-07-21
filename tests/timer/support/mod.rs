// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Provides support helpers shared by Timer integration tests.

mod timer_future_driver;

pub(super) use timer_future_driver::block_on_timer_future;
