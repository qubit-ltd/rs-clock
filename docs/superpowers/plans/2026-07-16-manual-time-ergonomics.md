# Manual Time Ergonomics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add cancellation-safe async future-deadline coordination and concise shared manual-time component construction, then migrate all direct downstream construction sites.

**Architecture:** `Arc<ManualMonotonicClock>` remains the test control-plane handle and gains constructors for same-domain concrete shared components. A new `ManualDeadlineFuture` uses the existing waiter registry with a typed observer state that latches the earliest deadline which was strictly future when observed. Production trait-object injection contracts remain unchanged.

**Tech Stack:** Rust 1.94, edition 2024, standard-library `Future`/`Waker`/`Arc`, Tokio only in existing async tests, repository-provided CI scripts.

## Global Constraints

- Preserve `Arc<dyn WallClock>`, `Arc<dyn BlockingSleeper>`, and `Arc<dyn AsyncSleeper>` as downstream injection types.
- Do not add universal clock-family wrappers, runtime downcasting, dependencies, or automatic wall-time anchors.
- Register async deadline observation when the future is created, not on first poll.
- Invoke and destroy detached wakers only after releasing the manual-clock state lock.
- Put every new Rust type in its own snake-case source file and every test in the mirrored external `tests/` path.
- Run `./align-ci.sh` before `./ci-check.sh` in every affected repository; run `./coverage.sh json` only when CI reports coverage below its threshold.
- Do not execute `git add`, `git commit`, or `git push`; user authorization covers code changes but not Git publication operations.

---

### Task 1: Async future-deadline observer

**Files:**
- Create: `src/monotonic/internal/manual_waiter_observer.rs`
- Create: `src/monotonic/manual_deadline_future.rs`
- Create: `tests/monotonic/manual_deadline_future_tests.rs`
- Modify: `src/monotonic/internal/mod.rs`
- Modify: `src/monotonic/internal/manual_waiter_registry.rs`
- Modify: `src/monotonic/manual_monotonic_clock.rs`
- Modify: `src/monotonic/mod.rs`
- Modify: `src/lib.rs`
- Modify: `tests/support/mod.rs`
- Modify: `tests/monotonic/mod.rs`
- Modify: `tests/monotonic/internal/manual_waiter_registry_tests.rs`

**Interfaces:**
- Consumes: `ManualWaiterRegistry::next_future_deadline`, `ManualMonotonicClock::lock_state`, and existing outside-lock waker destruction conventions.
- Produces: `ManualDeadlineFuture`, `ManualMonotonicClock::wait_for_next_deadline_async(self: &Arc<Self>) -> ManualDeadlineFuture`, and internal deadline-observer register/poll/unregister behavior.

- [ ] **Step 1: Add failing public behavior tests**

Create the test module entry in `tests/monotonic/mod.rs`:

```rust
mod manual_deadline_future_tests;
```

Create `tests/monotonic/manual_deadline_future_tests.rs` with the repository copyright header and these initial tests:

```rust
use qubit_clock::{
    AsyncSleeper,
    ManualAsyncSleeper,
    ManualMonotonicClock,
    MonotonicClock,
};
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

#[test]
fn test_manual_deadline_future_returns_existing_earliest_deadline() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let later = sleeper.sleep_for_async(Duration::from_secs(5));
    let earlier = sleeper.sleep_for_async(Duration::from_secs(2));
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("an existing future deadline should be ready");
    };
    assert_eq!(Duration::from_secs(2), deadline.elapsed_since_origin());
    drop((earlier, later));
}

#[test]
fn test_manual_deadline_future_registers_before_first_poll() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let sleep = sleeper.sleep_for_async(Duration::from_secs(3));
    drop(sleep);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("the registered deadline should be latched");
    };
    assert_eq!(Duration::from_secs(3), deadline.elapsed_since_origin());
}

#[test]
fn test_manual_deadline_future_ignores_reached_waiter() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let reached_sleep = sleeper.sleep_for_async(Duration::from_secs(1));
    clock
        .advance(Duration::from_secs(1))
        .expect("manual time should advance");
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));
    let future_sleep = sleeper.sleep_for_async(Duration::from_secs(2));
    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("the later deadline should satisfy the observer");
    };
    assert_eq!(Duration::from_secs(3), deadline.elapsed_since_origin());
    drop((future_sleep, reached_sleep));
}
```

- [ ] **Step 2: Run the focused tests and confirm the red state**

Run from `rs-clock`:

