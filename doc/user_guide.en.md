# Qubit Clock User Guide

`qubit-clock` separates civil time from monotonic scheduling and makes both
injectable. Applications use the same production code with real or manual time;
tests advance logical time instead of waiting.

## Capabilities

| Need | API | Production | Deterministic test |
|---|---|---|---|
| Civil timestamps | `WallClock` | `StdWallClock` | `FixedWallClock`, `ManualWallClock` |
| Monotonic instants | `MonotonicClock` | `StdMonotonicClock`, `TokioMonotonicClock` | `ManualMonotonicClock` |
| Async deadlines | `Timer` | `StdTimer`, `TokioTimer` | `ManualTimer` |
| Blocking waits | `BlockingSleeper` | compose a timer with independent progress | compose an externally driven `ManualTimer` |

Wall-clock values may jump. Use them for externally meaningful timestamps.
Monotonic instants belong to a private clock domain and must be used for
timeouts, retry delays, and elapsed-time measurements.

## Creating a timer

Every monotonic clock creates a same-domain timer without consuming the clock:

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock, Timer};
use std::time::Duration;

# async fn example() -> Result<(), qubit_clock::TimeError> {
let clock = StdMonotonicClock::new();
let timer = clock.new_timer();
timer.after(Duration::from_millis(10))?.await;
let _still_usable = clock.now();
# Ok(())
# }
```

`Timer::after` samples the clock and fixes its absolute deadline during the
call. `Timer::at` accepts an absolute `MonotonicInstant` and also fixes that
deadline before returning. The returned `TimerFuture` has output `()` and waits
only for the fixed deadline; a backend may enroll it with a native scheduler
when the future is first polled. Dropping an incomplete future cancels the
outstanding notification.

`StdTimer` treats an unexpected scheduler-worker exit as a fail-fast condition.
It wakes futures owned by the exited worker generation; their next poll panics
instead of remaining pending or reporting that their deadlines completed. A
later registration starts a replacement worker generation.

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
runtime.block_on(deadline);
```

Clock samples and `Sleep` creation briefly enter the retained handle, regardless
of the caller's ambient runtime. A returned timer future may therefore be
polled by another thread or runtime context. Deadline progress still belongs to
the target runtime: its `Runtime` owner must remain alive and its time driver
must be driven until the future completes. A future deadline on a runtime
without time enabled returns `TimerUnavailableError::TimeDriverDisabled`.
An already reached deadline returns an immediately ready future and needs no
time driver. Dropping a pending future cancels that wait.

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
    timer.after(Duration::from_secs(8))?.await;
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
    timer.after(Duration::from_secs(1))?.await;
    timer.after(Duration::from_secs(2))?.await;
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

A manual wall clock projects civil time from the shared monotonic timeline:

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

## IoC assembly

Application components should depend on `Arc<dyn WallClock>`,
`Arc<dyn MonotonicClock>`, or `Arc<dyn Timer>` according to their actual need.
Production assembly injects standard or Tokio implementations. Integration
tests inject a manual clock's timer and drive that clock explicitly. No test
mode or mock-specific branch is required in application code.

## Benchmarking

The process-wide standard timer scheduler benchmark is available with:

```bash
cargo bench --bench std_timer_scheduler
```

The benchmark reports registration/cancellation and deadline-completion
throughput for 1, 2, 4, 8, and 16 concurrent caller threads.

The small synchronization state machines used by `BlockingSleeper` and
`StdTimer` also have Loom model checks. Run them with:

```bash
RUSTFLAGS="--cfg loom" cargo test --release --test sleep_tests notification_latch_model
RUSTFLAGS="--cfg loom" cargo test --release --test timer_tests std_timer_waiter_model
```

## Errors

- `ClockDomainMismatch`: a deadline came from another monotonic domain.
- `InstantOverflow`: relative or native deadline conversion overflowed.
- `TimerUnavailable { source }`: deadline registration failed because the
  scheduler worker, time driver, or custom backend was
  unavailable. `TimerUnavailableError` identifies the backend and preserves
  its available source error.
- `TokioRuntimeError`: `try_current()` could not capture an ambient Tokio
  runtime. Explicit `from_handle` construction avoids this failure boundary.
- `CannotMoveBackward`: manual time was moved backward.
- `InvalidInstantOrder`: instant arithmetic used an invalid order.

The public error enums are `#[non_exhaustive]`. Match the variants you can
handle and keep a fallback arm so later backend errors remain source
compatible.
