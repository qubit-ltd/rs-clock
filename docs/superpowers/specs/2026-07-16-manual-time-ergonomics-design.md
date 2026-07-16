# Manual Time Ergonomics Design

## Goal

Reduce repeated manual-clock assembly in downstream tests while preserving the
existing separation between wall time, monotonic time, blocking sleep, and
asynchronous sleep.

The design also adds asynchronous coordination for the next strictly future
manual deadline so multi-round async test drivers do not rely on waiter counts
that may include reached but not yet removed waiters.

## Design Principles

1. Production code consumes only the capability it needs through
   `Arc<dyn WallClock>`, `Arc<dyn BlockingSleeper>`, or
   `Arc<dyn AsyncSleeper>`.
2. A sleeper remains the authoritative owner of its paired monotonic clock;
   production consumers do not receive a separate clock merely to measure the
   sleeper's deadlines.
3. A test retains `Arc<ManualMonotonicClock>` as its control-plane handle for
   advancing time and observing registrations.
4. Components derived from one manual clock must share exactly the same clock
   allocation and therefore the same `ClockDomain`.
5. The API solves the observed manual-test assembly problem without introducing
   a universal clock-family abstraction or wrappers around arbitrary
   `Arc<dyn Trait>` values.

## Public API

`ManualMonotonicClock` gains the following constructors:

```rust
impl ManualMonotonicClock {
    pub fn new_shared() -> Arc<Self>;

    pub fn new_wall_clock(
        self: &Arc<Self>,
        wall_time: SystemTime,
    ) -> Arc<ManualWallClock>;

    pub fn new_blocking_sleeper(
        self: &Arc<Self>,
    ) -> Arc<ManualBlockingSleeper>;

    pub fn new_async_sleeper(
        self: &Arc<Self>,
    ) -> Arc<ManualAsyncSleeper>;

    pub fn wait_for_next_deadline_async(
        self: &Arc<Self>,
    ) -> ManualDeadlineFuture;
}
```

The `new_*` names state that each call creates a new lightweight component
handle. The returned concrete `Arc<Manual...>` values remain usable through
manual-only APIs and coerce naturally to the existing `Arc<dyn ...>` injection
types.

`new_wall_clock` keeps the wall-time anchor explicit. No default wall timestamp
is invented.

## Async Deadline Coordination

`ManualDeadlineFuture` is a public, cancellation-safe future with
`Output = MonotonicInstant`.

Its contract is:

- registration occurs when `wait_for_next_deadline_async` is called, not on the
  first poll;
- every poll evaluates the current waiter state and resolves to the earliest
  active deadline strictly later than the clock's current elapsed time;
- waiter registration wakes a pending observer but does not latch a deadline;
- waiters cancelled before observer polling do not satisfy it;
- reached waiters that have not yet been removed do not satisfy it;
- dropping a pending future immediately removes its observer registration;
- replacing a waker does not drop the old waker while the clock state lock is
  held;
- it has no built-in real-time timeout because callers can compose one with
  their async runtime when required.

The waiter registry will distinguish count observers used by
`ManualWaiterFuture` from future-deadline observers used by
`ManualDeadlineFuture`. Registration, predicate evaluation, waker replacement,
and removal remain serialized under the existing manual-clock state lock.
Detached wakers continue to be invoked only after releasing that lock.

The returned instant is a state snapshot, not an atomic reservation. A
concurrent task can register an earlier deadline after the future resolves.
Drivers use `advance_to_next_deadline()` to select and advance to the current
earliest deadline atomically; the observer is only the synchronization signal
that makes the driver runnable.

## Intended Downstream Usage

Retry tests retain the clock and inject a derived sleeper:

```rust
let clock = ManualMonotonicClock::new_shared();
let retry = RetryBuilder::new()
    .async_sleeper(clock.new_async_sleeper())
    .build();

clock.wait_for_next_deadline_async().await;
let _ = clock
    .advance_to_next_deadline()
    .expect("a retry deadline should remain registered");
```

ID-generator tests retain the concrete wall clock when they need to reanchor
it:

```rust
let clock = ManualMonotonicClock::new_shared();
let wall_clock = clock.new_wall_clock(now);
let generator = builder
    .wall_clock(wall_clock.clone())
    .blocking_sleeper(clock.new_blocking_sleeper())
    .build()?;
```

Lock tests that only need manual advancement continue to use the shared manual
clock directly.

## Downstream Migration

After the `rs-clock` API passes its own validation, direct construction patterns
in `rs-id`, `rs-retry`, and `rs-lock` will be migrated where the new API reduces
assembly without hiding test-specific behavior.

The migration will not replace application fields typed as `Arc<dyn ...>` and
will not introduce dependencies between downstream crates. Each downstream
crate will be validated independently with its repository-prescribed scripts.

## Testing

The shared-construction API will cover:

- creation of one shared manual clock;
- domain identity across derived wall, blocking, and async components;
- automatic coercion of concrete shared components at trait-object injection
  sites;
- wall-clock projection after manual advancement;
- independent wrapper creation without independent timelines.

The async deadline future will cover:

- immediate completion when a future deadline already exists;
- pending behavior until a deadline is registered;
- selection of the earliest mixed blocking/async deadline;
- ignoring reached but not yet removed waiters;
- registration before first poll without retaining cancelled registrations;
- selection of the current earliest deadline at poll time;
- cancellation cleanup;
- waker replacement and destruction outside the state lock;
- concurrent registration without lost wake-ups.

## Non-Goals

- No `ArcMonotonicClock`, `ArcWallClock`, `ArcBlockingSleeper`, or
  `ArcAsyncSleeper` wrapper types.
- No runtime downcasting from arbitrary `MonotonicClock` implementations.
- No universal std/manual/Tokio clock bundle.
- No new std async sleeper or Tokio blocking sleeper.
- No change to existing trait-object injection contracts.
- No automatic wall-time anchor.
