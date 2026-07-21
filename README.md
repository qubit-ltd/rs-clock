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
| Blocking waits | `BlockingSleeper` adapter | compose a timer with independent progress | compose a manually driven timer |

Wall time may jump and is intended for externally meaningful timestamps.
Monotonic time never moves backward within one clock domain and is intended for
elapsed time, retries, and timeouts. Every clock creates a same-domain timer
directly with `clock.new_timer()`.

## Installation

```toml
[dependencies]
qubit-clock = "0.10"
```

Enable the Tokio-backed clock and timer when required:

```toml
[dependencies]
qubit-clock = { version = "0.10", features = ["tokio"] }
```

This feature exposes `TokioMonotonicClock` and `TokioTimer`. Manual timers and
manual coordination futures are executor-neutral and do not require it. The
async examples below choose Tokio only to run and spawn tasks. To copy them
into tests, declare Tokio directly:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
```

Tokio clocks and timers retain a runtime `Handle`. `current()` and
`try_current()` capture the ambient handle during construction;
`from_handle(handle)` supports explicit injection. Later clock samples and
timer registrations use that retained handle, so their futures may be polled
from another runtime context. The target runtime owner must remain alive and
its time driver must continue running until pending futures complete or are
dropped. If it shuts down first, a pending `TokioTimer` future returns
`TimeError::TimerUnavailable` with `TimerUnavailableError::RuntimeShuttingDown`.

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

Here, deterministic means that logical time, deadline selection, and deadline
completion are controlled explicitly. The wake order of waiters sharing one
deadline and the order in which an executor polls ready tasks are unspecified.

```rust
use qubit_clock::{ManualMonotonicClock, MonotonicClock, Timer};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clock = ManualMonotonicClock::new_shared();
    let timer = clock.new_timer();
    let task = tokio::spawn(async move {
        timer.after(Duration::from_secs(5))?.await?;
        Ok::<_, qubit_clock::TimeError>(())
    });

    let reached = clock.advance_to_next_deadline_async().await;
    assert_eq!(Duration::from_secs(5), reached.elapsed_since_origin());
    task.await??;
    Ok(())
}
```

`advance_to_next_deadline_async()` waits for an active future deadline and
atomically advances to the earliest deadline still registered at that moment.
Cancellation races are retried, and cancelling the driver future does not move
manual time. The [user guide](doc/user_guide.en.md#manual-time-coordination)
documents snapshots, count barriers, multi-stage coordination, runtime
capabilities, wall reanchoring, trait-object injection, and errors.

Synchronous driver threads can use
`advance_to_next_deadline_after_waiters()` to wait for a current waiter-count
condition and advance under the same clock-state lock, avoiding a cancellation
gap between observation and advancement.

`BlockingSleeper` parks its caller while polling the injected timer. Use it
only when that timer can progress independently: the standard timer has a
worker, while manual time must be advanced elsewhere. A Tokio timer must be
driven by another runtime thread; blocking the sole driver of a current-thread
runtime prevents its own deadline from firing.

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-clock](https://github.com/qubit-ltd/rs-clock)
