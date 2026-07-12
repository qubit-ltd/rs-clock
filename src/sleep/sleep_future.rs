// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the object-safe asynchronous sleep future type.

use crate::TimeError;
use std::future::Future;
use std::pin::Pin;

/// A sendable future resolving when a monotonic sleep completes.
pub type SleepFuture =
    Pin<Box<dyn Future<Output = Result<(), TimeError>> + Send + 'static>>;
