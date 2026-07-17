# Timer Time-Domain Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace backend-specific sleeper traits with one object-safe Timer capability, make every monotonic clock create a same-domain Timer through `new_timer(&self)`, and provide one concrete BlockingSleeper adapter for production and deterministic tests.

**Architecture:** `Timer::at` eagerly registers an owned `TimerFuture`; `Timer::after` fixes its absolute deadline at call time. `StdTimer`, `TokioTimer`, and `ManualTimer` bind to private same-domain clock handles, while `BlockingSleeper` manually polls any TimerFuture with a thread-unparking Waker. Manual time moves to shared private state so a borrowed clock can create independently owned timers without making public clock handles cloneable.

**Tech Stack:** Rust 2024, standard `Future`/`Waker`, `std::sync`, Tokio time driver behind the existing `tokio` feature, external integration tests, crate-provided `align-ci.sh` and `ci-check.sh`.

## Global Constraints

- Breaking changes are allowed; do not add deprecated aliases or compatibility wrappers.
- `TimerFuture` is `Pin<Box<dyn Future<Output = ()> + Send + 'static>>`.
- Timer methods are named `at` and `after`; only blocking APIs use `sleep_*` or `wait_*`.
- `MonotonicClock::new_timer` takes `&self`, does not move the clock, and returns `Arc<dyn Timer>` from the same `ClockDomain`.
- `at` registers before returning; completion before first poll is latched; dropping a pending future cancels registration.
- Timer registration errors are returned synchronously as `TimeError`; a successful TimerFuture cannot later return an error.
- `BlockingSleeper` is a concrete struct that composes `Arc<dyn Timer>` and never implements `Timer`.
- Public and private functions receive complete rustdoc according to the repository rules.
- Tests live under `tests/` with source-mirroring file names; no inline `#[cfg(test)]` modules.
- Do not modify wall-clock behavior except for necessary imports or same-domain manual shared-state plumbing.
- Each task preserves a compiling crate and ends with its focused tests passing.

---

### Task 1: Introduce the Timer contract and registration error

**Files:**
- Create: `src/timer/mod.rs`
- Create: `src/timer/timer.rs`
- Create: `src/timer/timer_future.rs`
- Modify: `src/error/time_error.rs`
- Modify: `src/lib.rs`
- Create: `tests/timer/mod.rs`
- Create: `tests/timer/timer_tests.rs`
- Create: `tests/timer_tests.rs`

**Interfaces:**
- Consumes: existing `MonotonicClock`, `MonotonicInstant`, and `TimeError`.
- Produces: `Timer`, `TimerFuture`, and `TimeError::TimerUnavailable` for all later tasks.

- [ ] **Step 1: Add failing public-contract tests**

Create a recording timer backed by the existing public manual clock, proving object safety, call-time deadline calculation, immediate creation errors, and `'static` futures. The timer records the deadline passed to `at` and returns `Box::pin(std::future::ready(()))`:

```rust
struct RecordingTimer {
    clock: Arc<ManualMonotonicClock>,
    deadline: Mutex<Option<MonotonicInstant>>,
}

impl RecordingTimer {
    fn new(clock: Arc<ManualMonotonicClock>) -> Self {
        Self {
            clock,
            deadline: Mutex::new(None),
        }
    }

    fn recorded_deadline(&self) -> Option<MonotonicInstant> {
        *self.deadline.lock().expect("deadline lock should succeed")
    }
}

impl Timer for RecordingTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn at(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        let expected = self.clock.now().domain();
        let actual = deadline.domain();
        if expected != actual {
            return Err(TimeError::ClockDomainMismatch {
                expected,
                actual,
            });
        }
        *self.deadline.lock().expect("deadline lock should succeed") =
            Some(deadline);
        Ok(Box::pin(std::future::ready(())))
    }
}

#[test]
fn test_after_fixes_deadline_when_called() {
    let clock = Arc::new(ManualMonotonicClock::new());
    clock
        .advance(Duration::from_secs(3))
        .expect("manual clock should advance");
    let timer = RecordingTimer::new(clock);
    let _future = timer
        .after(Duration::from_secs(5))
        .expect("timer registration should succeed");
    assert_eq!(
        Some(Duration::from_secs(8)),
        timer.recorded_deadline().map(|value| value.elapsed_since_origin()),
    );
}

fn assert_object_safe(_: &dyn Timer) {}

struct FailingTimer {
    clock: Arc<ManualMonotonicClock>,
}

impl Timer for FailingTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Err(TimeError::TimerUnavailable)
    }
}
```

