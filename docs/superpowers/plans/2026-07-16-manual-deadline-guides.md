# Manual Deadline Semantics and User Guides Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make asynchronous manual-deadline coordination report the earliest
currently active deadline, document the full `qubit-clock` toolkit in aligned
English and Chinese user guides, and migrate direct downstream tests to the
new coordination API.

**Architecture:** `ManualDeadlineFuture` remains eagerly registered, but its
poll result is derived from the waiter registry's current state instead of a
historically latched registration. This makes it the asynchronous equivalent
of `wait_for_next_deadline` and keeps time-driving decisions separate through
`advance_to_next_deadline`. Public documentation presents the capability model,
production clocks, manual test driver, and the distinction between waiter-count
and deadline coordination. `rs-retry` retains the concrete manual driver and
injects sleeper capabilities derived from it.

**Tech Stack:** Rust 1.94, edition 2024, standard-library synchronization,
Tokio test runtime, Markdown documentation, Cargo.

## Global Constraints

- Preserve the public signatures of `ManualMonotonicClock`,
  `ManualDeadlineFuture`, and the clock/sleeper traits.
- `wait_for_next_deadline_async` returns the earliest strictly future deadline
  that is still registered when the future is polled.
- A waiter registered and cancelled before observer polling must not produce a
  stale ready result.
- The observer must be registered when the method is called, not on first poll,
  so an active deadline cannot be missed.
- Time drivers should use `advance_to_next_deadline`; the observed instant is
  primarily useful for inspection and assertions.
- Keep `wait_for_waiters*` for tests that intentionally require a waiter count.
- Put the English and Chinese manuals in `doc/user_guide.en.md` and
  `doc/user_guide.zh_CN.md` as explicitly requested.
- Keep both manuals structurally aligned and link them from both README files.
- Do not add Arc wrapper types, universal time bundles, runtime downcasting, or
  new clock/sleeper implementations.
- Do not commit changes unless the user separately requests commits.

---

### Task 1: Current-state async deadline observation

**Files:**
- Modify: `tests/monotonic/manual_deadline_future_tests.rs`
- Modify: `src/monotonic/internal/manual_waiter_registry.rs`
- Modify: `src/monotonic/manual_monotonic_clock.rs`
- Modify: `src/monotonic/manual_deadline_future.rs`

**Interfaces:**
- Consumes: registered blocking and async waiter deadlines in
  `ManualWaiterRegistry`.
- Produces: unchanged public signature
  `ManualMonotonicClock::wait_for_next_deadline_async(self: &Arc<Self>) -> ManualDeadlineFuture`
  with current-state semantics.

- [ ] **Step 1: Change the first-registration regression test to require the
  earliest currently active deadline**

Replace the old first-registration-latching test with:

```rust
#[test]
fn test_manual_deadline_future_returns_earliest_deadline_at_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let sleeper = clock.new_async_sleeper();
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let _later = sleeper.sleep_for_async(Duration::from_secs(4));
    let _earlier = sleeper.sleep_for_async(Duration::from_secs(1));
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("an active future deadline should be ready");
    };
    assert_eq!(Duration::from_secs(1), deadline.elapsed_since_origin());
}
```

- [ ] **Step 2: Change the cancellation regression test to reject stale
  deadlines**

Replace the old registration-latching test with:

```rust
#[test]
fn test_manual_deadline_future_ignores_cancelled_deadline_before_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let sleeper = clock.new_async_sleeper();
    let mut observer = pin!(clock.wait_for_next_deadline_async());
    let cancelled = sleeper.sleep_for_async(Duration::from_secs(3));
    drop(cancelled);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    assert_eq!(Poll::Pending, observer.as_mut().poll(&mut context));

    let _active = sleeper.sleep_for_async(Duration::from_secs(2));
    let Poll::Ready(deadline) = observer.as_mut().poll(&mut context) else {
        panic!("the active future deadline should be ready");
    };
    assert_eq!(Duration::from_secs(2), deadline.elapsed_since_origin());
}
```

- [ ] **Step 3: Run the focused test and verify RED**

Run:

```bash
cargo test --all-features --test monotonic_tests manual_deadline_future
```

Expected: the earliest-at-poll test reports four seconds instead of one, and
the cancellation test reports `Ready` instead of `Pending`.

- [ ] **Step 4: Store only deadline-observer wakers and resolve readiness at
  poll time**

Change `deadline_observers` to:

```rust
deadline_observers: HashMap<u64, Option<Waker>>,
```

Make registration allocate and insert an empty waker without receiving elapsed
time:

```rust
pub(crate) fn register_deadline_observer(&mut self) -> u64 {
    let observer_id = allocate_identifier(
        &mut self.next_observer_id,
        "manual waiter observer identifiers exhausted",
    );
    self.deadline_observers.insert(observer_id, None);
    observer_id
}
```

Make polling calculate the current earliest deadline while holding the state
lock:

