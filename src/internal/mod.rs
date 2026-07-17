// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Provides crate-wide internal coordination helpers.

mod panic_fanout;

pub(crate) use panic_fanout::PanicFanout;