```bash
cargo test --test monotonic_tests manual_deadline_future -- --nocapture
```

Expected: compilation fails because `wait_for_next_deadline_async` does not yet exist.

- [ ] **Step 3: Add typed internal observer state**

Create `src/monotonic/internal/manual_waiter_observer.rs`:

```rust
use std::task::Waker;
use std::time::Duration;

/// Asynchronous condition observed within one manual waiter registry.
pub(crate) enum ManualWaiterObserver {
    /// Completes after the combined waiter count reaches a threshold.
    Count {
        /// Minimum number of registered waiters.
        expected_count: usize,
        /// Most recently registered task waker.
        waker: Option<Waker>,
    },
    /// Completes after a strictly future waiter deadline is observed.
    Deadline {
        /// Earliest deadline latched while it was strictly future.
        ready_deadline: Option<Duration>,
        /// Most recently registered task waker.
        waker: Option<Waker>,
    },
}
```

Declare it in `src/monotonic/internal/mod.rs`:

```rust
mod manual_waiter_observer;
```

Expose the exact source file to focused state-machine tests in `tests/support/mod.rs` before the registry path module:

```rust
#[allow(dead_code)]
#[path = "../../src/monotonic/internal/manual_waiter_observer.rs"]
pub(crate) mod manual_waiter_observer;
```

Change `ManualWaiterRegistry::observers` to:

```rust
observers: HashMap<u64, ManualWaiterObserver>,
```

Import the enum in the registry:

```rust
use super::manual_waiter_observer::ManualWaiterObserver;
```

Keep count-observer behavior by storing `ManualWaiterObserver::Count`, and add:

```rust
pub(crate) fn register_deadline_observer(
    &mut self,
    elapsed: Duration,
) -> u64 {
    let observer_id = allocate_identifier(
        &mut self.next_observer_id,
        "manual waiter observer identifiers exhausted",
    );
    let ready_deadline = self.next_future_deadline(elapsed);
    self.observers.insert(
        observer_id,
        ManualWaiterObserver::Deadline {
            ready_deadline,
            waker: None,
        },
    );
    observer_id
}

pub(crate) fn poll_deadline_observer(
    &mut self,
    observer_id: u64,
    context: &Context<'_>,
) -> (Poll<Duration>, Option<Waker>) {
    let Some(ManualWaiterObserver::Deadline {
        ready_deadline,
        ..
    }) = self.observers.get(&observer_id)
    else {
        panic!("manual deadline observer {observer_id} is not registered");
    };
    if let Some(deadline) = *ready_deadline {
        let Some(ManualWaiterObserver::Deadline { waker, .. }) =
            self.observers.remove(&observer_id)
        else {
            unreachable!("the matched observer must remain a deadline observer");
        };
        return (Poll::Ready(deadline), waker);
    }
    let Some(ManualWaiterObserver::Deadline { waker, .. }) =
        self.observers.get_mut(&observer_id)
    else {
        unreachable!("the matched observer must remain a deadline observer");
    };
    let replaced_waker = if waker
        .as_ref()
        .is_none_or(|registered| !registered.will_wake(context.waker()))
    {
        waker.replace(context.waker().clone())
    } else {
        None
    };
    (Poll::Pending, replaced_waker)
}
```

Update `reached_observer_wakers` to accept `elapsed: Duration`, remove reached
count observers, and latch rather than remove deadline observers:

```rust
pub(crate) fn reached_observer_wakers(
    &mut self,
    elapsed: Duration,
) -> Vec<Waker> {
    let count = self.count();
    let next_deadline = self.next_future_deadline(elapsed);
    let mut wakers = Vec::new();
    self.observers.retain(|_, observer| match observer {
        ManualWaiterObserver::Count {
            expected_count,
            waker,
        } => {
            if *expected_count <= count {
                if let Some(waker) = waker.take() {
                    wakers.push(waker);
                }
                false
            } else {
                true
            }
        },
        ManualWaiterObserver::Deadline {
            ready_deadline,
            waker,
        } => {
            if ready_deadline.is_none()
                && let Some(deadline) = next_deadline
            {
                *ready_deadline = Some(deadline);
                if let Some(waker) = waker.take() {
                    wakers.push(waker);
                }
            }
            true
        },
    });
    wakers
}
```

Replace the count poll and generic unregister bodies with:

