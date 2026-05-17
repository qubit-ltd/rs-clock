/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::timer::{AsyncSleeper, AsyncWaiter};

/// Marks timer domains that support both asynchronous sleep and asynchronous wait.
///
/// This facade carries no methods of its own. Import the underlying capability
/// traits to call their methods:
///
/// * [`AsyncSleeper`] for `sleep_*_async`.
/// * [`AsyncWaiter`] for `wait_*_async`.
/// * [`WaitNotifier`](crate::timer::WaitNotifier) for notification.
pub trait AsyncTimer: AsyncSleeper + AsyncWaiter {}

impl<T> AsyncTimer for T where T: AsyncSleeper + AsyncWaiter {}