Call `assert_object_safe(&timer)` from the deadline test. Add a second test around `FailingTimer` and assert that `after` returns `TimeError::TimerUnavailable` immediately without creating a Future.

- [ ] **Step 2: Run the contract test and verify RED**

Run:

```bash
cargo test --test timer_tests
```

Expected: compilation fails because `Timer`, `TimerFuture`, and `TimeError::TimerUnavailable` do not exist.

- [ ] **Step 3: Implement the core API**

Add the public alias and object-safe trait:

```rust
pub type TimerFuture =
    Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub trait Timer: Send + Sync {
    fn clock(&self) -> &dyn MonotonicClock;

    fn at(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError>;

    fn after(
        &self,
        duration: Duration,
    ) -> Result<TimerFuture, TimeError> {
        let deadline = self.clock().now().checked_add(duration)?;
        self.at(deadline)
    }
}
```

Implement forwarding for `Arc<T>` and `Box<T>` exactly as the existing clock/sleeper traits do. Add this non-payload error so `TimeError` remains `Copy`:

```rust
#[error("monotonic timer driver is unavailable")]
TimerUnavailable,
```

Declare and re-export the timer module from `src/lib.rs` without changing sleeper exports yet.

- [ ] **Step 4: Run focused tests and formatting**

Run:

```bash
cargo fmt --all -- --check
cargo test --test timer_tests
```

Expected: both commands pass.

- [ ] **Step 5: Commit the core contract**

```bash
git add src/error src/timer src/lib.rs tests/timer tests/timer_tests.rs
git commit -m "feat(clock): 引入 Timer 核心契约"
```

### Task 2: Implement ManualTimer on shared manual time-domain state

**Files:**
- Create: `src/monotonic/internal/manual_time_domain.rs`
- Modify: `src/monotonic/internal/mod.rs`
- Modify: `src/monotonic/manual_monotonic_clock.rs`
- Create: `src/timer/manual_timer.rs`
- Create: `src/timer/internal/mod.rs`
- Create: `src/timer/internal/manual_timer_future.rs`
- Modify: `src/timer/mod.rs`
- Create: `tests/timer/manual_timer_tests.rs`
- Modify: `tests/timer/mod.rs`
- Modify: `tests/monotonic/manual_monotonic_clock_tests.rs`
- Modify: `tests/monotonic/internal/manual_waiter_registry_tests.rs`

**Interfaces:**
- Consumes: `Timer`, `TimerFuture`, current manual waiter/deadline observer APIs.
- Produces: public `ManualTimer::from_clock(&ManualMonotonicClock)`, a private same-domain clock handle constructor, and eager/cancellation-safe manual TimerFuture behavior.

- [ ] **Step 1: Add failing ManualTimer lifecycle tests**

Add tests for registration before first poll, deadline latching, immediate readiness, cancellation, Waker replacement, and coordination APIs:

```rust
#[test]
fn test_manual_timer_registers_before_first_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let future = timer
        .after(Duration::from_secs(4))
        .expect("manual timer registration should succeed");

    assert_eq!(1, clock.pending_waiters());
    assert_eq!(
        Some(Duration::from_secs(4)),
        clock.next_deadline().map(|value| value.elapsed_since_origin()),
    );

    drop(future);
    assert_eq!(0, clock.pending_waiters());
}

#[tokio::test]
async fn test_manual_timer_latches_completion_before_first_poll() {
    let clock = ManualMonotonicClock::new_shared();
    let timer = ManualTimer::from_clock(clock.as_ref());
    let future = timer
        .after(Duration::from_secs(2))
        .expect("manual timer registration should succeed");

    clock
        .advance(Duration::from_secs(2))
        .expect("manual time should advance");
    future.await;
    assert_eq!(0, clock.pending_waiters());
}
```

`ManualTimer::from_clock` is the typed constructor used by this task and later by `MonotonicClock::new_timer`; it clones only the private time-domain handle and does not borrow the supplied clock.

- [ ] **Step 2: Run ManualTimer tests and verify RED**

