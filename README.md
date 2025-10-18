# Prism3 Clock

[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Thread-safe clock abstractions for Rust with monotonic and mock implementations.

## Overview

Prism3 Clock provides a comprehensive set of clock abstractions and implementations for handling various time-related scenarios in Rust applications. It offers robust, thread-safe clock implementations that follow Rust idioms and best practices.

## Features

### 🕐 **Clock Implementations**
- **SystemClock**: Standard clock based on system time
- **MonotonicClock**: Monotonically increasing clock (millisecond precision), unaffected by system time adjustments
- **NanoMonotonicClock**: High-precision monotonic clock (nanosecond precision)
- **MockClock**: Controllable test clock for testing scenarios

### 🔒 **Thread Safety**
- All clock implementations are thread-safe (`Send + Sync`)
- Immutable design for system and monotonic clocks
- Fine-grained locking for mock clock

### ⏱️ **Precision**
- Millisecond precision for general use cases
- Nanosecond precision for high-precision measurements
- Monotonic time sources unaffected by system time adjustments

### 🧪 **Testability**
- Mock clock for controlled testing
- Set time to specific points
- Advance time programmatically
- Auto-increment support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
prism3-clock = "0.1.0"
```

## Quick Start

### Basic Usage

```rust
use prism3_clock::{Clock, MonotonicClock};

// Create a monotonic clock
let clock = MonotonicClock::new();

// Get current time
let now = clock.now();
println!("Current time: {}", now);

// Get millisecond timestamp
let millis = clock.millis();
println!("Milliseconds since epoch: {}", millis);
```

### System Clock

```rust
use prism3_clock::{Clock, SystemClock};

let clock = SystemClock::new();
let now = clock.now();
println!("System time: {}", now);
```

### Monotonic Clock for Time Measurement

```rust
use prism3_clock::{Clock, MonotonicClock};
use std::thread;
use std::time::Duration;

let clock = MonotonicClock::new();
let start = clock.millis();

// Perform some operations
thread::sleep(Duration::from_millis(100));

let elapsed = clock.millis() - start;
println!("Elapsed: {} ms", elapsed);
```

### High-Precision Timing

```rust
use prism3_clock::{NanoClock, NanoMonotonicClock};

let clock = NanoMonotonicClock::new();
let start = clock.nanos();

// Perform some operations

let elapsed = clock.nanos() - start;
println!("Elapsed: {} ns", elapsed);
```

### Mock Clock for Testing

```rust
use prism3_clock::{Clock, ControllableClock, MockClock};
use chrono::Utc;
use std::time::Duration;

let clock = MockClock::new();

// Set to a specific time
let target_time = Utc::now();
clock.set_time(target_time);

// Advance time by 1 second
clock.add_duration(Duration::from_secs(1));

// Add milliseconds
clock.add_millis(500, false);

// Auto-increment on each call
clock.add_millis(10, true);

// Reset to initial state
clock.reset();
```

### Using with Timezones

```rust
use prism3_clock::{Clock, MonotonicClock};
use chrono_tz::Asia::Shanghai;

let clock = MonotonicClock::with_timezone(Shanghai);
let now_shanghai = clock.now_in_timezone(Shanghai);
println!("Shanghai time: {}", now_shanghai);
```

## API Reference

### Clock Trait

The core `Clock` trait provides:

- `millis()` - Returns current time in milliseconds since Unix epoch
- `now()` - Returns current time as `DateTime<Utc>`
- `timezone()` - Returns the clock's timezone
- `now_in_timezone(tz)` - Returns current time in specified timezone
- `with_timezone(tz)` - Creates a new clock with specified timezone

### NanoClock Trait

Extension trait for high-precision clocks:

- `nanos()` - Returns current time in nanoseconds since Unix epoch
- `now_precise()` - Returns high-precision `DateTime<Utc>`

### ControllableClock Trait

Extension trait for controllable clocks (testing):

- `set_time(instant)` - Sets the clock to a specific time
- `add_duration(duration)` - Advances the clock by a duration
- `reset()` - Resets the clock to initial state

## Clock Implementations

### SystemClock

- Based on system time (`std::time::SystemTime`)
- Affected by system time adjustments
- Simple and efficient
- Use for: logging, timestamps, system time synchronization

### MonotonicClock

- Based on `Instant` (monotonically increasing)
- Unaffected by system time adjustments
- Millisecond precision
- Use for: performance monitoring, timeout control, time interval calculation

### NanoMonotonicClock

- Based on `Instant` with nanosecond precision
- Unaffected by system time adjustments
- Higher computational overhead than `MonotonicClock`
- Use for: high-precision measurements, microbenchmarking

### MockClock

- Programmable clock for testing
- Thread-safe with fine-grained locking
- Supports time setting, advancement, and auto-increment
- Use for: unit tests, integration tests, time-dependent logic testing

## Design Philosophy

### Monotonicity

Traditional system clocks can be affected by:
- NTP synchronization
- Manual time adjustments
- Leap seconds

This can cause:
- Negative time durations
- Abnormally large time intervals
- Time going backward

Monotonic clocks solve these problems by:
1. Using `Instant` as the time source (monotonically increasing)
2. Recording initial baseline on creation
3. Calculating relative time based on elapsed duration
4. Ensuring time only moves forward

### Thread Safety

- **Immutable clocks** (`SystemClock`, `MonotonicClock`, `NanoMonotonicClock`):
  - All fields are immutable
  - Naturally thread-safe (`Send + Sync`)
  - No locking overhead

- **Mutable clocks** (`MockClock`):
  - Uses `Arc<Mutex<T>>` for shared state
  - Fine-grained locking strategy
  - Prevents time going backward in concurrent scenarios

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
```

## Dependencies

- **chrono**: Date and time handling
- **chrono-tz**: Timezone support
- **parking_lot**: Efficient mutex implementation

## License

Copyright (c) 2025 3-Prism Co. Ltd. All rights reserved.

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

**Haixing Hu** - *3-Prism Co. Ltd.*

---

For more information about the Prism3 ecosystem, visit our [GitHub homepage](https://github.com/3-prism).

