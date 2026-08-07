// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Error types exposed by this crate.

mod time_error;
mod timer_unavailable_error;

#[cfg(feature = "tokio")]
mod tokio_runtime_error;

pub use time_error::TimeError;
pub use timer_unavailable_error::TimerUnavailableError;
#[cfg(feature = "tokio")]
pub use tokio_runtime_error::TokioRuntimeError;
