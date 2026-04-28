/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Thread-safe clock abstractions for Rust.
//!
//! This crate provides a flexible and type-safe clock abstraction system with
//! support for:
//!
//! - **Basic time access**: Get current UTC time
//! - **High precision**: Nanosecond-level time measurements
//! - **Timezone support**: Convert to local time in any timezone
//! - **Monotonic time**: Time that never goes backwards
//! - **Testing support**: Controllable mock clocks for tests
//!
//! # Architecture
//!
//! The crate is built around several orthogonal traits:
//!
//! - [`Clock`]: Base trait providing UTC time
//! - [`NanoClock`]: Extension for nanosecond precision
//! - [`ZonedClock`]: Extension for timezone support
//! - [`ControllableClock`]: Extension for time control (testing)
//!
//! # Implementations
//!
//! Several clock implementations are provided:
//!
//! - [`SystemClock`]: Uses system wall clock time
//! - [`MonotonicClock`]: Monotonic time (unaffected by system time changes)
//! - [`NanoMonotonicClock`]: Monotonic time with nanosecond precision
//! - [`MockClock`]: Controllable clock for testing
//! - [`Zoned<C>`](Zoned): Wrapper that adds timezone support to any clock
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```
//! use qubit_clock::{Clock, SystemClock};
//!
//! let clock = SystemClock::new();
//! let timestamp = clock.millis();
//! let time = clock.time();
//! println!("Current time: {}", time);
//! ```
//!
//! ## With Timezone
//!
//! ```
//! use qubit_clock::{Clock, ZonedClock, SystemClock, Zoned};
//! use chrono_tz::Asia::Shanghai;
//!
//! let clock = Zoned::new(SystemClock::new(), Shanghai);
//! let local = clock.local_time();
//! println!("Local time in Shanghai: {}", local);
//! ```
//!
//! ## Monotonic Time for Performance Measurement
//!
//! ```
//! use qubit_clock::{Clock, MonotonicClock};
//! use std::thread;
//! use std::time::Duration;
//!
//! let clock = MonotonicClock::new();
//! let start = clock.millis();
//!
//! thread::sleep(Duration::from_millis(100));
//!
//! let elapsed = clock.millis() - start;
//! println!("Elapsed: {} ms", elapsed);
//! ```
//!
//! ## Testing with MockClock
//!
//! ```
//! use qubit_clock::{Clock, ControllableClock, MockClock};
//! use chrono::{DateTime, Duration, Utc};
//!
//! let clock = MockClock::new();
//!
//! // Set to a specific time
//! let fixed_time = DateTime::parse_from_rfc3339(
//!     "2024-01-01T00:00:00Z"
//! ).unwrap().with_timezone(&Utc);
//! clock.set_time(fixed_time);
//!
//! assert_eq!(clock.time(), fixed_time);
//!
//! // Advance time
//! clock.add_duration(Duration::hours(1));
//! assert_eq!(clock.time(), fixed_time + Duration::hours(1));
//! ```
//!
//! ## High-Precision Measurements
//!
//! ```
//! use qubit_clock::{NanoClock, NanoMonotonicClock};
//!
//! let clock = NanoMonotonicClock::new();
//! let start = clock.nanos();
//!
//! // Perform some operation
//! for _ in 0..1000 {
//!     // Some work
//! }
//!
//! let elapsed = clock.nanos() - start;
//! println!("Elapsed: {} ns", elapsed);
//! ```
//!
//! ## Time Meters for Elapsed Time Measurement
//!
//! ```
//! use qubit_clock::meter::TimeMeter;
//! use std::thread;
//! use std::time::Duration;
//!
//! let mut meter = TimeMeter::new();
//! meter.start();
//! thread::sleep(Duration::from_millis(100));
//! meter.stop();
//! println!("Elapsed: {}", meter.readable_duration());
//! ```
//!
//! # Design Principles
//!
//! - **Interface Segregation**: Don't force implementations to provide
//!   features they don't need
//! - **Single Responsibility**: Each trait and type has one clear purpose
//! - **Composition over Inheritance**: Extend functionality through wrappers
//! - **Zero-Cost Abstractions**: Pay only for what you use
//!
//! # Author
//!
//! Haixing Hu

// Re-export chrono types for convenience
pub use chrono::{DateTime, Duration, Utc};
pub use chrono_tz::Tz;

// Traits
mod clock;
mod controllable_clock;
mod nano_clock;
mod zoned_clock;

pub use clock::Clock;
pub use controllable_clock::ControllableClock;
pub use nano_clock::NanoClock;
pub use zoned_clock::ZonedClock;

// Implementations
mod mock_clock;
mod monotonic_clock;
mod nano_monotonic_clock;
mod system_clock;
mod zoned;

pub use mock_clock::MockClock;
pub use monotonic_clock::MonotonicClock;
pub use nano_monotonic_clock::NanoMonotonicClock;
pub use system_clock::SystemClock;
pub use zoned::Zoned;

// Time meters
pub mod meter;
