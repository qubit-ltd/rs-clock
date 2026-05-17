/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::future::Future;
use std::pin::Pin;

/// Future returned by asynchronous sleeper operations.
pub type AsyncSleepFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
