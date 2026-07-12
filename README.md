# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Injectable wall clocks, monotonic clocks, and deterministic blocking or async sleepers for Rust.

## Design

Qubit Clock separates four capabilities:

- `WallClock` reads civil time as `SystemTime`.
- `MonotonicClock` reads domain-scoped `MonotonicInstant` values.
- `BlockingSleeper` blocks against a monotonic deadline.
- `AsyncSleeper` returns a future against a monotonic deadline.

Wall time may jump. Monotonic time never moves backward. Sleepers are always paired with a concrete monotonic clock and never maintain a second time source.

## Implementations

| Capability | Real time | Deterministic tests |
|---|---|---|
| Wall time | `StdWallClock` | `FixedWallClock`, `ManualWallClock` |
| Monotonic time | `StdMonotonicClock`, `TokioMonotonicClock` | `ManualMonotonicClock` |
| Blocking sleep | `StdBlockingSleeper` | `ManualBlockingSleeper` |
| Async sleep | `TokioAsyncSleeper` | `ManualAsyncSleeper` |

Tokio types require the optional `tokio` feature. Manual async sleep is runtime-neutral and available without that feature.

## Installation

```toml
[dependencies]
qubit-clock = "0.9"
```

Enable Tokio integration when required:

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

## Wall Time

```rust
use qubit_clock::{StdWallClock, WallClock};

let clock = StdWallClock::new();
let now = clock.now();
println!("Current wall time: {now:?}");
```

## Monotonic Time

```rust
use qubit_clock::{MonotonicClock, StdMonotonicClock};

let clock = StdMonotonicClock::new();
let start = clock.now();
let elapsed = clock
    .now()
    .duration_since(start)
    .expect("instants come from the same clock");
println!("Elapsed: {elapsed:?}");
```

## Deterministic Blocking Sleep

Shared clock identity is explicit through `Arc`:

```rust
use qubit_clock::{
    BlockingSleeper, ManualBlockingSleeper, ManualMonotonicClock,
};
use std::sync::Arc;
use std::time::Duration;

let clock = Arc::new(ManualMonotonicClock::new());
let sleeper = Arc::new(ManualBlockingSleeper::from_clock(
    Arc::clone(&clock),
));
let worker_sleeper = Arc::clone(&sleeper);

let worker = std::thread::spawn(move || {
    worker_sleeper
        .sleep_for(Duration::from_secs(10))
        .expect("manual sleep should complete");
});

assert!(sleeper.wait_for_waiters(1, Duration::from_secs(1)));
clock
    .advance(Duration::from_secs(10))
    .expect("manual time should advance");
worker.join().expect("worker should not panic");
```

## Manual Wall Time

`ManualWallClock` projects one manual monotonic timeline onto a wall-time anchor:

```rust
use qubit_clock::{
    ManualMonotonicClock, ManualWallClock, WallClock,
};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

let monotonic_clock = Arc::new(ManualMonotonicClock::new());
let wall_clock = ManualWallClock::from_clock(
    UNIX_EPOCH,
    Arc::clone(&monotonic_clock),
);

monotonic_clock
    .advance(Duration::from_secs(600))
    .expect("manual time should advance");
assert_eq!(UNIX_EPOCH + Duration::from_secs(600), wall_clock.now());

wall_clock.reanchor(UNIX_EPOCH);
assert_eq!(UNIX_EPOCH, wall_clock.now());
```

Reanchoring wall time never changes monotonic deadlines.
`ManualWallClock::now()` panics if the anchor plus manually advanced duration
exceeds the platform's representable `SystemTime` range.

## Unified Manual-Time Driver

One `ManualMonotonicClock` can drive blocking and async sleepers together.
`pending_waiters()` counts both kinds, `next_deadline()` inspects their earliest
future deadline, and `advance_to_next_deadline()` advances atomically to it.
Async test coordination can await
`ManualMonotonicClock::wait_for_waiters_async(&clock, expected_count)` without
polling or depending on a particular runtime.

## Manual Advance Notifications

Synchronization test doubles that must race their own notification with a
manual deadline can subscribe to clock advances:

```rust
use qubit_clock::ManualMonotonicClock;
use std::sync::Arc;

let clock = Arc::new(ManualMonotonicClock::new());
let subscription = ManualMonotonicClock::subscribe_advances(
    &clock,
    || {
        // Signal the test double's Condvar, watch channel, or task wakers.
    },
);
```

The callback runs synchronously outside the clock mutex. It should be
idempotent and only signal another waiting primitive. Concurrent advances may
invoke callbacks concurrently and without ordering. If callbacks panic, all
callbacks captured for that advance are attempted before the first panic is
resumed. Dropping `subscription` prevents later registrations, but one callback
already captured by an in-flight advance may still run.

## Documentation

- [Refactoring design](doc/clock_refactoring_design.zh_CN.md)
- [Implementation plan](doc/clock_refactoring_implementation_plan.zh_CN.md)
- [Downstream integration plan](doc/downstream_integration_implementation_plan.zh_CN.md)

## License

Licensed under the Apache License, Version 2.0.
