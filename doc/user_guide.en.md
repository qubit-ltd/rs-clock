# Qubit Clock User Guide

Qubit Clock provides injectable wall clocks, monotonic clocks, blocking
sleepers, and asynchronous sleepers. Its manual implementations let tests
coordinate on registered deadlines and advance logical time without waiting
for real time.

For the Chinese version, see the [中文用户手册](user_guide.zh_CN.md).

## 1. Choose the capability, not the implementation

Time-related code usually needs one of four distinct capabilities:

| Need | Inject | Main operation |
|---|---|---|
| Civil timestamps, dates, audit fields | `WallClock` | `now() -> SystemTime` |
| Elapsed time, timeout calculations, deadlines | `MonotonicClock` | `now() -> MonotonicInstant` |
| Blocking the current thread | `BlockingSleeper` | `sleep_for`, `sleep_until` |
| Suspending an async task | `AsyncSleeper` | `sleep_for_async`, `sleep_until_async` |

Wall time and monotonic time are intentionally separate. A wall clock can move
forward or backward after a system adjustment, so it is suitable for civil
timestamps but not elapsed-time measurement. A monotonic clock never moves
backward inside one clock domain, so it is the right source for durations,
deadlines, retries, and timeouts.

Each sleeper owns its paired monotonic clock and exposes it through `clock()`.
Production components that need to calculate a deadline and then sleep usually
need only the sleeper:

```rust
use qubit_clock::BlockingSleeper;
use std::time::Duration;

fn wait_for_batch(sleeper: &dyn BlockingSleeper) -> Result<(), qubit_clock::TimeError> {
    let started = sleeper.clock().now();
    sleeper.sleep_for(Duration::from_millis(20))?;
    let elapsed = sleeper.clock().now().duration_since(started)?;
    assert!(elapsed >= Duration::from_millis(20));
    Ok(())
}
```

Do not inject an unrelated monotonic clock beside a sleeper. If a component
truly needs both handles, construct the sleeper from that exact clock so both
use the same `ClockDomain`.

## 2. Installation and features

The standard and manual implementations require no optional feature:

```toml
[dependencies]
qubit-clock = "0.9"
```

Enable `tokio` for `TokioMonotonicClock` and `TokioAsyncSleeper`:

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

`ManualAsyncSleeper` and the manual coordination futures are executor-neutral;
they do not require the `tokio` feature. An executor is needed only to poll
them.

## 3. Implementation overview

| Capability | Real-time implementation | Deterministic implementation |
|---|---|---|
| Wall time | `StdWallClock` | `FixedWallClock`, `ManualWallClock` |
| Monotonic time | `StdMonotonicClock`, `TokioMonotonicClock` | `ManualMonotonicClock` |
| Blocking sleep | `StdBlockingSleeper` | `ManualBlockingSleeper` |
| Async sleep | `TokioAsyncSleeper` | `ManualAsyncSleeper` |

`Arc<T>` and `Box<T>` implement all four capability traits whenever `T` does.
Concrete shared values therefore coerce directly to `Arc<dyn WallClock>`,
`Arc<dyn MonotonicClock>`, `Arc<dyn BlockingSleeper>`, or
`Arc<dyn AsyncSleeper>` without adapters.

## 4. Wall clocks

### 4.1 System wall time

Use `StdWallClock` for production civil time:

```rust
use qubit_clock::{StdWallClock, WallClock};

let clock = StdWallClock::new();
let created_at = clock.now();
println!("created at {created_at:?}");
```

`StdWallClock` is zero-sized, `Copy`, and backed by `SystemTime::now()`.

### 4.2 Fixed wall time

Use `FixedWallClock` when every read should return one immutable timestamp:

```rust
use qubit_clock::{FixedWallClock, WallClock};
use std::time::{Duration, UNIX_EPOCH};

let expected = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
let clock = FixedWallClock::new(expected);
assert_eq!(expected, clock.now());
assert_eq!(expected, clock.fixed_time());
```

This is the smallest test double for code that only stamps records.

### 4.3 Wall time driven by manual monotonic time