Run:

```bash
cargo test --test timer_tests manual_timer
```

Expected: compilation fails because `ManualTimer` and its future do not exist.

- [ ] **Step 3: Move manual mutable state behind one private Arc**

Introduce a private domain object:

```rust
pub(crate) struct ManualTimeDomain {
    pub(crate) state: Mutex<ManualMonotonicState>,
    pub(crate) changed: Condvar,
    pub(crate) waiters_changed: Condvar,
}
```

Change `ManualMonotonicClock` to:

```rust
pub struct ManualMonotonicClock {
    domain: ClockDomain,
    time_domain: Arc<ManualTimeDomain>,
}
```

Add a private `same_domain_handle(&self) -> Self` that copies `ClockDomain` and clones `time_domain`. Route every existing lock, advance, observer, wall-clock, and waiter method through `time_domain`. Keep the public clock non-`Clone`; `new_shared()` remains available.

Keep advance subscribers temporarily so this task remains source-compatible with the still-present legacy API. Task 7 removes the subscription wrapper, registry, and callback fanout together.

- [ ] **Step 4: Implement eager ManualTimer registration**

`ManualTimer` owns `Arc<ManualMonotonicClock>` created from the private same-domain handle:

```rust
pub struct ManualTimer {
    clock: Arc<ManualMonotonicClock>,
}

impl ManualTimer {
    pub fn from_clock(clock: &ManualMonotonicClock) -> Self {
        Self {
            clock: Arc::new(clock.same_domain_handle()),
        }
    }
}

impl Timer for ManualTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn at(
        &self,
        deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        let future = ManualTimerFuture::register(
            Arc::clone(&self.clock),
            deadline,
        )?;
        Ok(Box::pin(future))
    }
}
```

`ManualTimerFuture::register` calls the existing eager waiter registration and returns `Result<Self, TimeError>`; it does not store an error for later poll. Its `Future::Output` is `()`. Polling a reached waiter removes the ID; Drop unregisters an outstanding ID. An already reached deadline stores `None` and polls immediately ready.

Use the existing async waiter map for Timer registrations during this task so legacy blocking sleepers continue compiling. `pending_waiters`, next-deadline observers, and advance methods already combine both maps and must therefore observe Timer registrations immediately. Task 7 removes the legacy blocking map and renames the remaining async waiter concepts to Timer waiter concepts.

- [ ] **Step 5: Run manual clock and timer tests**

Run:

```bash
cargo fmt --all -- --check
cargo test --test timer_tests manual_timer
cargo test --test monotonic_tests manual
```

Expected: all commands pass; no test sleeps in real time.

- [ ] **Step 6: Commit ManualTimer**

```bash
git add src/monotonic src/timer tests/monotonic tests/timer
git commit -m "feat(clock): 实现手动时间 Timer"
```

### Task 3: Implement the shared-worker StdTimer

**Files:**
- Modify: `src/monotonic/std_monotonic_clock.rs`
- Create: `src/timer/std_timer.rs`
- Create: `src/timer/internal/std_timer_scheduler.rs`
- Create: `src/timer/internal/std_timer_future.rs`
- Create: `src/timer/internal/std_timer_waiter.rs`
- Modify: `src/timer/internal/mod.rs`
- Modify: `src/timer/mod.rs`
- Create: `tests/timer/std_timer_tests.rs`
- Modify: `tests/timer/mod.rs`
- Create: `tests/support/block_on_timer_future.rs`
- Modify: `tests/support/mod.rs`

**Interfaces:**
- Consumes: `Timer`, `TimerFuture`, `TimeError::TimerUnavailable`, and the standard clock domain/origin.
- Produces: public `StdTimer::from_clock(&StdMonotonicClock)` with one lazy scheduler worker per timer instance.

- [ ] **Step 1: Add failing StdTimer behavior tests**

Cover past deadlines, real short deadlines, cancellation, a newly inserted earlier deadline, and many futures sharing one scheduler. The single-worker invariant is verified by the scheduler structure and code review rather than a public test hook.

Add a test helper that polls a `TimerFuture` with a thread-unparking Waker until ready. The helper uses the same wake-before-park atomic protocol specified for the production BlockingSleeper, but remains test-only.