```rust
pub(crate) fn poll_observer(
    &mut self,
    observer_id: u64,
    count: usize,
    context: &Context<'_>,
) -> (Poll<()>, Option<Waker>) {
    let Some(observer) = self.observers.get(&observer_id) else {
        return (Poll::Ready(()), None);
    };
    let ManualWaiterObserver::Count { expected_count, .. } = observer else {
        panic!("manual observer {observer_id} does not track waiter count");
    };
    if count >= *expected_count {
        let Some(ManualWaiterObserver::Count { waker, .. }) =
            self.observers.remove(&observer_id)
        else {
            unreachable!("the matched observer must remain a count observer");
        };
        return (Poll::Ready(()), waker);
    }
    let Some(ManualWaiterObserver::Count { waker, .. }) =
        self.observers.get_mut(&observer_id)
    else {
        unreachable!("the matched observer must remain a count observer");
    };
    let replaced_waker = if waker
        .as_ref()
        .is_none_or(|registered| !registered.will_wake(context.waker()))
    {
        waker.replace(context.waker().clone())
    } else {
        None
    };
    (Poll::Pending, replaced_waker)
}

pub(crate) fn unregister_observer(
    &mut self,
    observer_id: u64,
) -> Option<Waker> {
    match self.observers.remove(&observer_id) {
        Some(ManualWaiterObserver::Count { waker, .. })
        | Some(ManualWaiterObserver::Deadline { waker, .. }) => waker,
        None => None,
    }
}
```

Change existing registry tests to call
`reached_observer_wakers(Duration::ZERO)`.

- [ ] **Step 4: Add the public future and clock adapters**

Create `src/monotonic/manual_deadline_future.rs`:

```rust
use crate::{ManualMonotonicClock, MonotonicInstant};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// A future that observes the next strictly future manual deadline.
#[derive(Debug)]
pub struct ManualDeadlineFuture {
    /// Manual clock that owns the observer registration.
    clock: Arc<ManualMonotonicClock>,
    /// Active observer identifier, or `None` after completion.
    observer_id: Option<u64>,
}

impl ManualDeadlineFuture {
    /// Registers a future-deadline observer before returning.
    #[inline]
    pub(crate) fn new(clock: Arc<ManualMonotonicClock>) -> Self {
        let observer_id = clock.register_deadline_observer();
        Self {
            clock,
            observer_id: Some(observer_id),
        }
    }
}

impl Future for ManualDeadlineFuture {
    type Output = MonotonicInstant;

    /// Polls the latched future deadline and updates the registered waker.
    fn poll(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let observer_id = self
            .observer_id
            .expect("a completed manual deadline future must not be polled");
        let result = self.clock.poll_deadline_observer(observer_id, context);
        if result.is_ready() {
            self.observer_id = None;
        }
        result
    }
}

impl Drop for ManualDeadlineFuture {
    /// Removes an incomplete observer and destroys its waker after unlocking.
    #[inline]
    fn drop(&mut self) {
        if let Some(observer_id) = self.observer_id.take() {
            self.clock.unregister_waiter_observer(observer_id);
        }
    }
}
```

Declare and re-export `ManualDeadlineFuture` from `src/monotonic/mod.rs` and
`src/lib.rs`.

Add these restricted adapters to `ManualMonotonicClock`:

```rust
pub(crate) fn register_deadline_observer(&self) -> u64 {
    let mut state = self.lock_state();
    let elapsed = state.elapsed;
    state.waiters.register_deadline_observer(elapsed)
}

pub(crate) fn poll_deadline_observer(
    &self,
    observer_id: u64,
    context: &Context<'_>,
) -> Poll<MonotonicInstant> {
    let (poll_result, replaced_waker) = {
        let mut state = self.lock_state();
        state.waiters.poll_deadline_observer(observer_id, context)
    };
    drop(replaced_waker);
    poll_result.map(|elapsed| MonotonicInstant::new(self.domain, elapsed))
}
```

Add the public method next to `wait_for_next_deadline`:

```rust
#[must_use]
#[inline(always)]
pub fn wait_for_next_deadline_async(
    self: &Arc<Self>,
) -> ManualDeadlineFuture {
    ManualDeadlineFuture::new(Arc::clone(self))
}
```

In blocking and async waiter registration, copy `state.elapsed` and call
`state.waiters.reached_observer_wakers(elapsed)` so both observer kinds see the
new registration before the state lock is released.

- [ ] **Step 5: Run focused public and internal tests**

Run:

```bash
cargo test --test monotonic_tests manual_deadline_future -- --nocapture
cargo test --test monotonic_tests manual_registry -- --nocapture
```