```rust
pub(crate) fn poll_deadline_observer(
    &mut self,
    observer_id: u64,
    elapsed: Duration,
    context: &Context<'_>,
) -> (Poll<Duration>, Option<Waker>) {
    if !self.deadline_observers.contains_key(&observer_id) {
        panic!("manual deadline observer {observer_id} is not registered");
    }
    if let Some(deadline) = self.next_future_deadline(elapsed) {
        let removed_waker = self
            .deadline_observers
            .remove(&observer_id)
            .flatten();
        return (Poll::Ready(deadline), removed_waker);
    }
    let registered_waker = self
        .deadline_observers
        .get_mut(&observer_id)
        .expect("deadline observer existence was checked");
    let replaced_waker = if registered_waker
        .as_ref()
        .is_none_or(|waker| !waker.will_wake(context.waker()))
    {
        registered_waker.replace(context.waker().clone())
    } else {
        None
    };
    (Poll::Pending, replaced_waker)
}
```

Wake deadline observers whenever at least one future deadline is active, but do
not remove or latch them until poll:

```rust
if next_deadline.is_some() {
    self.deadline_observers.values_mut().for_each(|waker| {
        if let Some(waker) = waker.take() {
            wakers.push(waker);
        }
    });
}
```

Adjust unregistering to remove the stored waker directly. Adjust
`ManualMonotonicClock::{register_deadline_observer,poll_deadline_observer}` to
the new internal signatures and continue dropping detached wakers after the
mutex is released.

- [ ] **Step 5: Run the focused tests and verify GREEN**

Run:

```bash
cargo test --all-features --test monotonic_tests manual_deadline_future
```

Expected: all manual deadline future tests pass.

- [ ] **Step 6: Run all monotonic tests**

Run:

```bash
cargo test --all-features --test monotonic_tests
```

Expected: all monotonic tests pass without warnings.

---

### Task 2: Public Rustdoc contract and examples

**Files:**
- Modify: `src/monotonic/manual_monotonic_clock.rs`
- Modify: `src/monotonic/manual_deadline_future.rs`
- Modify: `src/sleep/async_sleeper.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: current-state semantics from Task 1 and existing shared factories.
- Produces: runnable Rustdoc examples for the recommended manual-time workflow.

- [ ] **Step 1: Document the exact asynchronous deadline contract**

Update `wait_for_next_deadline_async` and `ManualDeadlineFuture` to state:

- the observer is registered at method call time;
- each poll examines currently registered deadlines strictly after `clock.now()`;
- cancelled or already due registrations do not make it ready;
- the returned deadline was earliest while polling held the state lock, but a
  later concurrent registration can change current ordering;
- drivers should call `advance_to_next_deadline` rather than blindly advancing
  to an earlier observation;
- dropping the observer unregisters its waker and does not affect sleepers.

Add a runnable example which creates the observer before concurrently polling
the producer task and then drives the current earliest deadline:

```rust
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_async_sleeper();
let task = tokio::spawn(async move {
    sleeper.sleep_for_async(Duration::from_secs(5)).await
});

let observed_deadline = clock.wait_for_next_deadline_async().await;
assert_eq!(Duration::from_secs(5), observed_deadline.elapsed_since_origin());
let _ = clock
    .advance_to_next_deadline()
    .expect("the active deadline should remain registered");