```rust
#[test]
fn test_std_timer_completes_many_deadlines_with_one_scheduler() {
    let clock = StdMonotonicClock::new();
    let timer = StdTimer::from_clock(&clock);
    let futures = (0..32)
        .map(|_| timer.after(Duration::from_millis(5)))
        .collect::<Result<Vec<_>, _>>()
        .expect("all deadlines should register");

    futures.into_iter().for_each(block_on_timer_future);
}
```

- [ ] **Step 2: Run StdTimer tests and verify RED**

Run:

```bash
cargo test --test timer_tests std_timer
```

Expected: compilation fails because `StdTimer` does not exist.

- [ ] **Step 3: Implement scheduler state and waiter future**

Use one `Arc<StdTimerScheduler>` containing a mutex, condvar, deadline min-heap, active waiter map, monotonic ID allocator, and `worker_running` flag. Each waiter owns an atomic ready bit and optional Waker protected by a mutex:

```rust
pub(crate) struct StdTimerWaiter {
    ready: AtomicBool,
    waker: Mutex<Option<Waker>>,
}
```

Registration inserts the waiter and heap key while holding the scheduler lock. If no worker is running, start one with `thread::Builder`; on spawn failure remove the just-added registration, reset `worker_running`, and return `TimeError::TimerUnavailable`.

The worker waits until the earliest native `Instant`, skips cancelled stale heap keys, removes due active waiters, marks them ready, and invokes detached Wakers outside the scheduler lock. It sets `worker_running = false` and exits when no active deadlines remain; a later registration starts a new worker.

`StdTimerFuture::poll` installs/replaces its Waker, double-checks the atomic ready bit, and returns `Ready(())` when due. Drop removes an active registration and wakes the scheduler so it can recalculate its next deadline.

- [ ] **Step 4: Implement StdTimer domain conversion**

Add private `StdMonotonicClock::same_domain_handle(&self) -> Self`, preserving domain and origin without implementing public `Clone`. `StdTimer::from_clock(&StdMonotonicClock)` stores that handle, validates domain, converts elapsed deadline to native origin-relative `Instant`, returns an immediately ready future for reached deadlines, and otherwise registers with the scheduler.

- [ ] **Step 5: Run focused stress tests**

Run:

```bash
cargo fmt --all -- --check
cargo test --test timer_tests std_timer -- --test-threads=1
```

Expected: all tests pass repeatedly without leaked worker threads or deadline hangs.

- [ ] **Step 6: Commit StdTimer**

```bash
git add src/monotonic/std_monotonic_clock.rs src/timer tests/timer
git commit -m "feat(clock): 实现共享调度器 StdTimer"
```

### Task 4: Implement TokioTimer with eager Tokio registration

**Files:**
- Modify: `src/monotonic/tokio_monotonic_clock.rs`
- Create: `src/timer/tokio_timer.rs`
- Modify: `src/timer/mod.rs`
- Create: `tests/timer/tokio_timer_tests.rs`
- Modify: `tests/timer/mod.rs`

**Interfaces:**
- Consumes: `Timer`, `TimerFuture`, Tokio's `Sleep`, and `TimeError::TimerUnavailable`.
- Produces: feature-gated public `TokioTimer::from_clock(&TokioMonotonicClock)` with eager registration and paused-time support.

- [ ] **Step 1: Add failing paused-time and runtime-error tests**

```rust
#[tokio::test(start_paused = true)]
async fn test_tokio_timer_latches_before_first_poll() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);
    let future = timer
        .after(Duration::from_secs(8))
        .expect("Tokio deadline should register");

    tokio::time::advance(Duration::from_secs(8)).await;
    future.await;
}

#[test]
fn test_tokio_timer_reports_missing_driver_at_registration() {
    let clock = TokioMonotonicClock::new();
    let timer = TokioTimer::from_clock(&clock);
    assert_eq!(
        Err(TimeError::TimerUnavailable),
        timer.after(Duration::from_secs(1)).map(drop),
    );
}
```

- [ ] **Step 2: Run TokioTimer tests and verify RED**

Run:

```bash
cargo test --all-features --test timer_tests tokio_timer
```

Expected: compilation fails because `TokioTimer` does not exist.

- [ ] **Step 3: Implement eager Tokio Sleep creation**

