# rs-clock Design Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans when executing this plan in a separate session. The current session executes it inline after explicit user approval.

**Goal:** Tighten clock-domain allocation, make manual async wake fanout one-shot, and document the Tokio/manual driver contracts without changing the public sleeper model.

**Architecture:** Keep the existing single-mutex manual clock and public APIs. Represent committed advance side effects with a private named value, move due wakers out of the registry before unlocking, and keep persistent advance callbacks as snapshots. Treat Tokio runtime/time-driver affinity as a documented caller contract because Tokio exposes no stable time-driver identity for this crate to validate.

**Tech Stack:** Rust 2024, Rust 1.94, standard-library atomics and synchronization, optional Tokio 1.52.

## Global Constraints

- Modify only `rs-clock`; do not change downstream crates.
- Do not add dependencies or public driver abstractions.
- Keep `ManualWaiterFuture` public.
- Keep all manual elapsed time, waiters, observers, and subscribers under one mutex.
- `ClockDomain` must not implement `Default`; identifier `u64::MAX` is valid, and only the following allocation panics.
- Do not run `git add`, `git commit`, or `git push` without explicit user approval.

---

### Task 1: Clock-domain allocation boundary

**Files:**

- Modify: `tests/monotonic/clock_domain_tests.rs`
- Modify: `src/monotonic/clock_domain.rs`

**Interfaces:**

- Consumes: the process-wide `AtomicU64` allocator.
- Produces: explicit-only `ClockDomain::new()` and a private allocator helper whose zero state means exhausted.

- [x] **Step 1: Write failing tests**

Use the project's established source-contract test style to verify both
boundary transitions without exposing allocator internals:

```rust
#[test]
fn test_clock_domain_allocator_exhausts_after_maximum_identifier() {
    assert!(
        CLOCK_DOMAIN_SOURCE.contains("u64::MAX => Some(0)"),
        "the maximum identifier must be returned while marking exhaustion",
    );
    assert!(
        CLOCK_DOMAIN_SOURCE.contains("0 => None"),
        "the exhausted allocator state must reject later allocations",
    );
    assert!(
        CLOCK_DOMAIN_SOURCE.contains(
            ".expect(\"monotonic clock domain identifiers exhausted\")",
        ),
        "a rejected allocation must panic with the exhaustion diagnostic",
    );
}
```

Replace the old `ClockDomain::default()` test with this source assertion:

```rust
const CLOCK_DOMAIN_SOURCE: &str =
    include_str!("../../src/monotonic/clock_domain.rs");

#[test]
fn test_clock_domain_does_not_implement_default() {
    assert!(
        !CLOCK_DOMAIN_SOURCE.contains("impl Default for ClockDomain"),
        "clock domains must be allocated explicitly",
    );
}
```

Add this `compile_fail` example to the public type documentation:

```rust
/// ```compile_fail
/// use qubit_clock::ClockDomain;
///
/// let domain = ClockDomain::default();
/// ```
```

- [x] **Step 2: Verify RED**

Run:

```text
cargo +1.94.0 test --test monotonic_tests clock_domain -- --nocapture
cargo +1.94.0 test --doc clock_domain
```

Expected: the integration test fails because the terminal allocator transitions
are absent, and the doctest fails because `ClockDomain::default()` still
compiles.

- [x] **Step 3: Implement the allocator transition**

Use zero as the terminal atomic state:

```rust
fn allocate_clock_domain_identifier(next: &AtomicU64) -> u64 {
    next.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| match value {
        0 => None,
        u64::MAX => Some(0),
        value => Some(value + 1),
    })
    .expect("monotonic clock domain identifiers exhausted")
}
```

Have `ClockDomain::new()` wrap that value and remove `impl Default for ClockDomain`.

- [x] **Step 4: Verify GREEN**

Run the two Task 1 commands again and require both to pass.

### Task 2: One-shot manual wake effects

**Files:**

- Modify: `tests/sleep/manual_sleep_future_tests.rs`
- Modify: `src/monotonic/manual_waiter_registry.rs`
- Modify: `src/monotonic/manual_monotonic_clock.rs`

**Interfaces:**

- Consumes: async waiter entries `(Duration, Option<Waker>)`.
- Produces: private `AdvanceEffects` and `take_due_async_wakers(&mut self, elapsed)`.

- [x] **Step 1: Write the failing wake-count test**

Poll a manual sleep with a counting waker, reach its deadline, advance again before repolling, and require the clock to issue only one wake while the registration remains pending:

```rust
#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn test_manual_sleep_future_is_woken_once_after_deadline_is_reached() {
    let clock = Arc::new(ManualMonotonicClock::new());
    let sleeper = ManualAsyncSleeper::from_clock(Arc::clone(&clock));
    let mut future = sleeper.sleep_for_async(Duration::from_secs(5));
    let wake_counter = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake_counter));
    let mut context = Context::from_waker(&waker);

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
    clock
        .advance(Duration::from_secs(5))
        .expect("deadline advance should succeed");
    assert_eq!(1, wake_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, clock.pending_waiters());

    clock
        .advance(Duration::from_secs(1))
        .expect("post-deadline advance should succeed");
    assert_eq!(1, wake_counter.0.load(Ordering::Relaxed));
    assert_eq!(1, clock.pending_waiters());

    assert!(matches!(
        future.as_mut().poll(&mut context),
        Poll::Ready(Ok(())),
    ));
    assert_eq!(0, clock.pending_waiters());
}
```

- [x] **Step 2: Verify RED**

Run:

```text
cargo +1.94.0 test --test sleep_tests test_manual_sleep_future_is_woken_once_after_deadline_is_reached -- --nocapture
```

Expected: failure with wake count `2` instead of `1`.

- [x] **Step 3: Implement minimal one-shot extraction**

Replace the tuple alias with:

```rust
struct AdvanceEffects {
    due_wakers: Vec<Waker>,
    advance_callbacks: Vec<AdvanceCallback>,
}
```

Change due-waker collection to iterate mutable waiter entries and call `Option::take()`. Keep the waiter entry until its future is polled or dropped, keep callback snapshots persistent, and preserve the existing Condvar → waker → callback order outside the mutex.

- [x] **Step 4: Verify GREEN**

Run the target test, then all manual monotonic and sleep tests.

### Task 3: Runtime and manual-driver contracts

**Files:**

- Modify: `src/monotonic/tokio_monotonic_clock.rs`
- Modify: `src/sleep/tokio_async_sleeper.rs`
- Modify: `src/sleep/manual_async_sleeper.rs`
- Modify: `README.md`
- Modify: `README.zh_CN.md`

**Interfaces:**

- Tokio clock/sleeper: coherent paused/advanced time requires creation, reads, and timer polling under the same Tokio runtime time driver; task movement between threads in that runtime remains valid.
- Manual sleeper: registration happens at future creation, relative deadlines are fixed at method call, unpolled futures count as pending, and cancellation unregisters.

- [x] **Step 1: Update public rustdoc**

Document that driver affinity is not runtime-validated and that the Tokio timer is still lazily created on first poll. Expand the manual sleeper type and method docs with the full registration/cancellation contract.

- [x] **Step 2: Update user documentation**

Add concise Tokio runtime-affinity paragraphs and complete the English and Chinese unified-manual-driver sections.

- [x] **Step 3: Verify documentation**

Run:

```text
cargo +1.94.0 test --all-features --doc
RUSTDOCFLAGS="-D warnings -D missing-docs" cargo +1.94.0 doc --all-features --no-deps
```

### Task 4: Reconcile historical design records

**Files:**

- Modify: `doc/clock_api_simplification_design.zh_CN.md`
- Modify: `doc/clock_api_simplification_implementation_plan.zh_CN.md`

- [x] **Step 1: Mark implementation status accurately**

Change the design status from pending to implemented/reviewed and add the same historical-record status to the plan.

- [x] **Step 2: Align documented responsibilities**

Record that `ManualMonotonicClock` owns elapsed transitions and effect collection, `ManualMonotonicState` groups mutex-protected data, `AdvanceEffects` is the only named effect introduced, and due wakers are taken once while callbacks remain snapshots.

- [x] **Step 3: Record boundary contracts**

Document explicit `ClockDomain` construction, the valid `u64::MAX` allocation followed by terminal panic, and Tokio same-driver requirements.

### Task 5: Full verification

- [x] **Step 1: Format and lint**

```text
cargo +nightly-2026-06-05 fmt --all -- --check --config-path .rs-ci/rustfmt.toml
cargo +nightly-2026-06-05 clippy --all-targets --all-features -- -D warnings
```

- [x] **Step 2: Test both feature configurations**

```text
cargo +1.94.0 test --no-default-features
cargo +1.94.0 test --all-features
```

- [x] **Step 3: Check documentation and diff**

```text
RUSTDOCFLAGS="-D warnings -D missing-docs" cargo +1.94.0 doc --all-features --no-deps
git --no-pager diff --check
git status --short
```

Expected: every command exits successfully; changes remain limited to the files listed above plus this plan.
