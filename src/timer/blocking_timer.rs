/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use crate::timer::{BlockingSleeper, BlockingWaiter};

/// Marks timer domains that support both blocking sleep and blocking wait.
///
/// This facade carries no methods of its own. Import the underlying capability
/// traits to call their methods:
///
/// * [`BlockingSleeper`] for `sleep_*`.
/// * [`BlockingWaiter`] for `wait_*`.
/// * [`WaitNotifier`](crate::timer::WaitNotifier) for notification.
pub trait BlockingTimer: BlockingSleeper + BlockingWaiter {}

impl<T> BlockingTimer for T where T: BlockingSleeper + BlockingWaiter {}