Add private `TokioMonotonicClock::same_domain_handle`. `TokioTimer::at` validates domain and native conversion, returns immediate ready for a reached deadline, and constructs `tokio::time::sleep_until` before returning:

```rust
let sleep = std::panic::catch_unwind(
    std::panic::AssertUnwindSafe(|| tokio::time::sleep_until(native_deadline)),
)
.map_err(|_| TimeError::TimerUnavailable)?;
Ok(Box::pin(async move {
    sleep.await;
}))
```

Use `Handle::try_current` first to avoid entering the panic path when no runtime exists. The catch boundary covers a runtime with time disabled. Do not defer `sleep_until` into the async block.

- [ ] **Step 4: Run Tokio feature tests**

Run:

```bash
cargo fmt --all -- --check
cargo test --all-features --test timer_tests tokio_timer
```

Expected: all tests pass under real and paused Tokio time.

- [ ] **Step 5: Commit TokioTimer**

```bash
git add src/monotonic/tokio_monotonic_clock.rs src/timer tests/timer
git commit -m "feat(clock): 实现 TokioTimer"
```

### Task 5: Add `MonotonicClock::new_timer(&self)` to every clock

**Files:**
- Modify: `src/monotonic/monotonic_clock.rs`
- Modify: `src/monotonic/manual_monotonic_clock.rs`
- Modify: `src/monotonic/std_monotonic_clock.rs`
- Modify: `src/monotonic/tokio_monotonic_clock.rs`
- Modify: `tests/monotonic/monotonic_clock_tests.rs`
- Modify: `tests/monotonic/manual_monotonic_clock_tests.rs`
- Modify: `tests/monotonic/std_monotonic_clock_tests.rs`
- Modify: `tests/monotonic/tokio_monotonic_clock_tests.rs`

**Interfaces:**
- Consumes: all three concrete Timer implementations.
- Produces: final object-safe `MonotonicClock::new_timer(&self) -> Arc<dyn Timer>` and forwarding through `Arc<T>`/`Box<T>`.

- [ ] **Step 1: Add failing direct-call and same-domain tests**

```rust
#[test]
fn test_new_timer_does_not_consume_trait_object_clock() {
    let clock: Arc<dyn MonotonicClock> =
        Arc::new(ManualMonotonicClock::new());
    let timer = clock.new_timer();

    assert_eq!(clock.now().domain(), timer.clock().now().domain());
    let _still_usable = clock.now();
}

fn assert_clock_object_safe(_: &dyn MonotonicClock) {}
```

Update `ExternalMonotonicClock` compile tests to implement a small test Timer, proving external implementors receive the deliberate breaking requirement.

- [ ] **Step 2: Run clock tests and verify RED**

Run:

```bash
cargo test --test monotonic_tests monotonic_clock
```

Expected: compilation fails because `new_timer` is not part of the trait.

- [ ] **Step 3: Add and implement the factory method**

Add:

```rust
#[must_use = "the timer should be retained to register deadlines"]
fn new_timer(&self) -> Arc<dyn Timer>;
```

Implement `ManualMonotonicClock` with `Arc::new(ManualTimer::from_clock(self))`, `StdMonotonicClock` with `Arc::new(StdTimer::from_clock(self))`, and the feature-gated Tokio clock with `Arc::new(TokioTimer::from_clock(self))`. Extend `Arc<T>` and `Box<T>` blanket implementations by delegation. Update rustdoc to define same-domain, independent-lifetime, and non-consuming semantics.

- [ ] **Step 4: Run all monotonic and timer tests**

Run:

```bash
cargo fmt --all -- --check
cargo test --all-features --test monotonic_tests
cargo test --all-features --test timer_tests
```

Expected: all tests pass.

- [ ] **Step 5: Commit clock factories**

```bash
git add src/monotonic tests/monotonic
git commit -m "feat(clock): 由单调时钟创建同域 Timer"
```

### Task 6: Replace the BlockingSleeper trait with one Timer adapter

**Files:**
- Replace: `src/sleep/blocking_sleeper.rs`
- Modify: `src/sleep/mod.rs`
- Modify: `src/lib.rs`
- Replace: `tests/sleep/blocking_sleeper_tests.rs`
- Modify: `tests/sleep/mod.rs`
- Modify: `tests/support/block_on_timer_future.rs`