Expected: both commands exit 0; existing count-observer tests remain green and
the three new public tests pass.

- [ ] **Step 6: Add cancellation and waker lifecycle regression tests**

Add this cancellation test after defining a `WakeCounter` with an
`AtomicUsize` field and a `Wake::wake` implementation that increments it:

```rust
#[test]
fn test_manual_deadline_future_unregisters_on_drop() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);
    let mut observer = Box::pin(clock.wait_for_next_deadline_async());
    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));
    drop(observer);

    let sleep = sleeper.sleep_for_async(Duration::from_secs(1));
    assert_eq!(0, wake_counter.wakes.load(Ordering::SeqCst));
    drop(sleep);
}
```

Define `ReentrantDropWaker` with a `Weak<ManualMonotonicClock>` and a
`SyncSender<()>`, matching the existing external-test pattern. Add
`test_manual_deadline_future_replacement_drops_waker_outside_clock_lock` and
`test_manual_deadline_future_cancellation_drops_waker_outside_clock_lock`;
each must use `recv_timeout(Duration::from_secs(1))` to prove the destructor
re-entered `pending_waiters()` without deadlocking.

Add internal registry tests which verify:

```rust
let observer_id = registry.register_deadline_observer(Duration::ZERO);
let waiter_id = registry.register_async(Duration::from_secs(2));
let _ = registry.reached_observer_wakers(Duration::ZERO);
assert!(registry.unregister_async(waiter_id).is_some());
let context = Context::from_waker(Waker::noop());
let (poll, detached_waker) =
    registry.poll_deadline_observer(observer_id, &context);
assert_eq!(Poll::Ready(Duration::from_secs(2)), poll);
assert!(detached_waker.is_none());
```

Also verify a waiter at `deadline == elapsed` leaves a deadline observer
pending.

- [ ] **Step 7: Re-run all monotonic integration tests**

Run:

```bash
cargo test --test monotonic_tests
```

Expected: exit 0 with no failed tests.

---

### Task 2: Shared manual component constructors

**Files:**
- Modify: `src/monotonic/manual_monotonic_clock.rs`
- Modify: `tests/monotonic/manual_monotonic_clock_tests.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: existing `ManualWallClock::from_clock`, `ManualBlockingSleeper::from_clock`, and `ManualAsyncSleeper::from_clock` constructors.
- Produces: `new_shared`, `new_wall_clock`, `new_blocking_sleeper`, and `new_async_sleeper` on `ManualMonotonicClock`.

- [ ] **Step 1: Add failing construction tests**

Add imports for `WallClock` and `UNIX_EPOCH`, then add:

```rust
#[test]
fn test_manual_monotonic_clock_shared_components_use_one_domain() {
    let clock = ManualMonotonicClock::new_shared();
    let blocking_sleeper = clock.new_blocking_sleeper();
    let async_sleeper = clock.new_async_sleeper();

    assert_eq!(clock.now().domain(), blocking_sleeper.clock().now().domain());
    assert_eq!(clock.now().domain(), async_sleeper.clock().now().domain());
}

#[test]
fn test_manual_monotonic_clock_new_wall_clock_follows_shared_time() {
    let clock = ManualMonotonicClock::new_shared();
    let wall_clock = clock.new_wall_clock(UNIX_EPOCH);
    clock
        .advance(Duration::from_secs(4))
        .expect("manual time should advance");

    assert_eq!(
        UNIX_EPOCH + Duration::from_secs(4),
        wall_clock.now(),
    );
}

#[test]
fn test_manual_monotonic_clock_shared_components_coerce_to_trait_objects() {
    fn accept_wall_clock(_: Arc<dyn WallClock>) {}
    fn accept_blocking_sleeper(_: Arc<dyn BlockingSleeper>) {}
    fn accept_async_sleeper(_: Arc<dyn AsyncSleeper>) {}

    let clock = ManualMonotonicClock::new_shared();
    accept_wall_clock(clock.new_wall_clock(UNIX_EPOCH));
    accept_blocking_sleeper(clock.new_blocking_sleeper());
    accept_async_sleeper(clock.new_async_sleeper());
}
```

- [ ] **Step 2: Run tests and confirm missing methods**

Run:

```bash
cargo test --test monotonic_tests manual_monotonic_clock_shared -- --nocapture
cargo test --test monotonic_tests manual_monotonic_clock_new_wall -- --nocapture
```

Expected: compilation fails because the shared constructors are absent.

- [ ] **Step 3: Implement the shared constructors**

Import `ManualAsyncSleeper`, `ManualBlockingSleeper`, `ManualWallClock`, and
`SystemTime` in `manual_monotonic_clock.rs`, then add immediately after `new`:

```rust
#[must_use]
#[inline(always)]
pub fn new_shared() -> Arc<Self> {
    Arc::new(Self::new())
}

