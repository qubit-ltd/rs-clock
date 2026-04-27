# Qubit Clock

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-clock.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-clock)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-clock/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-clock?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-clock.svg?color=blue)](https://crates.io/crates/qubit-clock)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Thread-safe clock abstractions for Rust with monotonic and mock implementations.

## Overview

Qubit Clock provides a flexible and type-safe clock abstraction system for Rust applications. It offers robust, thread-safe clock implementations with support for basic time access, high-precision measurements, timezone handling, monotonic time, and testing support.

## Features

### 🕐 **Clock Abstractions**
- **Trait-based Design**: Flexible clock abstraction through orthogonal traits
- **Interface Segregation**: Don't force implementations to provide features they don't need
- **Composition over Inheritance**: Extend functionality through wrappers
- **Zero-Cost Abstractions**: Pay only for what you use

### ⏰ **Clock Implementations**
- **SystemClock**: Uses system wall clock time
- **MonotonicClock**: Monotonic time (unaffected by system time changes)
- **NanoMonotonicClock**: Monotonic time with nanosecond precision
- **MockClock**: Controllable clock for testing
- **Zoned\<C\>**: Wrapper that adds timezone support to any clock

### ⏱️ **Time Meters**
- **TimeMeter**: Millisecond-precision time measurement for general use
- **NanoTimeMeter**: Nanosecond-precision time measurement for high-precision needs
- **Human-Readable Output**: Format elapsed time in readable strings
- **Speed Calculation**: Calculate processing speed (items per second/minute)
- **Test-Friendly**: Support injecting mock clocks for deterministic testing

### 🔒 **Thread Safety**
- All clock implementations are `Send + Sync`
- Immutable design for system and monotonic clocks
- Fine-grained locking for mock clock
- Safe to share across threads

### 🌍 **Timezone Support**
- Convert UTC time to any timezone
- Wrap any clock with timezone support
- Based on `chrono-tz` for comprehensive timezone database

### 🧪 **Testing Support**
- Mock clock with controllable time
- Set time to specific points
- Advance time programmatically
- Auto-increment support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-clock = "0.2.0"
```

## Quick Start

### Basic Usage

```rust
use qubit_clock::{Clock, SystemClock};

let clock = SystemClock::new();
let timestamp = clock.millis();
let time = clock.time();
println!("Current time: {}", time);
```

### With Timezone

```rust
use qubit_clock::{Clock, ZonedClock, SystemClock, Zoned};
use chrono_tz::Asia::Shanghai;

let clock = Zoned::new(SystemClock::new(), Shanghai);
let local = clock.local_time();
println!("Local time in Shanghai: {}", local);
```

### Monotonic Time for Performance Measurement

```rust
use qubit_clock::{Clock, MonotonicClock};
use std::thread;
use std::time::Duration;

let clock = MonotonicClock::new();
let start = clock.millis();

thread::sleep(Duration::from_millis(100));

let elapsed = clock.millis() - start;
println!("Elapsed: {} ms", elapsed);
```

### Testing with MockClock

```rust
use qubit_clock::{Clock, ControllableClock, MockClock};
use chrono::{DateTime, Duration, Utc};

let clock = MockClock::new();

// Set to a specific time
let fixed_time = DateTime::parse_from_rfc3339(
    "2024-01-01T00:00:00Z"
).unwrap().with_timezone(&Utc);
clock.set_time(fixed_time);

assert_eq!(clock.time(), fixed_time);

// Advance time
clock.add_duration(Duration::hours(1));
assert_eq!(clock.time(), fixed_time + Duration::hours(1));
```

### High-Precision Measurements

```rust
use qubit_clock::{NanoClock, NanoMonotonicClock};

let clock = NanoMonotonicClock::new();
let start = clock.nanos();

// Perform some operation
for _ in 0..1000 {
    // Some work
}

let elapsed = clock.nanos() - start;
println!("Elapsed: {} ns", elapsed);
```

### Time Meters for Elapsed Time Measurement

```rust
use qubit_clock::meter::TimeMeter;
use std::thread;
use std::time::Duration;

let mut meter = TimeMeter::new();
meter.start();
thread::sleep(Duration::from_millis(100));
meter.stop();
println!("Elapsed: {}", meter.readable_duration());
```

### High-Precision Time Meter

```rust
use qubit_clock::meter::NanoTimeMeter;

let mut meter = NanoTimeMeter::new();
meter.start();

// Perform some operation
for _ in 0..1000 {
    // Some work
}

meter.stop();
println!("Elapsed: {} ns", meter.nanos());
println!("Readable: {}", meter.readable_duration());
```

### Speed Calculation with Time Meter

```rust
use qubit_clock::meter::TimeMeter;
use std::thread;
use std::time::Duration;

let mut meter = TimeMeter::new();
meter.start();

// Process 1000 items
for _ in 0..1000 {
    thread::sleep(Duration::from_micros(100));
}

meter.stop();
println!("Processed 1000 items in {}", meter.readable_duration());
println!("Speed: {}", meter.formatted_speed_per_second(1000));
```

## Architecture

The crate is built around several orthogonal traits:

- **Clock**: Base trait providing UTC time
- **NanoClock**: Extension for nanosecond precision
- **ZonedClock**: Extension for timezone support
- **ControllableClock**: Extension for time control (testing)

This design follows the **Interface Segregation Principle**, ensuring that implementations only need to provide the features they actually support.

## Clock Implementations

### SystemClock

- Based on system wall clock time
- Subject to system time adjustments (NTP, manual changes)
- Zero-sized type (ZST) with no runtime overhead
- Use for: logging, timestamps, general time queries

### MonotonicClock

- Based on `std::time::Instant` (monotonically increasing)
- Unaffected by system time adjustments
- Millisecond precision
- Records base point on creation
- Use for: performance monitoring, timeout control, time interval measurements

### NanoMonotonicClock

- Based on `std::time::Instant` with nanosecond precision
- Unaffected by system time adjustments
- Higher precision than `MonotonicClock`
- Use for: high-precision measurements, microbenchmarking

### MockClock

- Controllable clock for testing
- Thread-safe with `Arc<Mutex<>>`
- Supports time setting, advancement, and auto-increment
- Based on `MonotonicClock` for stability
- Use for: unit tests, integration tests, time-dependent logic testing

### Zoned\<C\>

- Wrapper that adds timezone support to any clock
- Generic over any `Clock` implementation
- Converts UTC time to local time in specified timezone
- Use for: displaying local time, timezone conversions

## Time Meters

### TimeMeter

A millisecond-precision time meter for measuring elapsed time with the following features:

- **Flexible Clock Source**: Supports any clock implementing `Clock` trait
- **Default to MonotonicClock**: Uses monotonic time by default for stable measurements
- **Multiple Output Formats**: Milliseconds, seconds, minutes, and human-readable format
- **Speed Calculation**: Calculate processing speed (items per second/minute)
- **Real-Time Monitoring**: Get elapsed time without stopping the meter
- **Test-Friendly**: Inject `MockClock` for deterministic testing

Example output formats:
- `123 ms` - Less than 1 second
- `1.5s` - 1-60 seconds
- `1m 23s` - More than 1 minute
- `1h 1m 5s` - More than 1 hour

### NanoTimeMeter

A nanosecond-precision time meter with features similar to `TimeMeter`:

- **Nanosecond Precision**: Based on `NanoClock` trait
- **Default to NanoMonotonicClock**: Uses high-precision monotonic time
- **Human-Readable Output**: Automatically chooses appropriate unit (ns, μs, ms, s, m, h)
- **Speed Calculation**: High-precision speed calculation
- **Test-Friendly**: Supports mock clock injection

Example output formats:
- `123 ns` - Less than 1 microsecond
- `123.4 μs` - 1-1000 microseconds
- `123.4 ms` - 1-1000 milliseconds
- `1.5s` - 1-60 seconds
- `1m 23s` - More than 1 minute
- `1h 1m 5s` - More than 1 hour

## Why Not Just Use `std::time::Instant`?

`std::time::Instant` is the right primitive for measuring real elapsed time in
production code. It is monotonic, fast, and simple:

```rust
let start = std::time::Instant::now();
// do work
let elapsed = start.elapsed();
```

This crate exists for the cases where elapsed-time measurement is only part of
the problem:

- Use `MockClock` when tests need deterministic control over time. Instead of
  sleeping for real seconds or minutes, tests can set the current time, advance
  it instantly, reset it, or enable auto-advance on each read.
- Use `TimeMeter` or `NanoTimeMeter` when application code needs a reusable
  start/stop meter with formatted durations, speed calculations, and an
  injectable clock source. They use monotonic clocks by default, but can accept
  `MockClock` in tests.
- Use the `Clock` traits when business logic depends on "current time" and must
  be testable without coupling directly to the system clock or `Instant::now()`.

In short, `Instant` measures real elapsed time; `MockClock` makes time
controllable for tests; `TimeMeter` turns elapsed-time measurement into a
reusable, formatted, testable abstraction.

## API Reference

### Clock Trait

The core `Clock` trait provides:

- `millis()` - Returns current time in milliseconds since Unix epoch
- `time()` - Returns current time as `DateTime<Utc>`

### NanoClock Trait

Extension trait for high-precision clocks:

- `nanos()` - Returns current time in nanoseconds since Unix epoch
- `time_precise()` - Returns high-precision `DateTime<Utc>`

### ZonedClock Trait

Extension trait for timezone support:

- `timezone()` - Returns the clock's timezone
- `local_time()` - Returns current time in the clock's timezone

Use `Zoned::new(clock, tz)` to select the timezone for a clock.

### ControllableClock Trait

Extension trait for controllable clocks (testing):

- `set_time(instant)` - Sets the clock to a specific time
- `add_duration(duration)` - Advances the clock by a duration
- `reset()` - Resets the clock to initial state

## Design Principles

### Interface Segregation

The crate follows the Interface Segregation Principle by providing separate traits for different capabilities:

- Not all clocks need nanosecond precision → `NanoClock` is separate
- Not all clocks need timezone support → `ZonedClock` is separate
- Only test clocks need controllability → `ControllableClock` is separate

This allows implementations to provide only the features they need, keeping the API clean and focused.

### Single Responsibility

Each trait and type has one clear purpose:

- `Clock` - Provide UTC time
- `NanoClock` - Provide high-precision time
- `ZonedClock` - Provide timezone conversion
- `ControllableClock` - Provide time control for testing

### Composition over Inheritance

Functionality is extended through wrappers rather than inheritance:

- `Zoned<C>` wraps any `Clock` to add timezone support
- Time meters accept any `Clock` implementation via generics

### Zero-Cost Abstractions

The design ensures you only pay for what you use:

- `SystemClock` and `MonotonicClock` are zero-sized or minimal overhead
- Trait methods are often inlined
- Generic code is monomorphized at compile time

## Testing & Code Coverage

This project maintains comprehensive test coverage with detailed validation of all functionality.

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
./coverage.sh

# Generate text format report
./coverage.sh text

# Run CI checks (tests, lints, formatting)
./ci-check.sh
```

## Dependencies

- **chrono**: Date and time handling with serialization support
- **chrono-tz**: Comprehensive timezone database
- **parking_lot**: Efficient mutex implementation for mock clock

## Use Cases

### Performance Monitoring

```rust
use qubit_clock::meter::TimeMeter;

let mut meter = TimeMeter::new();
meter.start();

// Perform operation
process_data();

meter.stop();
log::info!("Processing took: {}", meter.readable_duration());
```

### Timeout Control

```rust
use qubit_clock::{Clock, MonotonicClock};
use std::time::Duration;

let clock = MonotonicClock::new();
let deadline = clock.millis() + 5000; // 5 seconds from now

while clock.millis() < deadline {
    if try_operation() {
        break;
    }
}
```

### Testing Time-Dependent Logic

```rust
use qubit_clock::{Clock, ControllableClock, MockClock};
use chrono::Duration;

#[test]
fn test_expiration() {
    let clock = MockClock::new();
    let item = Item::new(clock.clone());

    // Fast-forward 1 hour
    clock.add_duration(Duration::hours(1));

    assert!(item.is_expired());
}
```

### Benchmarking

```rust
use qubit_clock::meter::NanoTimeMeter;

let mut meter = NanoTimeMeter::new();
meter.start();

for _ in 0..10000 {
    expensive_operation();
}

meter.stop();
println!("Average time per operation: {} ns", meter.nanos() / 10000);
```

## License

Copyright (c) 2025 - 2026. Haixing Hu, Qubit Co. Ltd. All rights reserved.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

See [LICENSE](LICENSE) for the full license text.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

---

For more information about the Qubit open source projects, visit our [GitHub homepage](https://github.com/qubit-ltd).