**Interfaces:**
- Consumes: `Arc<dyn Timer>`, `Timer::at`, and `Timer::after`.
- Produces: concrete cloneable `BlockingSleeper` with `new`, `timer`, `sleep_until`, and `sleep_for`.

- [ ] **Step 1: Add failing backend-independent blocking tests**

Test the same concrete type with Std and Manual timers:

```rust
#[test]
fn test_blocking_sleeper_uses_manual_timer_without_real_delay() {
    let clock = ManualMonotonicClock::new_shared();
    let sleeper = BlockingSleeper::new(clock.new_timer());
    let worker = thread::spawn(move || {
        sleeper.sleep_for(Duration::from_secs(16))
    });

    assert!(clock.wait_for_waiters(1, Duration::from_secs(1)));
    clock
        .advance_to_next_deadline()
        .expect("blocking deadline should exist");
    worker
        .join()
        .expect("blocking worker should finish")
        .expect("blocking sleep should succeed");
}
```

Also test already-ready futures and wake-before-park with a deterministic custom Timer.

- [ ] **Step 2: Run blocking tests and verify RED**

Run:

```bash
cargo test --test sleep_tests blocking_sleeper
```

Expected: compilation fails because `BlockingSleeper` is still a trait.

- [ ] **Step 3: Implement the concrete adapter**

Replace the trait with:

```rust
#[derive(Clone)]
pub struct BlockingSleeper {
    timer: Arc<dyn Timer>,
}
```

Implement a private `ThreadWaker` containing `std::thread::Thread` and `AtomicBool`. Its `Wake` implementation stores the notification bit with `Release` ordering before calling `unpark`. The blocking poll loop polls first, then consumes the notification bit; if absent it parks once and always repolls after returning, including spurious wakes.

`sleep_until` calls `timer.at(deadline)?`; `sleep_for` calls `timer.after(duration)?`; both pass the returned future to the same private blocking poll function. `timer()` returns `self.timer.as_ref()`.

- [ ] **Step 4: Run sleep and timer tests**

Run:

```bash
cargo fmt --all -- --check
cargo test --all-features --test sleep_tests blocking_sleeper
cargo test --all-features --test timer_tests
```

Expected: all tests pass.

- [ ] **Step 5: Commit BlockingSleeper**

```bash
git add src/sleep/blocking_sleeper.rs src/sleep/mod.rs src/lib.rs tests/sleep tests/support
git commit -m "refactor(clock): 用 Timer 组合阻塞 Sleeper"
```

### Task 7: Remove legacy sleepers and advance subscriptions

**Files:**
- Delete: `src/sleep/async_sleeper.rs`
- Delete: `src/sleep/sleep_future.rs`
- Delete: `src/sleep/std_blocking_sleeper.rs`
- Delete: `src/sleep/manual_blocking_sleeper.rs`
- Delete: `src/sleep/manual_async_sleeper.rs`
- Delete: `src/sleep/tokio_async_sleeper.rs`
- Delete: `src/sleep/internal/manual_sleep_future.rs`
- Delete: `src/sleep/internal/mod.rs`
- Delete: `src/monotonic/manual_advance_subscription.rs`
- Delete: `src/monotonic/internal/manual_advance_registry.rs`
- Modify: `src/sleep/mod.rs`
- Modify: `src/monotonic/mod.rs`
- Modify: `src/monotonic/internal/mod.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`
- Delete: `tests/sleep/async_sleeper_tests.rs`
- Delete: `tests/sleep/manual_async_sleeper_tests.rs`
- Delete: `tests/sleep/manual_blocking_sleeper_tests.rs`
- Delete: `tests/sleep/std_blocking_sleeper_tests.rs`
- Delete: `tests/sleep/tokio_async_sleeper_tests.rs`
- Delete: `tests/sleep/sleep_future_tests.rs`
- Delete: `tests/sleep/internal/manual_sleep_future_tests.rs`
- Delete: `tests/monotonic/manual_advance_subscription_tests.rs`
- Modify: `tests/sleep/mod.rs`
- Modify: `tests/monotonic/mod.rs`
- Modify: `tests/scenario/lock_timeout_tests.rs`
- Modify: `tests/scenario/login_lockout_tests.rs`
- Modify: `tests/scenario/retry_backoff_tests.rs`
- Modify: `tests/scenario/mod.rs`
- Modify: `README.md`
- Modify: `README.zh_CN.md`
- Modify: `doc/user_guide.en.md`
- Modify: `doc/user_guide.zh_CN.md`

