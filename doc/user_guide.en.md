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
call. `Timer::at` accepts an absolute `MonotonicInstant`. Both methods register
before returning. Registration failures are returned immediately; the returned
`TimerFuture` has output `()` and only represents deadline completion. Dropping
an incomplete future cancels its registration.

## Tokio timer

Enable the `tokio` feature for `TokioMonotonicClock` and `TokioTimer`.
Registration must occur inside a runtime with time enabled. A missing or
disabled driver returns `TimeError::TimerUnavailable` from `at` or `after`.
When time is paused, create the clock, register deadlines, advance time, and
poll futures under the same runtime time driver.

## Deterministic manual time

Keep the manual clock as the test control plane and inject its timer into the
component under test:

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

let deadline = clock.wait_for_next_deadline_async().await;
assert_eq!(Duration::from_secs(8), deadline.elapsed_since_origin());
clock.advance_to_next_deadline().expect("deadline should remain active");
task.await??;
# Ok(())
# }
```

Manual registrations are eager: `pending_waiters`, `next_deadline`,
`wait_for_waiters`, and `wait_for_next_deadline_async` observe a returned future
even before its first poll. Completion is latched if manual time reaches the
deadline before polling. `advance_to_next_deadline` atomically selects and
reaches the earliest strictly future active deadline.

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

## Errors

- `ClockDomainMismatch`: a deadline came from another monotonic domain.
- `InstantOverflow`: relative or native deadline conversion overflowed.
- `TimerUnavailable`: the timer driver or scheduler could not be created.
- `CannotMoveBackward`: manual time was moved backward.
- `InvalidInstantOrder`: instant arithmetic used an invalid order.