Use `ManualWallClock` when wall time must advance together with manual
deadlines:

```rust
use qubit_clock::{ManualMonotonicClock, WallClock};
use std::time::{Duration, UNIX_EPOCH};

let clock = ManualMonotonicClock::new_shared();
let wall_clock = clock.new_wall_clock(UNIX_EPOCH);

clock
    .advance(Duration::from_secs(60))
    .expect("manual time should advance");
assert_eq!(UNIX_EPOCH + Duration::from_secs(60), wall_clock.now());

wall_clock.reanchor(UNIX_EPOCH + Duration::from_secs(3_600));
assert_eq!(UNIX_EPOCH + Duration::from_secs(3_600), wall_clock.now());
```

`reanchor()` changes only the wall-time mapping. It can move the wall reading
forward or backward, but it does not move manual monotonic time and does not
wake sleepers. `ManualWallClock::now()` panics only if the anchor plus elapsed
manual time is outside the platform's representable `SystemTime` range.

## 5. Monotonic clocks, domains, and instants

### 5.1 Standard monotonic time

`StdMonotonicClock` maps `std::time::Instant` onto a private origin:

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock};

let clock = StdMonotonicClock::new();
let start = clock.now();
let end = clock.now();
let elapsed = end
    .duration_since(start)
    .expect("both instants should share one clock domain");
println!("elapsed: {elapsed:?}");
```

Each newly constructed monotonic clock owns a distinct `ClockDomain`.
`MonotonicInstant` carries that domain and an elapsed duration from the clock's
private origin. Its principal operations are:

- `domain()` — identify the originating timeline;
- `elapsed_since_origin()` — inspect the domain-relative duration;
- `checked_add(duration)` — form a same-domain deadline;
- `duration_since(earlier)` — measure a same-domain interval.

Instants from different domains cannot be used together. `duration_since`
returns `TimeError::ClockDomainMismatch`, and `PartialOrd` returns `None`, for
cross-domain values. This prevents a deadline sampled from one clock from being
silently passed to another sleeper.

### 5.2 Manual monotonic time

`ManualMonotonicClock` starts at elapsed duration zero and changes only through
the test control plane:

```rust
use qubit_clock::{ManualMonotonicClock, MonotonicClock};
use std::time::Duration;

let clock = ManualMonotonicClock::new_shared();
assert_eq!(Duration::ZERO, clock.now().elapsed_since_origin());

clock
    .advance(Duration::from_secs(2))
    .expect("manual time should advance");
assert_eq!(Duration::from_secs(2), clock.now().elapsed_since_origin());

let target = clock
    .now()
    .checked_add(Duration::from_secs(3))
    .expect("the target should be representable");
clock
    .advance_to(target)
    .expect("the target should be in the same domain and not earlier");
assert_eq!(Duration::from_secs(5), clock.now().elapsed_since_origin());
```

`advance(duration)` performs a relative forward move. `advance_to(target)`
performs an absolute same-domain move. Moving backward returns
`TimeError::CannotMoveBackward`; passing a foreign target returns
`TimeError::ClockDomainMismatch`. A zero-duration advance is a successful
no-op and emits no advance notification.

## 6. Blocking sleepers

### 6.1 Real blocking sleep

`StdBlockingSleeper::new()` creates a sleeper and its own paired standard
monotonic clock:

```rust
use qubit_clock::{BlockingSleeper, StdBlockingSleeper};
use std::time::Duration;

let sleeper = StdBlockingSleeper::new();
sleeper
    .sleep_for(Duration::from_millis(10))
    .expect("the blocking sleep should complete");
```

If another component must retain the exact clock identity, construct both from
one `Arc`:

```rust
use qubit_clock::{StdBlockingSleeper, StdMonotonicClock};
use std::sync::Arc;

let clock = Arc::new(StdMonotonicClock::new());
let sleeper = StdBlockingSleeper::from_clock(Arc::clone(&clock));
```

### 6.2 Deterministic blocking sleep

For tests, keep the shared manual clock and derive the sleeper from it:

```rust
use qubit_clock::{BlockingSleeper, ManualMonotonicClock};
use std::time::Duration;