**Interfaces:**
- Consumes: final Timer and BlockingSleeper APIs.
- Produces: a public surface with no AsyncSleeper, backend Sleeper types, SleepFuture, or advance subscription.

- [ ] **Step 1: Add compile-surface assertions**

Update public API tests and doctests to import the replacement surface:

```rust
use qubit_clock::{
    BlockingSleeper,
    ManualMonotonicClock,
    MonotonicClock,
    StdMonotonicClock,
    Timer,
    TimerFuture,
};
```

Replace retry/backoff scenario setup with `clock.new_timer()` and either direct TimerFuture awaiting or `BlockingSleeper::new(timer)`.

- [ ] **Step 2: Inventory remaining legacy references**

Run:

```bash
rg -n "AsyncSleeper|SleepFuture|StdBlockingSleeper|ManualBlockingSleeper|ManualAsyncSleeper|TokioAsyncSleeper|subscribe_advances|ManualAdvanceSubscription" src tests README.md README.zh_CN.md doc
```

Expected: every match belongs to a source file, test, or user document listed in this task; the superseded design spec is excluded from the migration.

- [ ] **Step 3: Delete legacy implementation and migrate callers**

Remove the listed source and test files with `apply_patch`. Remove their module declarations and re-exports. Remove the blocking waiter map and blocking registration guard used only by `ManualBlockingSleeper`; rename the remaining async waiter concepts to Timer waiter concepts. Remove `ManualAdvanceRegistry` from manual state, reduce `AdvanceEffects` to due Timer Wakers, and delete callback fanout. Rename descriptions and examples from asynchronous sleep to Timer registration. Preserve Manual waiter/deadline observers.

Update the crate description to describe injectable clocks, timers, and deterministic blocking adaptation. Do not add aliases for old names.

- [ ] **Step 4: Run full crate validation**

Run:

```bash
./align-ci.sh
./ci-check.sh
```

Expected: formatting, all feature combinations, Clippy, tests, doctests, and rustdoc pass. Run `./coverage.sh json` only if `ci-check.sh` reports coverage below the repository threshold.

- [ ] **Step 5: Commit removal and documentation migration**

```bash
git add Cargo.toml src tests README.md README.zh_CN.md doc
git commit -m "refactor(clock)!: 删除旧 Sleeper 抽象" -m "BREAKING CHANGE: AsyncSleeper、后端 Sleeper 与 SleepFuture 由 Timer 和具体 BlockingSleeper 取代。"
```

### Task 8: Validate rs-clock for downstream consumption

**Files:**
- Update: `docs/superpowers/plans/2026-07-17-timer-time-domain-redesign.md`

**Interfaces:**
- Consumes: final rs-clock public API.
- Produces: a clean, locally consumable rs-clock revision for the separate rs-lock plan.

- [ ] **Step 1: Run targeted API searches**

Run:

```bash
rg -n "AsyncSleeper|SleepFuture|StdBlockingSleeper|ManualBlockingSleeper|ManualAsyncSleeper|TokioAsyncSleeper|subscribe_advances|ManualAdvanceSubscription" src tests README.md README.zh_CN.md doc
```

Expected: no live API references; historical design documents may mention removed names only when clearly marked as superseded.

- [ ] **Step 2: Run release-equivalent verification again**

Run:

```bash
./align-ci.sh
./ci-check.sh
git status --short
```

Expected: validation passes and the status contains only this plan's checkbox updates. Any production failure is fixed in the task that owns that file before this validation task continues.

- [ ] **Step 3: Review the complete diff and public docs**

Run:

```bash
git --no-pager diff HEAD~6 --stat
git --no-pager log -8 --oneline
```

Expected: changes are limited to Timer, monotonic shared state, BlockingSleeper, their tests/docs, and the approved removals.

- [ ] **Step 4: Record plan completion**

Mark every completed checkbox in this plan, then commit only the plan update:

```bash
git add docs/superpowers/plans/2026-07-17-timer-time-domain-redesign.md
git commit -m "docs(clock): 完成 Timer 重构实施计划"
```
