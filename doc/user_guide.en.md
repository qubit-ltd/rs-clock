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
| Blocking waits | `BlockingSleeper` | compose any `Timer` | compose `ManualTimer` |

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

## Tokio timer

Enable the `tokio` feature for `TokioMonotonicClock` and `TokioTimer`.
Creating a future deadline must occur inside a runtime with time enabled. A
missing runtime returns `TimeError::TimerUnavailable` with
`TimerUnavailableReason::RuntimeNotEntered`; a disabled time driver reports
`TimerUnavailableReason::TimeDriverDisabled`. An already reached deadline
returns a ready future without runtime access. `TokioTimer` fixes the `Sleep`
deadline during the call, while Tokio may enroll that sleep with its time
driver on first poll. Create the clock and future, advance paused time, and poll
the future under the same runtime time driver.

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

### Runtime affinity and cancellation

Manual coordination futures are runtime-neutral: they use ordinary Rust
futures and can be polled by any executor. Cancelling an observer or driver
future removes only that observation; it does not cancel timer waiters.
`TokioMonotonicClock` and `TokioTimer` are different: create their future
deadlines and poll them under the same Tokio time driver that supplied the
clock's origin.

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

The adapter polls the timer future and parks only the calling thread. The same
adapter works with a manual timer, so blocking integration tests also avoid
real delays.

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

## Errors

- `ClockDomainMismatch`: a deadline came from another monotonic domain.
- `InstantOverflow`: relative or native deadline conversion overflowed.
- `TimerUnavailable { reason }`: deadline registration failed because the
  scheduler worker, async runtime, time driver, or custom backend was
  unavailable. `TimerUnavailableReason` identifies the specific cause.
- `CannotMoveBackward`: manual time was moved backward.
- `InvalidInstantOrder`: instant arithmetic used an invalid order.
