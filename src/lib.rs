/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
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
//! - **Testing support**: Shared mock timelines for clocks and sleepers
//! - **Mockable sleeps**: Relative sleep abstractions with real and mock implementations
//!
//! # Architecture
//!
//! The crate is built around several orthogonal traits:
//!
//! - [`Clock`]: Base trait providing UTC time
//! - [`NanoClock`]: Extension for nanosecond precision
//! - [`ZonedClock`]: Extension for timezone support
//! - [`ControllableClock`]: Extension for time control (testing)
//! - [`sleep::Sleeper`]: Relative blocking sleep abstraction
//! - `sleep::AsyncSleeper`: Tokio async relative sleep abstraction enabled by
//!   the `tokio` feature
//! - [`MockTimeline`]: Shared monotonic mock time source for deterministic tests
//!
//! # Implementations
//!
//! Several clock implementations are provided:
//!
//! - [`SystemClock`]: Uses system wall clock time
//! - [`MonotonicClock`]: Monotonic time (unaffected by system time changes)
//! - [`NanoMonotonicClock`]: Monotonic time with nanosecond precision
//! - [`MockClock`]: UTC and nanosecond clock backed by a mock timeline
//! - [`MockTime`]: Convenience facade bundling one timeline, clock, and sleeper
//! - [`Zoned<C>`](Zoned): Wrapper that adds timezone support to any clock
//! - [`sleep::SystemSleeper`]: Real relative sleeper, with async support when
//!   the `tokio` feature is enabled
//! - [`sleep::MockSleeper`]: Timeline-backed relative sleeper for tests,
//!   with async support when the `tokio` feature is enabled
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

// Re-export chrono types for convenience
pub use chrono::{
    DateTime,
    Duration,
    Utc,
};
pub use chrono_tz::Tz;

// Clock traits and implementations
pub mod clock;

pub use clock::{
    Clock,
    ControllableClock,
    MonotonicClock,
    NanoClock,
    NanoMonotonicClock,
    SystemClock,
    Zoned,
    ZonedClock,
};

// Unified mock time runtime
pub mod mock;

pub use mock::{
    MockClock,
    MockInstant,
    MockTime,
    MockTimeError,
    MockTimeline,
    MockWaiterKind,
};

// Time meters
pub mod meter;

// Relative sleep abstractions
pub mod sleep;
