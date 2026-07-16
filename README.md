# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Injectable wall clocks, monotonic clocks, and deterministic blocking or async
sleepers for Rust.

Detailed documentation:

- [English User Guide](doc/user_guide.en.md)
- [中文用户手册](doc/user_guide.zh_CN.md)
- [API documentation](https://docs.rs/qubit-clock)

## Choose a capability

| Need | Trait | Real time | Deterministic tests |
|---|---|---|---|
| Civil timestamps | `WallClock` | `StdWallClock` | `FixedWallClock`, `ManualWallClock` |
| Elapsed time and deadlines | `MonotonicClock` | `StdMonotonicClock`, `TokioMonotonicClock` | `ManualMonotonicClock` |
| Blocking waits | `BlockingSleeper` | `StdBlockingSleeper` | `ManualBlockingSleeper` |
| Async waits | `AsyncSleeper` | `TokioAsyncSleeper` | `ManualAsyncSleeper` |

Wall time may jump and is intended for externally meaningful timestamps.
Monotonic time never moves backward within one clock domain and is intended for
elapsed time, retries, and timeouts. Every sleeper owns and exposes its paired
monotonic clock, preventing deadline calculations from silently using another
timeline.

## Installation

```toml
[dependencies]
qubit-clock = "0.9"
```

Enable Tokio-backed clock and sleeper types when required:

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

Manual async sleep and manual coordination are runtime-neutral and do not
require the crate's `tokio` feature.

## Real-time use

```rust
use qubit_clock::{BlockingSleeper, StdBlockingSleeper, StdWallClock, WallClock};
use std::time::Duration;

let wall_clock = StdWallClock::new();
let sleeper = StdBlockingSleeper::new();

let started_at = wall_clock.now();
sleeper
    .sleep_for(Duration::from_millis(10))
    .expect("the blocking sleep should complete");
println!("started at {started_at:?}");
```

## Deterministic manual time

Keep one shared manual clock as the test control plane and derive all consumer
capabilities from it:

```rust
use qubit_clock::{AsyncSleeper, ManualMonotonicClock};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let sleeper = clock.new_async_sleeper();
let task = tokio::spawn(async move {
    sleeper.sleep_for_async(Duration::from_secs(5)).await
});

let observed = clock.wait_for_next_deadline_async().await;
assert_eq!(Duration::from_secs(5), observed.elapsed_since_origin());

clock
    .advance_to_next_deadline()
    .expect("the active sleep should have a future deadline");
task.await??;
Ok(())
}
```

`wait_for_next_deadline_async()` is a state observer. At each poll it returns
the current earliest strictly future active deadline; cancelled and already-due
waiters are ignored. The returned value is a snapshot, so concurrent drivers
should use `advance_to_next_deadline()` to select and advance atomically. The
[user guide](doc/user_guide.en.md#11-exact-semantics-of-wait_for_next_deadline_async)
documents the complete coordination contract, count-based barriers,
multi-stage examples, Tokio runtime affinity, wall reanchoring, subscriptions,
trait-object injection, and errors.

## License

Licensed under the Apache License, Version 2.0.
