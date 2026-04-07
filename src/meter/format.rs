/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Duration and speed formatting utilities.
//!
//! This module provides functions to format durations and speeds into
//! human-readable strings.
//!
//! # Author
//!
//! Haixing Hu

/// Formats a duration in milliseconds into a human-readable string.
///
/// The format adapts based on the duration:
/// - Less than 1 second: "X ms"
/// - Less than 1 minute: "X.Ys" (rounded to 1 decimal place)
/// - Less than 1 hour: "Xm Ys"
/// - 1 hour or more: "Xh Ym Zs"
///
/// # Arguments
///
/// * `millis` - The duration in milliseconds
///
/// # Returns
///
/// A human-readable string representation of the duration
///
/// # Examples
///
/// ```
/// use qubit_clock::meter::format_duration_millis;
///
/// assert_eq!(format_duration_millis(500), "500 ms");
/// assert_eq!(format_duration_millis(1500), "1.5s");
/// assert_eq!(format_duration_millis(65000), "1m 5s");
/// assert_eq!(format_duration_millis(3665000), "1h 1m 5s");
/// ```
pub fn format_duration_millis(millis: i64) -> String {
    if millis < 0 {
        return "0 ms".to_string();
    }

    if millis < 1000 {
        return format!("{} ms", millis);
    }

    let total_seconds = millis / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        if minutes > 0 && seconds > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}h {}m", hours, minutes)
        } else if seconds > 0 {
            format!("{}h {}s", hours, seconds)
        } else {
            format!("{}h", hours)
        }
    } else if minutes > 0 {
        if seconds > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}m", minutes)
        }
    } else {
        let millis_part = millis % 1000;
        if millis_part > 0 {
            format!("{}.{}s", seconds, millis_part / 100)
        } else {
            format!("{}s", seconds)
        }
    }
}

/// Formats a duration in nanoseconds into a human-readable string.
///
/// The format adapts based on the duration:
/// - Less than 1 microsecond: "X ns"
/// - Less than 1 millisecond: "X.Y μs" (rounded to 1 decimal place)
/// - Less than 1 second: "X.Y ms" (rounded to 1 decimal place)
/// - 1 second or more: delegates to `format_duration_millis`
///
/// # Arguments
///
/// * `nanos` - The duration in nanoseconds
///
/// # Returns
///
/// A human-readable string representation of the duration
///
/// # Examples
///
/// ```
/// use qubit_clock::meter::format_duration_nanos;
///
/// assert_eq!(format_duration_nanos(500), "500 ns");
/// assert_eq!(format_duration_nanos(1500), "1.5 μs");
/// assert_eq!(format_duration_nanos(1500000), "1.5 ms");
/// assert_eq!(format_duration_nanos(1500000000), "1.5s");
/// ```
pub fn format_duration_nanos(nanos: i128) -> String {
    if nanos < 0 {
        return "0 ns".to_string();
    }

    if nanos < 1000 {
        return format!("{} ns", nanos);
    }

    if nanos < 1_000_000 {
        let micros = nanos / 1000;
        let nanos_part = nanos % 1000;
        if nanos_part > 0 {
            format!("{}.{} μs", micros, nanos_part / 100)
        } else {
            format!("{} μs", micros)
        }
    } else if nanos < 1_000_000_000 {
        let millis = nanos / 1_000_000;
        let micros_part = (nanos % 1_000_000) / 100_000;
        if micros_part > 0 {
            format!("{}.{} ms", millis, micros_part)
        } else {
            format!("{} ms", millis)
        }
    } else {
        let millis = (nanos / 1_000_000) as i64;
        format_duration_millis(millis)
    }
}

/// Formats a speed value with a unit suffix.
///
/// The speed is formatted with 2 decimal places. If the speed is NaN or
/// infinite, returns "N/A".
///
/// # Arguments
///
/// * `speed` - The speed value
/// * `unit` - The unit suffix (e.g., "/s", "/m")
///
/// # Returns
///
/// A formatted string like "123.45/s" or "N/A"
///
/// # Examples
///
/// ```
/// use qubit_clock::meter::format_speed;
///
/// assert_eq!(format_speed(123.456, "/s"), "123.46/s");
/// assert_eq!(format_speed(0.0, "/m"), "0.00/m");
/// assert_eq!(format_speed(f64::NAN, "/s"), "N/A");
/// ```
pub fn format_speed(speed: f64, unit: &str) -> String {
    if speed.is_nan() || speed.is_infinite() {
        "N/A".to_string()
    } else {
        format!("{:.2}{}", speed, unit)
    }
}