#[must_use]
#[inline(always)]
pub fn new_async_sleeper(
    self: &Arc<Self>,
) -> Arc<ManualAsyncSleeper> {
    Arc::new(ManualAsyncSleeper::from_clock(Arc::clone(self)))
}

#[must_use]
#[inline(always)]
pub fn new_blocking_sleeper(
    self: &Arc<Self>,
) -> Arc<ManualBlockingSleeper> {
    Arc::new(ManualBlockingSleeper::from_clock(Arc::clone(self)))
}

#[must_use]
#[inline(always)]
pub fn new_wall_clock(
    self: &Arc<Self>,
    wall_time: SystemTime,
) -> Arc<ManualWallClock> {
    Arc::new(ManualWallClock::from_clock(
        wall_time,
        Arc::clone(self),
    ))
}
```

Give every method complete Rustdoc covering the shared allocation/domain,
return value, wall-time parameter, allocation behavior, and clock-domain
identifier exhaustion inherited by `new_shared`.

- [ ] **Step 4: Update the crate-level example**

Change `src/lib.rs` from nested `Arc::new`/`from_clock` construction to:

```rust
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_blocking_sleeper();
```

Remove the now-unused `std::sync::Arc` doctest import.

- [ ] **Step 5: Run constructor tests and doctests**

Run:

```bash
cargo test --test monotonic_tests manual_monotonic_clock_shared -- --nocapture
cargo test --test monotonic_tests manual_monotonic_clock_new_wall -- --nocapture
cargo test --doc
```

Expected: all commands exit 0.

---

### Task 3: Validate rs-clock

**Files:**
- Inspect all changes under `rs-clock` after repository scripts format or align them.

**Interfaces:**
- Consumes: Tasks 1 and 2.
- Produces: a validated local `qubit-clock` API for downstream path dependencies.

- [ ] **Step 1: Run repository alignment**

Run from `rs-clock`:

```bash
./align-ci.sh
```

Expected: exit 0. Reinspect `git --no-pager diff` because this script may edit
files.

- [ ] **Step 2: Run CI-equivalent validation**

Run:

```bash
./ci-check.sh
```

Expected: exit 0 across formatting, checks, tests, no-default-features, and
documentation validation.

- [ ] **Step 3: Handle conditional coverage only when requested by CI**

If and only if Step 2 reports coverage below its configured threshold, run:

```bash
./coverage.sh json
```

Add tests only for a reported uncovered business branch in the new observer or
constructor behavior, rerun `./align-ci.sh`, then rerun `./ci-check.sh`.

---

### Task 4: Migrate rs-id manual-time assembly

**Files:**
- Modify: `../rs-id/tests/support/manual_time.rs`

**Interfaces:**
- Consumes: `ManualMonotonicClock::{new_shared,new_wall_clock,new_blocking_sleeper}`.
- Produces: the same local `ManualTime` test fixture behavior with less assembly code.

- [ ] **Step 1: Replace fixture construction**

Replace:

```rust
let monotonic_clock = Arc::new(ManualMonotonicClock::new());
let wall_clock = Arc::new(ManualWallClock::from_clock(
    now,
    Arc::clone(&monotonic_clock),
));
let blocking_sleeper = Arc::new(ManualBlockingSleeper::from_clock(
    Arc::clone(&monotonic_clock),
));
```

with:

```rust
let monotonic_clock = ManualMonotonicClock::new_shared();
let wall_clock = monotonic_clock.new_wall_clock(now);
let blocking_sleeper = monotonic_clock.new_blocking_sleeper();
```

Remove only imports made unused by this replacement. Keep the fixture because
it adds ID-specific reanchor, panic policy, and deadline-driving behavior.

- [ ] **Step 2: Run the focused ID tests**

Run from `rs-id`:

```bash
cargo test --tests
```

Expected: exit 0.

- [ ] **Step 3: Run rs-id repository validation**

Run in order:

```bash
./align-ci.sh
./ci-check.sh
```

Expected: both commands exit 0. Run `./coverage.sh json` only if CI explicitly
reports coverage below threshold.

---

### Task 5: Migrate rs-retry manual-time assembly

**Files:**
- Modify: `../rs-retry/tests/error/retry_error_tests.rs`
- Modify: `../rs-retry/tests/executor/async_retry_runner_tests.rs`
- Modify: `../rs-retry/tests/executor/retry_failure_handler_tests.rs`
- Modify: `../rs-retry/tests/executor/retry_runner_tests.rs`
- Modify: `../rs-retry/tests/executor/worker_retry_runner_tests.rs`

**Interfaces:**
- Consumes: all shared manual constructors and `wait_for_next_deadline_async`.
- Produces: retry tests which retain the manual clock as driver and inject derived sleepers directly.

- [ ] **Step 1: Replace repeated clock and sleeper construction**

Apply these exact semantic replacements at every matching setup while
preserving variable names and test behavior:

```rust
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_blocking_sleeper();
```

and:

```rust
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_async_sleeper();
```

Where a builder expects `Arc<dyn BlockingSleeper>` or `Arc<dyn AsyncSleeper>`,
pass the returned concrete `Arc` directly and rely on the existing coercion.
Keep direct `from_clock` construction only in tests whose subject is that
constructor itself; none of the five listed downstream files has that purpose.

- [ ] **Step 2: Replace multi-stage async count coordination where semantics match**

In tests that currently use `wait_for_waiters_async(1)` solely to discover the
next retry/backoff deadline, replace the coordination with:

```rust
let _ = clock.wait_for_next_deadline_async().await;
```

Do not replace count-based assertions that intentionally verify multiple
simultaneous registrations.

- [ ] **Step 3: Run focused retry tests**

Run from `rs-retry`:

```bash
cargo test --features tokio --test lib_tests
```

Expected: exit 0.

- [ ] **Step 4: Run rs-retry repository validation**

Run in order:

```bash
./align-ci.sh
./ci-check.sh
```

Expected: both commands exit 0. Run `./coverage.sh json` only if CI explicitly
reports coverage below threshold.

---

### Task 6: Migrate rs-lock manual-clock construction

**Files:**
- Modify: `../rs-lock/src/monitor/mock_monitor.rs`
- Modify: `../rs-lock/tests/monitor/mock_monitor_tests.rs`
- Modify: `../rs-lock/tests/monitor/arc_mock_monitor_tests.rs`

**Interfaces:**
- Consumes: `ManualMonotonicClock::new_shared`.
- Produces: unchanged mock-monitor behavior with direct shared-clock construction.

- [ ] **Step 1: Replace shared manual-clock construction**

Replace every:

```rust
Arc::new(ManualMonotonicClock::new())
```

with:

```rust
ManualMonotonicClock::new_shared()
```

Do not alter `subscribe_advances`, monitor notification, or lock-order logic.
Remove only imports made unused by the replacement.

- [ ] **Step 2: Run focused mock-monitor tests**

Run from `rs-lock`:

```bash
cargo test --features mock --test monitor_tests mock_monitor
```

Expected: exit 0.

- [ ] **Step 3: Run rs-lock repository validation**

Run in order:

```bash
./align-ci.sh
./ci-check.sh
```

Expected: both commands exit 0. Run `./coverage.sh json` only if CI explicitly
reports coverage below threshold.

---

### Task 7: Final cross-repository review

**Files:**
- Inspect diffs in `rs-clock`, `rs-id`, `rs-retry`, and `rs-lock` independently.

**Interfaces:**
- Consumes: all prior tasks and validation output.
- Produces: final evidence-backed handoff without commits.

- [ ] **Step 1: Confirm no old downstream assembly remains unintentionally**

Run from `rust-common`:

```bash
rg -n 'Arc::new\(ManualMonotonicClock::new\(\)\)' rs-id rs-retry rs-lock
rg -n 'Manual(?:Async|Blocking)Sleeper::from_clock|ManualWallClock::from_clock' rs-id rs-retry rs-lock
```

Expected: no matches in the migrated downstream implementation/test files;
document any retained constructor-focused test match.

- [ ] **Step 2: Inspect each repository diff and status separately**

Run `git status --short` and `git --no-pager diff` from each of the four
repository roots. Confirm that every changed line maps to the approved design,
that no unrelated user change was modified, and that no generated coverage
artifact is present.

- [ ] **Step 3: Report exact verification evidence**

Report every validation command and exit status per repository, list any
conditional coverage command actually run, identify unresolved risks, and
state explicitly that no Git commit or push was performed.
