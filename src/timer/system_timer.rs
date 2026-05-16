/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
#[cfg(feature = "tokio")]
use std::future::Future;
#[cfg(feature = "tokio")]
use std::pin::Pin;
#[cfg(feature = "tokio")]
use std::sync::Arc;
use std::time::Instant;

use qubit_lock::ArcMonitor;
#[cfg(feature = "tokio")]
use tokio::sync::Notify;

#[cfg(feature = "tokio")]
use crate::timer::AsyncTimer;
use crate::timer::{
    BlockingTimer,
    MonotonicTimer,
    TimerDomainId,
    TimerError,
    TimerInstant,
    TimerWaitOutcome,
};

/// A real monotonic timer backed by [`std::time::Instant`].
///
/// Clones share the same timer domain and notification state. Deadlines are
/// measured relative to the instant at which the original timer was created.
#[derive(Clone)]
pub struct SystemTimer {
    domain: TimerDomainId,
    origin: Instant,
    wait_generation: ArcMonitor<u64>,
    #[cfg(feature = "tokio")]
    async_notifier: Arc<Notify>,
}

impl SystemTimer {
    /// Creates a new system timer with its own timer domain.
    ///
    /// The domain's zero point is the [`std::time::Instant`] captured at
    /// construction. Clones share the same domain, origin, and notification state.
    ///
    /// # Returns
    ///
    /// A new [`SystemTimer`] backed by the system monotonic clock.
    pub fn new() -> Self {
        Self {
            domain: TimerDomainId::new_unique(),
            origin: Instant::now(),
            wait_generation: ArcMonitor::new(0),
            #[cfg(feature = "tokio")]
            async_notifier: Arc::new(Notify::new()),
        }
    }

    /// Advances the notification generation and wakes blocking waiters.
    ///
    /// Waiters blocked in [`BlockingTimer::wait_until`] observe a generation change
    /// and return [`TimerWaitOutcome::Notified`].
    fn wake_blocking_waiters(&self) {
        self.wait_generation.write_notify_all(|generation| {
            *generation = generation.wrapping_add(1);
        });
    }
}

impl MonotonicTimer for SystemTimer {
    /// Returns the timer domain owned by this system timer.
    ///
    /// # Returns
    ///
    /// The [`TimerDomainId`] assigned when this timer was first created.
    fn timer_domain(&self) -> TimerDomainId {
        self.domain
    }

    /// Returns the current monotonic instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// Elapsed time since the shared origin [`std::time::Instant`], wrapped as a
    /// [`TimerInstant`].
    fn now(&self) -> TimerInstant {
        TimerInstant::new(self.domain, self.origin.elapsed())
    }
}

impl BlockingTimer for SystemTimer {
    /// Blocks until the deadline is reached or waiters are notified.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// The same outcomes as [`BlockingTimer::wait_until`].
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::TimerDomainMismatch`] when `deadline` belongs to
    /// another timer domain.
    fn wait_until(&self, deadline: TimerInstant) -> Result<TimerWaitOutcome, TimerError> {
        deadline.ensure_domain(self.domain)?;
        let deadline_elapsed = deadline.elapsed_since_timer_start();
        let mut generation = self.wait_generation.lock();

        loop {
            let now_elapsed = self.origin.elapsed();
            if now_elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }

            let remaining = deadline_elapsed - now_elapsed;
            let observed_generation = *generation;
            let (next_generation, _status) = generation.wait_timeout(remaining);
            generation = next_generation;

            if *generation != observed_generation {
                return Ok(TimerWaitOutcome::Notified);
            }
        }
    }

    /// Wakes current waiters without changing the timer's monotonic time.
    ///
    /// Blocking waiters and, when the `tokio` feature is enabled, asynchronous
    /// waiters registered on this timer are notified.
    fn notify_waiters(&self) {
        self.wake_blocking_waiters();
        #[cfg(feature = "tokio")]
        self.async_notifier.notify_waiters();
    }
}

#[cfg(feature = "tokio")]
impl AsyncTimer for SystemTimer {
    /// Waits asynchronously until the deadline is reached or waiters are
    /// notified.
    ///
    /// # Arguments
    ///
    /// * `deadline` - The target instant in this timer's domain.
    ///
    /// # Returns
    ///
    /// A future with the same outcomes as [`AsyncTimer::wait_until_async`].
    ///
    /// # Errors
    ///
    /// The future resolves to [`TimerError::TimerDomainMismatch`] when `deadline`
    /// belongs to another timer domain.
    fn wait_until_async<'a>(
        &'a self,
        deadline: TimerInstant,
    ) -> Pin<Box<dyn Future<Output = Result<TimerWaitOutcome, TimerError>> + Send + 'a>> {
        Box::pin(async move {
            deadline.ensure_domain(self.domain)?;
            let deadline_elapsed = deadline.elapsed_since_timer_start();
            let now_elapsed = self.origin.elapsed();
            if now_elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }

            let remaining = deadline_elapsed - now_elapsed;
            let outcome = tokio::select! {
                _ = tokio::time::sleep(remaining) => {
                    TimerWaitOutcome::DeadlineReached
                }
                _ = self.async_notifier.notified() => {
                    TimerWaitOutcome::Notified
                }
            };
            Ok(outcome)
        })
    }
}

impl Default for SystemTimer {
    /// Creates a new system timer.
    ///
    /// # Returns
    ///
    /// A timer equivalent to [`SystemTimer::new`].
    fn default() -> Self {
        Self::new()
    }
}
