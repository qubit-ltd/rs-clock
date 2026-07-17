// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod manual_timer_tests;
mod std_timer_tests;
mod timer_tests;

mod internal;

#[cfg(feature = "tokio")]
mod tokio_timer_tests;
