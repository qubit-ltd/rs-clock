# Qubit Clock User Guide

Time-dependent code often hides its most important dependency. A component
that calls `SystemTime::now()`, `Instant::now()`, or a sleep function directly
has already chosen its clock and waiting mechanism. Unit tests then have to
wait in real time, race the scheduler, or add mock-only branches to production
code.

`qubit-clock` provides clock and timer traits designed for dependency injection
(IoC). Components depend on the capability they need; an application
composition root supplies a standard or Tokio implementation in production,
while tests supply fixed or manually advanced time. The component itself is
unchanged.

## A first example

Consider a session that expires after a configured duration. It needs a
monotonic clock because expiration is elapsed-time behavior and must not be
affected by wall-clock adjustments:

```rust
use qubit_clock::{
    ManualMonotonicClock, MonotonicClock, MonotonicInstant, StdMonotonicClock,
    TimeError,
};
use std::{sync::Arc, time::Duration};

struct Session {
    clock: Arc<dyn MonotonicClock>,
    expires_at: MonotonicInstant,
}

impl Session {
    fn new(
        clock: Arc<dyn MonotonicClock>,
        ttl: Duration,
    ) -> Result<Self, TimeError> {
        let expires_at = clock.now().checked_add(ttl)?;
        Ok(Self { clock, expires_at })
    }

    fn is_expired(&self) -> bool {
        self.clock.now() >= self.expires_at
    }
}

fn main() -> Result<(), TimeError> {
    let _production = Session::new(
        Arc::new(StdMonotonicClock::new()),
        Duration::from_secs(30),
    )?;

    let clock = ManualMonotonicClock::new_shared();
    let session = Session::new(clock.clone(), Duration::from_secs(30))?;
    assert!(!session.is_expired());

    clock.advance(Duration::from_secs(30))?;
    assert!(session.is_expired());
    Ok(())
}
```

The production assembly uses `StdMonotonicClock`. The test retains
`ManualMonotonicClock` as its control plane and injects another reference into
the session. Advancing logical time reaches the exact expiration boundary
immediately. There is no sleep, global clock override, mock flag, or duplicate
expiration algorithm.

## The injection pattern

Use the narrowest dependency that expresses the component's actual behavior:

- Inject `Arc<dyn WallClock>` when the component creates externally meaningful
  timestamps.
- Inject `Arc<dyn MonotonicClock>` when it samples elapsed time or checks
  deadlines without waiting.
- Inject `Arc<dyn Timer>` when it must asynchronously wait for a deadline. A
  timer also exposes its same-domain monotonic clock.
- Construct `BlockingSleeper` from an injected timer when synchronous code must
  block.

Keep concrete types in the composition root. Production assembly chooses
`Std*` or `Tokio*`; tests choose `FixedWallClock` or a capability derived from
`ManualMonotonicClock`. This is ordinary dependency injection, not a separate
test mode.

## Components

| Need | API | Production | Deterministic test |
|---|---|---|---|
| Externally meaningful timestamps | `WallClock` | `StdWallClock` | `FixedWallClock`, `ManualWallClock` |
| Monotonic instants | `MonotonicClock` | `StdMonotonicClock`, `TokioMonotonicClock` | `ManualMonotonicClock` |
| Async deadlines | `Timer` | `StdTimer`, `TokioTimer` | `ManualTimer` |
| Blocking waits | `BlockingSleeper` | compose a timer with independent progress | compose an externally driven `ManualTimer` |

Wall-clock values may jump. Use them for externally meaningful timestamps.
Monotonic instants belong to a private clock domain and must be used for
timeouts, retry delays, and elapsed-time measurements.

### `WallClock`

`WallClock::now()` returns `SystemTime` for values that leave the process or
need a calendar meaning: creation times, audit records, protocol timestamps,
and persisted metadata. Use `StdWallClock` in production, `FixedWallClock` when
one constant value is enough for a test, and `ManualWallClock` when the value
must move together with a manual monotonic timeline. Do not use wall-clock time
to measure elapsed duration because it may jump.

