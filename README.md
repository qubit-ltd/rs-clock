# Qubit Clock

[![Rust CI](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-clock/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-clock/coverage-badge.json)](https://qubit-ltd.github.io/rs-clock/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Time is a hidden dependency. Code that calls `SystemTime::now()`,
`Instant::now()`, or a sleep function directly is tied to the machine clock:
tests must really wait, boundary cases are hard to reach, and clock changes can
make results nondeterministic.

`qubit-clock` turns time into an injectable dependency. Application components
depend on small clock and timer traits; the composition root supplies standard
implementations in production and fixed or manually advanced implementations
in tests. The same business code runs in both environments, with no test-only
branch and no real delay.

Detailed documentation:

- [English User Guide](doc/user_guide.en.md)
- [中文用户手册](doc/user_guide.zh_CN.md)
- [API documentation](https://docs.rs/qubit-clock)

## A first example

This session records a monotonic deadline instead of reading the global clock.
Its constructor accepts `Arc<dyn MonotonicClock>`, so the caller chooses how
time progresses:

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
    // Production assembly uses the operating system's monotonic clock.
    let _production = Session::new(
        Arc::new(StdMonotonicClock::new()),
        Duration::from_secs(30),
    )?;

    // A test injects manual time and reaches the boundary immediately.
    let clock = ManualMonotonicClock::new_shared();
    let session = Session::new(clock.clone(), Duration::from_secs(30))?;
    assert!(!session.is_expired());

    clock.advance(Duration::from_secs(30))?;
    assert!(session.is_expired());
    Ok(())
}
```

The test covers an exact 30-second boundary without sleeping for 30 seconds.
Only the assembly changes; `Session` contains no mock flag or test-specific
logic.

## Components at a glance

| Need | Trait | Real time | Deterministic tests |
|---|---|---|---|
| Externally meaningful timestamps | `WallClock` | `StdWallClock` | `FixedWallClock`, `ManualWallClock` |
| Elapsed time and deadlines | `MonotonicClock` | `StdMonotonicClock`, `TokioMonotonicClock` | `ManualMonotonicClock` |
| Async deadlines | `Timer` | `StdTimer`, `TokioTimer` | `ManualTimer` |
| Blocking waits | `BlockingSleeper` adapter | compose a timer with independent progress | compose a manually driven timer |

Wall-clock time may jump and is intended for externally meaningful timestamps.
Monotonic time never moves backward within one clock domain and is intended for
elapsed time, retries, and timeouts. Every clock creates a same-domain timer
directly with `clock.new_timer()`.

## Installation

```toml
[dependencies]
qubit-clock = "0.12"
```

Enable the `tokio` feature when you need Tokio-backed clock and timer types and
their runtime-related errors:

```toml
[dependencies]
qubit-clock = { version = "0.12", features = ["tokio"] }
```

The `tokio` feature exposes `TokioMonotonicClock`, `TokioTimer`, and their
runtime-related errors. Manual time is executor-neutral and does not require
this feature. Tests that need deterministic timer failures can enable the
default-off `test-util` feature in a development dependency.

## Timers and waits

Inject `Arc<dyn Timer>` when a component must await a deadline instead of only
checking the current time. `Timer::after` creates a relative deadline and
`Timer::at` accepts an absolute `MonotonicInstant`. A
`ManualMonotonicClock` creates a same-domain manual timer, so tests can advance
logical time instead of waiting for a scheduler or the operating system.

`BlockingSleeper` adapts a timer for synchronous code. Its timer backend must
be able to progress independently while the calling thread is parked. The
[user guide](doc/user_guide.en.md) covers manual-time coordination, Tokio
runtime ownership, wall-clock projection, cancellation, and error handling.

## Use in related libraries

The same injection model is used by `rs-lock` to test timeout-aware waits and
by `rs-retry` to test retry delays, attempt timeouts, and elapsed-time budgets.
Those libraries inject a `Timer` or `MonotonicClock`; their production code
does not contain a separate mock waiting algorithm.

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
