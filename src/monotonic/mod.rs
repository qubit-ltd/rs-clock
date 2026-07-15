// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Monotonic clocks and domain-scoped instants.

mod clock_domain;
mod internal;
mod manual_advance_subscription;
mod manual_monotonic_clock;
mod manual_waiter_future;
mod monotonic_clock;
mod monotonic_instant;
mod std_monotonic_clock;

#[cfg(feature = "tokio")]
mod tokio_monotonic_clock;

pub use clock_domain::ClockDomain;
pub use manual_advance_subscription::ManualAdvanceSubscription;
pub use manual_monotonic_clock::ManualMonotonicClock;
pub use manual_waiter_future::ManualWaiterFuture;
pub use monotonic_clock::MonotonicClock;
pub use monotonic_instant::MonotonicInstant;
pub use std_monotonic_clock::StdMonotonicClock;

#[cfg(feature = "tokio")]
pub use tokio_monotonic_clock::TokioMonotonicClock;
