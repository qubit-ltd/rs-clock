# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Injectable wall clocks, monotonic clocks, and deterministic timers for Rust.

Detailed documentation:

- [English User Guide](doc/user_guide.en.md)
- [中文用户手册](doc/user_guide.zh_CN.md)
- [API documentation](https://docs.rs/qubit-clock)

## Choose a capability

| Need | Trait | Real time | Deterministic tests |
|---|---|---|---|
| Civil timestamps | `WallClock` | `StdWallClock` | `FixedWallClock`, `ManualWallClock` |
| Elapsed time and deadlines | `MonotonicClock` | `StdMonotonicClock`, `TokioMonotonicClock` | `ManualMonotonicClock` |
| Async deadlines | `Timer` | `StdTimer`, `TokioTimer` | `ManualTimer` |
| Blocking waits | `BlockingSleeper` adapter | compose a real timer | compose a manual timer |

Wall time may jump and is intended for externally meaningful timestamps.
Monotonic time never moves backward within one clock domain and is intended for
elapsed time, retries, and timeouts. Every clock creates a same-domain timer
directly with `clock.new_timer()`.

## Installation

```toml
[dependencies]
qubit-clock = "0.9"
```

Enable the Tokio-backed clock and timer when required:

```toml
[dependencies]
qubit-clock = { version = "0.9", features = ["tokio"] }
```

Manual timers and manual coordination are runtime-neutral.

## Real-time use

```rust
use qubit_clock::{BlockingSleeper, MonotonicClock, StdMonotonicClock, StdWallClock, WallClock};
use std::time::Duration;

let wall_clock = StdWallClock::new();
let clock = StdMonotonicClock::new();
let sleeper = BlockingSleeper::new(clock.new_timer());

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
use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
let clock = ManualMonotonicClock::new_shared();
let timer = clock.new_timer();
let task = tokio::spawn(async move {
    timer.after(Duration::from_secs(5))?.await;
    Ok::<_, qubit_clock::TimeError>(())
});

let observed = clock.wait_for_next_deadline_async().await;
assert_eq!(Duration::from_secs(5), observed.elapsed_since_origin());

clock
    .advance_to_next_deadline()
    .expect("the active timer should have a future deadline");
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
multi-stage examples, Tokio runtime affinity, wall reanchoring, trait-object
injection, and errors.

## License

Licensed under the Apache License, Version 2.0.
