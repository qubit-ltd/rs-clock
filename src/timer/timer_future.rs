// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Defines the future returned by timer registrations.

use std::future::Future;
use std::pin::Pin;

/// An owned future that becomes ready when a timer reaches its deadline.
///
/// A timer registration is complete before this future is returned. Dropping
/// the future cancels that registration when it has not yet completed.
pub type TimerFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