### `MonotonicClock`

`MonotonicClock::now()` returns a domain-scoped `MonotonicInstant` that never
moves backward within that domain. Use it for expiration, elapsed-time budgets,
retry policies, and timeout calculations. Independent clock domains must not be
mixed. Every monotonic clock can create a `Timer` in its own domain with
`new_timer()`.

### `Timer`

`Timer` turns a monotonic deadline into a Future. `after(duration)` fixes a
relative deadline when called; `at(instant)` accepts an absolute deadline from
the same clock domain. Inject it into asynchronous components that need to wait,
race an operation against a timeout, or schedule retry delays.

### `BlockingSleeper`

`BlockingSleeper` adapts a `Timer` for synchronous code by polling its Future
and parking the calling thread. It does not create another timing model: the
injected timer still owns deadline calculation and progress.

## Installation

The standard and manual implementations are available with the default feature
set:

```toml
[dependencies]
qubit-clock = "0.11"
```

Enable `tokio` when production code needs `TokioMonotonicClock` or
`TokioTimer`:

```toml
[dependencies]
qubit-clock = { version = "0.11", features = ["tokio"] }
```

Manual clocks, manual timers, and their coordination futures are
executor-neutral and do not require this feature.

## Using `Timer`

Every monotonic clock creates a same-domain timer without consuming the clock:

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock, Timer};
use std::time::Duration;

# async fn example() -> Result<(), qubit_clock::TimeError> {
let clock = StdMonotonicClock::new();
let timer = clock.new_timer();
timer.after(Duration::from_millis(10))?.await?;
let _still_usable = clock.now();
# Ok(())
# }
```

`Timer::after` samples the clock and fixes its absolute deadline during the
call. `Timer::at` accepts an absolute `MonotonicInstant` and also fixes that
deadline before returning. The returned `TimerFuture` has output
`Result<(), TimeError>` and waits only for the fixed deadline; a backend may
enroll it with a native scheduler when the future is first polled. Dropping an
incomplete future cancels the outstanding notification.

`StdTimer` wakes futures owned by an unexpectedly exited scheduler-worker
generation. Their next poll returns `TimerUnavailable` with
`SchedulerWorkerTerminated` instead of remaining pending or reporting deadline
completion. A later registration starts a replacement worker generation.

## Tokio timer

Enable the `tokio` feature for `TokioMonotonicClock` and `TokioTimer`.
Both types retain a Tokio runtime `Handle`. `current()` and `try_current()`
capture the ambient handle during construction; `try_current()` returns
`TokioRuntimeError::NotEntered` when there is no ambient runtime. Prefer
`from_handle(handle)` at dependency-injection boundaries:

```rust
use qubit_clock::{Timer, TokioTimer};
use std::time::Duration;

let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_time()
    .build()
    .expect("Tokio runtime should build");
let timer = TokioTimer::from_handle(runtime.handle().clone());
let deadline = timer
    .after(Duration::from_millis(1))
    .expect("deadline should register on the retained runtime");
runtime
    .block_on(deadline)
    .expect("deadline should complete");
```

Clock samples and `Sleep` creation briefly enter the retained handle, regardless
of the caller's ambient runtime. A returned timer future may therefore be
polled by another thread or runtime context. Deadline progress still belongs to
the target runtime: its `Runtime` owner must remain alive and its time driver
must be driven until the future completes. A future deadline on a runtime
without time enabled returns `TimerUnavailableError::TimeDriverDisabled`.
An already reached deadline returns an immediately ready future and needs no
time driver. Dropping a pending future cancels that wait.

Tokio `Sleep` uses millisecond-level scheduling granularity rather than
high-resolution timing. `TokioTimer` preserves the full logical `Duration`,
but completion timing inherits that granularity and any additional platform
scheduling delay.

Tokio currently provides no public `Handle` capability query for the time
driver. It reports a disabled driver by panicking during future-sleep creation.
`TokioTimer` uses `catch_unwind` to convert that condition into
`TimeDriverDisabled` when unwinding is enabled, but the process panic hook runs
before the catch and can still log or observe it. A `panic = "abort"` build
cannot recover. Replacing the process-global hook temporarily would race with
application panic handling, so the library does not do so. Enabling time on
every runtime injected into `TokioTimer` is the only fully side-effect-free
configuration with the current public Tokio API.

Downstream tests can enable the default-off `test-util` feature in a development
dependency. `FaultInjectingTimer` then provides reusable, runtime-neutral
registration and completion failures while preserving normal domain and
reached-deadline behavior.

<a id="manual-time-coordination"></a>

## Deterministic manual time

Keep the manual clock as the test control plane and inject its timer into the
component under test:

Manual timers and their coordination futures are executor-neutral and do not
require the `qubit-clock/tokio` feature. The examples in this section choose
Tokio only to run, spawn, and select tasks. To run them in tests, declare Tokio
as a direct development dependency:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
```

