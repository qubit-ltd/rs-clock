// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_clock::{AsyncSleeper, ManualAsyncSleeper, ManualMonotonicClock, SleepFuture};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_sleep_future_is_send() {
    fn assert_send<T: Send>(_: &T) {}

    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let future: SleepFuture = sleeper.sleep_for_async(Duration::ZERO);
    assert_send(&future);
}