task.await??;
```

- [ ] **Step 2: Distinguish count and deadline coordination**

Update `wait_for_waiters_async` to link to
`wait_for_next_deadline_async` for later-stage coordination and explain that
count coordination intentionally includes due async waiters awaiting cleanup.

- [ ] **Step 3: Modernize the AsyncSleeper example and crate-level example**

Use `ManualMonotonicClock::new_shared()` and
`clock.new_async_sleeper()`/`clock.new_blocking_sleeper()`. Remove manual Arc
assembly from recommendation-oriented examples. Keep low-level `from_clock`
examples only where that constructor itself is being documented or tested.

- [ ] **Step 4: Run doctests**

Run:

```bash
cargo test --all-features --doc
```

Expected: every Rustdoc example passes.

---

### Task 3: Aligned English and Chinese user manuals

**Files:**
- Create: `doc/user_guide.en.md`
- Create: `doc/user_guide.zh_CN.md`
- Modify: `README.md`
- Modify: `README.zh_CN.md`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: all public clock, sleeper, instant, error, manual coordination, and
  subscription APIs.
- Produces: bilingual manuals linked by both README files and included in the
  published Cargo package.

- [ ] **Step 1: Write structurally aligned manuals**

Give both manuals the same section order:

1. Mental model and capability selection.
2. Installation and the `tokio` feature.
3. Wall clocks: `StdWallClock`, `FixedWallClock`, `ManualWallClock`.
4. Monotonic clocks and domain-scoped `MonotonicInstant`.
5. Blocking sleepers.
6. Tokio clocks and async sleepers, including runtime affinity.
7. Manual test setup through `new_shared` and derived factories.
8. Waiter-count versus future-deadline coordination.
9. Blocking multi-stage time driving.
10. Async coordination with concurrent polling.
11. Mixed blocking and async waiters.
12. Manual wall-time reanchoring.
13. Advance subscriptions and callback constraints.
14. Trait-object injection and custom implementations.
15. `TimeError`, clock-domain mismatch, overflow, and backward movement.
16. Recommended patterns and common mistakes.

Every section that introduces an operation must include a minimal code example
when code improves understanding. English and Chinese examples must remain
semantically identical.

- [ ] **Step 2: Reduce README files to accurate entry documents**

Keep the badges, one-paragraph purpose, capability table, installation snippets,
a minimal recommended manual-time example, links to both manuals, and license.
Remove duplicated long-form guidance so the manuals are the canonical source.

- [ ] **Step 3: Include manuals in Cargo packages**

Add `"/doc/**"` to `package.include` in `Cargo.toml`.

- [ ] **Step 4: Check package contents**

Run:

```bash
cargo package --list --allow-dirty
```

Expected: both `doc/user_guide.en.md` and `doc/user_guide.zh_CN.md` appear in
the package list.

---

### Task 4: Scenario and downstream adoption

**Files:**
- Modify: `tests/scenario/retry_backoff_tests.rs`
- Modify: `../rs-retry/tests/executor/async_retry_runner_tests.rs`

**Interfaces:**
- Consumes: `ManualMonotonicClock::{new_shared,new_async_sleeper,wait_for_next_deadline_async,advance_to_next_deadline}`.
- Produces: scenario tests that use deadline coordination for stage discovery
  and retain waiter counts only for explicit count assertions.

- [ ] **Step 1: Modernize the rs-clock async retry scenario**

Use shared factories and wait for the future deadline:

```rust
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_async_sleeper();
// spawn producer task
let _observed_deadline = clock.wait_for_next_deadline_async().await;
let _ = clock
    .advance_to_next_deadline()
    .expect("the timeout deadline should remain active");
```

- [ ] **Step 2: Replace nine rs-retry stage-discovery count observers**

At each `wait_for_waiters_async(1)` used only to discover a timeout or backoff
deadline, use:

```rust
let deadline_registration = clock.wait_for_next_deadline_async();
tokio::select! {
    result = &mut retry_future => {
        panic!("retry completed before manual time advanced: {result:?}");
    }
    _ = deadline_registration => {}
}
```

Keep explicit `pending_waiters()` assertions unchanged. Do not replace any
coordination that intentionally requires more than one simultaneous waiter.

- [ ] **Step 3: Make the async backoff test cover two successive deadlines**

Configure three attempts, fail the first two, and for each stage create a new
deadline observer before polling the retry future. Advance with
`advance_to_next_deadline()` after each observation. Assert three total attempts
and final success. This verifies that a due waiter awaiting cleanup cannot
satisfy the next-stage observer.

- [ ] **Step 4: Run focused rs-clock scenarios**

Run from `rs-clock`:

```bash
cargo test --all-features --test scenario_tests
```

Expected: all scenarios pass.

- [ ] **Step 5: Run focused rs-retry tests**

Run from `rs-retry`:

```bash
cargo test --all-features --test lib_tests async_retry_runner_tests
```

Expected: all async retry runner tests pass.

---

### Task 5: Repository validation and final review

**Files:**
- Inspect: all changed files in `rs-clock`
- Inspect: all changed files in `rs-retry`

**Interfaces:**
- Consumes: completed Tasks 1–4.
- Produces: formatted, CI-checked changes with no unrelated modifications.

- [ ] **Step 1: Run rs-clock alignment and inspect mutations**

Run from `rs-clock`:

```bash
./align-ci.sh
git --no-pager diff
```

Expected: alignment exits zero; the diff contains only deadline semantics,
documentation, examples, and planned tests.

- [ ] **Step 2: Run rs-clock CI-equivalent validation**

Run:

```bash
./ci-check.sh
```

Expected: exit zero. Run `./coverage.sh json` only if CI explicitly reports
coverage below threshold.

- [ ] **Step 3: Run rs-retry alignment and inspect mutations**

Run from `rs-retry`:

```bash
./align-ci.sh
git --no-pager diff
```

Expected: alignment exits zero; only async manual-time tests change.

- [ ] **Step 4: Run rs-retry CI-equivalent validation**

Run:

```bash
./ci-check.sh
```

Expected: exit zero. Run `./coverage.sh json` only if CI explicitly reports
coverage below threshold.

- [ ] **Step 5: Audit requirements and repository state**

Check:

```bash
git status --short --branch
git -C ../rs-retry status --short --branch
rg -n 'wait_for_waiters_async\(1\)' ../rs-retry/tests/executor/async_retry_runner_tests.rs
rg -n 'Arc::new\(ManualMonotonicClock::new\(\)\)' README.md README.zh_CN.md doc src/lib.rs src/sleep/async_sleeper.rs
```

Expected: the two searches return no matches in recommendation-oriented docs
or the migrated rs-retry test; repository status lists only planned files.

No commit is part of this plan because the current user request authorizes code
and documentation changes but does not authorize new Git commits.
