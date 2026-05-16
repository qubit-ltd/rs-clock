/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the timer module.

#[cfg(feature = "tokio")]
mod async_timer_tests;
mod blocking_timer_tests;
mod mock_timer_tests;
mod monotonic_timer_tests;
mod system_timer_tests;
mod timer_domain_id_tests;
mod timer_error_tests;
mod timer_instant_tests;
mod timer_wait_outcome_tests;