Deterministic manual time controls logical time, deadline selection, and
deadline completion. It does not define the wake order of waiters sharing one
deadline or the order in which an executor polls ready tasks.

```rust
use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::time::Duration;

# #[tokio::main(flavor = "current_thread")]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let task = tokio::spawn(async move {
    timer.after(Duration::from_secs(8))?.await?;
    Ok::<_, qubit_clock::TimeError>(())
});

let reached = clock.advance_to_next_deadline_async().await;
assert_eq!(Duration::from_secs(8), reached.elapsed_since_origin());
task.await??;
# Ok(())
# }
```

Manual timer registration is eager: `pending_waiters`, `next_deadline`, and
the coordination APIs can observe a returned timer future before its first
poll. Completion remains latched if manual time reaches the deadline first.

- `pending_waiters()` and `next_deadline()` are immediate snapshots.
- `wait_for_waiters()` and `wait_for_waiters_async()` are count barriers. Once
  their threshold is observed, completion is latched even if a waiter is then
  cancelled.
- `wait_for_next_deadline()` and `wait_for_next_deadline_async()` observe the
  earliest active deadline strictly later than current manual time. Their
  return value is a snapshot and must not be used as an atomic advance target.
- `advance_to_next_deadline()` atomically selects and reaches the current
  earliest future deadline, returning `None` when none exists.
- `advance_to_next_deadline_after_waiters()` blocks until the current waiter
  count reaches its threshold and a future deadline exists, then checks both
  conditions and advances under the same clock-state lock. This avoids a
  cancellation gap between a count observation and the advance.
- `advance_to_next_deadline_async()` waits until an active future deadline
  exists, then performs that atomic advance. If cancellation wins the race, it
  waits again. Cancelling the driver future does not advance manual time.

### Driving multiple stages

For a task that registers several deadlines in sequence, race its completion
against the manual-time driver. This avoids guessing how many stages the task
will execute and avoids waiting for another deadline after it has completed:

```rust
# use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
# use std::time::Duration;
# #[tokio::main(flavor = "current_thread")]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let mut task = tokio::spawn(async move {
    timer.after(Duration::from_secs(1))?.await?;
    timer.after(Duration::from_secs(2))?.await?;
    Ok::<_, qubit_clock::TimeError>(())
});

loop {
    tokio::select! {
        result = &mut task => {
            result??;
            break;
        }
        _ = clock.advance_to_next_deadline_async() => {}
    }
}
# Ok(())
# }
```

### Runtime ownership and cancellation

Manual coordination futures are runtime-neutral: they use ordinary Rust
futures and can be polled by any executor. Cancelling an observer or driver
future removes only that observation; it does not cancel timer waiters.
`TokioMonotonicClock` and `TokioTimer` instead retain an explicit runtime
capability. Their samples and timer registrations use that handle, while the
returned future may be polled in another execution context. Cancelling it
removes the Tokio sleep; moving it does not transfer deadline ownership to the
polling runtime. The retained target runtime must remain alive and driven.

## Wall-clock projection and reanchoring

A manual wall clock projects wall-clock time from the shared monotonic timeline:

