// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the future returned by timer registrations.

use std::future::Future;
use std::pin::Pin;

/// An owned future that becomes ready when a timer reaches its deadline.
///
/// The logical deadline and cancellation ownership are fixed before this
/// future is returned. A backend may defer enrollment with its native
/// scheduler until the future is first polled. Dropping an incomplete future
/// cancels its outstanding notification in either case.
pub type TimerFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
