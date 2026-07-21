// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_clock::TimerFuture;
use std::future;

#[test]
fn test_timer_future_is_send_and_static() {
    fn assert_send_static<T: Send + 'static>(_: T) {}

    let future: TimerFuture = Box::pin(future::pending());
    assert_send_static(future);
}