let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_blocking_sleeper();
let worker = std::thread::spawn(move || {
    sleeper.sleep_for(Duration::from_secs(10))
});

let observed = clock
    .wait_for_next_deadline(Duration::from_secs(1))
    .expect("worker should register a deadline");
assert_eq!(Duration::from_secs(10), observed.elapsed_since_origin());

clock
    .advance_to_next_deadline()
    .expect("the worker deadline should remain active");
worker
    .join()
    .expect("worker should not panic")
    .expect("manual sleep should complete");
```

The timeout passed to `wait_for_next_deadline` is a real-time test guard. It
never advances logical time.

## 7. Tokio asynchronous sleep

`TokioAsyncSleeper` follows Tokio's time driver and requires the `tokio`
feature:

```rust
use qubit_clock::{AsyncSleeper, TokioAsyncSleeper};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), qubit_clock::TimeError> {
let sleeper = TokioAsyncSleeper::new();
sleeper.sleep_for_async(Duration::from_millis(10)).await?;
Ok(())
}
```

Creating the returned `SleepFuture` is lazy and does not itself require a
runtime. Its first poll requires a Tokio runtime with time enabled.

When Tokio time is paused or explicitly advanced, create and read the paired
`TokioMonotonicClock` and poll all sleeper futures under the same runtime time
driver. Tasks may move between worker threads of that runtime, but the
clock/sleeper pair must not move between independent runtimes. Driver identity
is a caller contract and cannot be checked by this crate.

Use `TokioAsyncSleeper::from_clock(Arc<TokioMonotonicClock>)` only when another
component must retain the same clock identity. Otherwise `new()` is simpler.

## 8. Build one manual-time family

The recommended manual setup has one control-plane clock and derives all
consumer capabilities from it:

```rust
use qubit_clock::{AsyncSleeper, BlockingSleeper, ManualMonotonicClock, WallClock};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

let clock = ManualMonotonicClock::new_shared();
let wall_clock = clock.new_wall_clock(UNIX_EPOCH);
let blocking_sleeper = clock.new_blocking_sleeper();
let async_sleeper = clock.new_async_sleeper();