```rust
use qubit_clock::{ManualMonotonicClock, WallClock};
use std::time::{Duration, UNIX_EPOCH};

let clock = ManualMonotonicClock::new_shared();
let wall_clock = clock.new_wall_clock(UNIX_EPOCH);
clock.advance(Duration::from_secs(5))?;
assert_eq!(UNIX_EPOCH + Duration::from_secs(5), wall_clock.now());

wall_clock.reanchor(UNIX_EPOCH + Duration::from_secs(100));
assert_eq!(UNIX_EPOCH + Duration::from_secs(100), wall_clock.now());
# Ok::<_, qubit_clock::TimeError>(())
```

`reanchor` changes only the wall-time mapping. It does not move the monotonic
timeline or alter deadlines and timer registrations.

## Blocking adaptation

`BlockingSleeper` is a concrete adapter, not another scheduling abstraction:

```rust
use qubit_clock::{BlockingSleeper, MonotonicClock, StdMonotonicClock};
use std::time::Duration;

let clock = StdMonotonicClock::new();
let sleeper = BlockingSleeper::new(clock.new_timer());
sleeper.sleep_for(Duration::from_millis(10))?;
# Ok::<_, qubit_clock::TimeError>(())
```

The adapter polls the timer future and parks only the calling thread. Once that
thread parks, the timer backend must still be able to make progress. `StdTimer`
has its own scheduler worker. `ManualTimer` requires another thread or test
controller to advance the clock. `TokioTimer` requires its retained runtime to
be driven independently; never block the sole driver thread of a
current-thread runtime while waiting on that runtime's timer.

## Use in related libraries

`rs-lock` injects timers into timeout-aware monitor implementations. Production
waits use a standard or Tokio timer; tests inject a timer created by
`ManualMonotonicClock` and advance it to the timeout without an eight-second
wall-clock delay.

`rs-retry` uses an injected timer's monotonic clock for elapsed-time budgets and
the same timer for retry delays and attempt timeouts. Its tests can therefore
cover exponential backoff, exact timeout boundaries, and cancellation without
waiting in real time.

These libraries are larger applications of the same pattern shown by
`Session`: depend on a trait, select the concrete clock at the composition root,
and let tests retain the manual control plane.

## Benchmarking

The process-wide standard timer scheduler benchmark is available with:

```bash
cargo bench --bench std_timer_scheduler
cargo bench --bench manual_timer
cargo bench --bench tokio_timer --features tokio
```

The standard-timer benchmark reports registration/cancellation and
deadline-completion throughput for 1, 2, 4, 8, and 16 concurrent caller
threads. The manual-timer benchmark measures registration/cancellation, batch
completion, and sequential completion at 1, 8, 32, 128, and 1,024 waiters. The
Tokio benchmark compares native sleeps, the retained legacy per-deadline
sentinel, and the shared-sentinel implementation at 1,024 and 10,240 pending
deadlines.

The synchronization state machines used by `BlockingSleeper`, `StdTimer`, and
the manual timer registry also have Loom models that exercise their production
algorithms. Run every model with:

```bash
RUSTFLAGS="--cfg loom" cargo test --release --all-features loom
```

Loom model test names contain `loom`. The shared CI check discovers that set
before execution and fails when it discovers no models, preventing an empty
filter from silently passing.

## Errors

- `ClockDomainMismatch`: a deadline came from another monotonic domain.
- `InstantOverflow`: relative or native deadline conversion overflowed.
- `TimerUnavailable { source }`: deadline registration or completion failed
  because the scheduler worker, time driver, or custom backend was unavailable.
  `TimerUnavailableError` identifies the backend and preserves its available
  source error.
- `TokioRuntimeError`: `try_current()` could not capture an ambient Tokio
  runtime. Explicit `from_handle` construction avoids this failure boundary.
- `CannotMoveBackward`: manual time was moved backward.
- `InvalidInstantOrder`: instant arithmetic used an invalid order.

The public error enums are `#[non_exhaustive]`. Match the variants you can
handle and keep a fallback arm so later backend errors remain source
compatible.
