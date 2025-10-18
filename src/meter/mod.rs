/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Time measurement utilities.
//!
//! This module provides time meters for measuring elapsed time with
//! different precision levels:
//!
//! - [`TimeMeter`]: Millisecond precision time meter for general use cases
//! - [`NanoTimeMeter`]: Nanosecond precision time meter for
//!   high-precision measurements
//!
//! # Examples
//!
//! ## Basic Usage with TimeMeter
//!
//! ```
//! use prism3_clock::meter::TimeMeter;
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
//! ## High-Precision Measurement with NanoTimeMeter
//!
//! ```
//! use prism3_clock::meter::NanoTimeMeter;
//!
//! let mut meter = NanoTimeMeter::new();
//! meter.start();
//! // Perform some operation
//! meter.stop();
//! println!("Elapsed: {} ns", meter.nanos());
//! ```
//!
//! # Author
//!
//! Haixing Hu

mod format;
mod nano_time_meter;
mod time_meter;

pub use format::{format_duration_millis, format_duration_nanos, format_speed};
pub use nano_time_meter::NanoTimeMeter;
pub use time_meter::TimeMeter;