let injected_wall: Arc<dyn WallClock> = wall_clock;
let injected_blocking: Arc<dyn BlockingSleeper> = blocking_sleeper;
let injected_async: Arc<dyn AsyncSleeper> = async_sleeper;
```

The factory methods return concrete `Arc<Manual...>` values, so tests retain
manual-only APIs while production builders receive ordinary trait objects.
Every derived component shares the exact clock allocation and domain. Calling a
factory more than once creates independent lightweight wrappers over the same
timeline; it does not create a new clock.

The lower-level `ManualWallClock::from_clock`,
`ManualBlockingSleeper::from_clock`, and `ManualAsyncSleeper::from_clock`
constructors remain available. Prefer the clock factories in normal tests
because they make same-domain assembly evident and avoid repeated `Arc::new`
and `Arc::clone` plumbing.

## 9. How manual sleep futures behave

`ManualAsyncSleeper` deliberately registers a waiter when
`sleep_for_async()` or `sleep_until_async()` is called, before the returned
future is first polled. Consequently:

- an unpolled sleep future is visible to `pending_waiters`, deadline observers,
  and `advance_to_next_deadline`;
- a relative deadline is fixed at the sleep method call, not at first poll;
- advancing to the deadline before first poll makes the first poll ready;
- dropping an incomplete future immediately unregisters its waiter;
- a foreign deadline produces an immediately ready error future.

This eager registration is specific to manual sleep and makes deterministic
coordination possible. `TokioAsyncSleeper` creates its native timer on first
poll instead.

`SleepFuture` is the object-safe return type shared by async sleeper
implementations:

```rust
type SleepFuture = Pin<Box<dyn Future<Output = Result<(), TimeError>> + Send + 'static>>;
```

Callers normally use the alias through `AsyncSleeper` and do not name it.

## 10. Count coordination versus deadline coordination

The manual clock exposes two intentionally different kinds of synchronization:

| API | Semantics | Best use |
|---|---|---|
| `pending_waiters()` | Current registration count; due async waiters remain counted until repolled or dropped | Diagnostics and assertions |
| `wait_for_waiters(count, real_timeout)` | Blocking, latched count threshold | Startup barriers for a known number of waiters |
| `wait_for_waiters_async(count)` | Async, latched count threshold | Async startup barriers |
| `next_deadline()` | Snapshot of the earliest active strictly future deadline | Inspection |
| `wait_for_next_deadline(real_timeout)` | Blocking wait for current active future deadline | Blocking multi-stage drivers |
| `wait_for_next_deadline_async()` | Async state observer for current active future deadline | Async multi-stage drivers |
| `advance_to_next_deadline()` | Atomically select and advance to the current earliest future deadline | Driving logical time |

A count observer is event-like: once its threshold has been reached, completion
stays latched even if a waiter disappears before the observer is polled again.
A reached async waiter may still contribute to the count until its future is
repolled or dropped.

A deadline observer is state-based: cancelled and already-due waiters do not
satisfy it. This distinction matters in retry loops and other multi-stage
operations, where the previous stage's due waiter can still be registered while
the next stage is being created.

## 11. Exact semantics of `wait_for_next_deadline_async`

`wait_for_next_deadline_async()` returns `ManualDeadlineFuture` and follows
these rules:

1. Calling the method registers an observer immediately, before first poll.
   This closes the lost-wakeup window between creating the observer and polling
   the producer.
2. Registration does not reserve or latch a waiter. Every poll examines current
   state while holding the manual clock mutex.
3. If active waiters exist, the poll returns the earliest deadline strictly
   later than current manual time and removes the observer.
4. If no active future deadline exists, the poll stores or updates the task
   waker and returns `Pending`.
5. Registering a future waiter wakes pending observers, but a waiter cancelled
   before the observer's next poll is ignored.
6. Waiters whose deadlines are already due are also ignored, even if their
   futures have not yet cleaned up their registrations.
7. Dropping a pending `ManualDeadlineFuture` unregisters the observer.

The ready instant is a snapshot chosen under the state lock, not a reservation.
Another task can register an earlier deadline immediately afterward. Use the
observer to synchronize, then call `advance_to_next_deadline()` to atomically
choose and advance to whatever deadline is earliest at that moment:

```rust
let _observed = clock.wait_for_next_deadline_async().await;
let reached = clock
    .advance_to_next_deadline()
    .expect("an active future deadline should remain");
```

Avoid `clock.advance_to(observed)` in a concurrent driver unless the test has
otherwise proved that no earlier deadline can be registered.

## 12. A multi-stage async driver

The operation under test must be polled concurrently with its deadline
observer. Spawning it or using `select!` both work. The following example uses
`select!` to drive two sequential sleeps:

```rust
use qubit_clock::{AsyncSleeper, ManualMonotonicClock, TimeError};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), TimeError> {
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_async_sleeper();
let mut operation = Box::pin(async move {
    sleeper.sleep_for_async(Duration::from_secs(2)).await?;
    sleeper.sleep_for_async(Duration::from_secs(3)).await?;
    Ok::<_, TimeError>("done")
});

for expected_elapsed in [Duration::from_secs(2), Duration::from_secs(5)] {
    tokio::select! {
        result = &mut operation => {
            panic!("operation completed before its deadline: {result:?}");
        }
        _ = clock.wait_for_next_deadline_async() => {}
    }

    let reached = clock
        .advance_to_next_deadline()
        .expect("the operation should have an active deadline");
    assert_eq!(expected_elapsed, reached.elapsed_since_origin());
}

assert_eq!("done", operation.await?);
Ok(())
}
```

Merely constructing an async block does not run it. If the operation is not
spawned or polled in `select!`, it cannot register its sleeper deadline and the
observer correctly remains pending.

## 13. Mixed blocking and async waiters

One manual clock can drive both sleeper kinds. `pending_waiters`,
`next_deadline`, and `advance_to_next_deadline` consider their combined
registrations. This is useful for testing a system where a worker thread and an
async task share one logical timeline:

```rust
let clock = ManualMonotonicClock::new_shared();
let blocking = clock.new_blocking_sleeper();
let asynchronous = clock.new_async_sleeper();
```

Keep the concrete `clock` in the test harness. Inject only the derived
capability each production component needs.

## 14. Advance subscriptions

`subscribe_advances` is for synchronization test doubles that maintain their
own condition variable, channel, or task-waker set:

```rust
use qubit_clock::ManualMonotonicClock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

