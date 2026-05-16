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
use std::sync::{
    Arc,
    Condvar,
    Mutex,
};
use std::time::Instant;

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
    wait_state: Arc<(Mutex<u64>, Condvar)>,
    #[cfg(feature = "tokio")]
    async_notifier: Arc<Notify>,
}

impl SystemTimer {
    /// Creates a new system timer with its own timer domain.
    pub fn new() -> Self {
        Self {
            domain: TimerDomainId::new_unique(),
            origin: Instant::now(),
            wait_state: Arc::new((Mutex::new(0), Condvar::new())),
            #[cfg(feature = "tokio")]
            async_notifier: Arc::new(Notify::new()),
        }
    }

    /// Advances the notification generation and wakes blocking waiters.
    fn wake_blocking_waiters(&self) {
        let (generation_lock, condition) = self.wait_state.as_ref();
        let mut generation = generation_lock
            .lock()
            .expect("system timer notification state should not be poisoned");
        *generation = generation.wrapping_add(1);
        drop(generation);
        condition.notify_all();
    }
}

impl Default for SystemTimer {
    /// Creates a new system timer.
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicTimer for SystemTimer {
    /// Returns the timer domain owned by this system timer.
    fn timer_domain(&self) -> TimerDomainId {
        self.domain
    }

    /// Returns the current monotonic instant in this timer's domain.
    fn now(&self) -> TimerInstant {
        TimerInstant::new(self.domain, self.origin.elapsed())
    }
}

impl BlockingTimer for SystemTimer {
    /// Blocks until the deadline is reached or waiters are notified.
    fn wait_until(&self, deadline: TimerInstant) -> Result<TimerWaitOutcome, TimerError> {
        deadline.ensure_domain(self.domain)?;
        let deadline_elapsed = deadline.elapsed_since_timer_start();
        let (generation_lock, condition) = self.wait_state.as_ref();
        let mut generation = generation_lock
            .lock()
            .expect("system timer notification state should not be poisoned");

        loop {
            let now_elapsed = self.origin.elapsed();
            if now_elapsed >= deadline_elapsed {
                return Ok(TimerWaitOutcome::DeadlineReached);
            }

            let remaining = deadline_elapsed - now_elapsed;
            let observed_generation = *generation;
            let (next_generation, wait_result) = condition
                .wait_timeout(generation, remaining)
                .expect("system timer notification state should not be poisoned");
            generation = next_generation;

            if *generation != observed_generation {
                return Ok(TimerWaitOutcome::Notified);
            }
            if wait_result.timed_out() {
                continue;
            }
        }
    }

    /// Wakes current waiters without changing the timer's monotonic time.
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