let clock = ManualMonotonicClock::new_shared();
let notified = Arc::new(AtomicBool::new(false));
let callback_flag = Arc::clone(&notified);
let subscription = clock.subscribe_advances(move || {
    callback_flag.store(true, Ordering::SeqCst);
});

clock
    .advance(Duration::from_secs(1))
    .expect("manual time should advance");
assert!(notified.load(Ordering::SeqCst));
drop(subscription); // unregister future callbacks
```

Callbacks run synchronously after the clock mutex is released. They should be
idempotent, fast, and limited to signaling another waiting primitive.
Concurrent advances may invoke callbacks concurrently and in no defined order.
Retain the returned `ManualAdvanceSubscription` for as long as notifications
are needed. Dropping it prevents future registration, although a callback
already captured by an in-flight advance may still run once.

## 15. Trait-object injection

Application code should depend on the narrowest trait:

```rust
use qubit_clock::{AsyncSleeper, WallClock};
use std::sync::Arc;

struct SessionService {
    wall_clock: Arc<dyn WallClock>,
    sleeper: Arc<dyn AsyncSleeper>,
}
```

Production assembly can inject `StdWallClock` and `TokioAsyncSleeper`; tests can
inject a `ManualWallClock` and `ManualAsyncSleeper` derived from one manual
clock. The service does not need a test-only mode.

If implementing a custom `MonotonicClock`, allocate one `ClockDomain` when the
clock is constructed and reuse it for every `now()` result. Never allocate a
new domain per sample. A custom sleeper must validate deadlines against and
report the same stable domain through `clock()`.

## 16. Error handling

`TimeError` is `#[non_exhaustive]`; include a wildcard arm when matching it:

```rust
use qubit_clock::TimeError;

fn classify(error: TimeError) -> &'static str {
    match error {
        TimeError::ClockDomainMismatch { .. } => "wrong clock",
        TimeError::InstantOverflow => "deadline overflow",
        TimeError::CannotMoveBackward => "backward manual advance",
        TimeError::InvalidInstantOrder => "reversed interval",
        _ => "other time error",
    }
}
```

The variants mean:

- `ClockDomainMismatch` — a deadline or instant came from another monotonic
  clock;
- `InstantOverflow` — duration arithmetic or native deadline conversion is not
  representable;
- `CannotMoveBackward` — `ManualMonotonicClock::advance_to` received an earlier
  same-domain target;
- `InvalidInstantOrder` — `duration_since` received an instant later than the
  receiver.

## 17. Recommended patterns and common mistakes

- Use `WallClock` for externally meaningful timestamps and `MonotonicClock` or
  a sleeper for elapsed time.
- In manual tests, start with `ManualMonotonicClock::new_shared()` and use its
  `new_wall_clock`, `new_blocking_sleeper`, and `new_async_sleeper` factories.
- Keep the concrete manual clock only in the test harness; inject narrow trait
  objects into production components.
- Use waiter-count coordination for startup barriers with a known count.
- Use deadline coordination for retries, repeated timeouts, and multi-stage
  workflows.
- After observing a deadline, prefer `advance_to_next_deadline()` over
  `advance_to(observed)` so selection and advancement are atomic.
- Ensure the async operation under test is actually being polled.
- Do not compare, order, or subtract instants from different clock domains.
- Do not move a paused-time Tokio clock/sleeper pair across independent
  runtimes.
- Retain advance-subscription handles; dropping one unregisters it.
- Treat real-time timeout parameters on blocking coordination methods as test
  guards, never as logical time.

Following these patterns keeps production APIs small while giving tests one
explicit, deterministic control plane for every time-dependent component.
