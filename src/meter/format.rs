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
/// - Less than 1 hour: "Xm Ys" (rounded to the nearest second)
/// - 1 hour or more: "Xh Ym Zs" (rounded to the nearest second)
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
    format_duration_millis_i128(i128::from(millis))
}

/// Formats a value stored in tenths using the specified unit suffix.
fn format_tenths(value_tenths: i128, unit: &str) -> String {
    let whole = value_tenths / 10;
    let tenths = value_tenths % 10;
    if tenths > 0 {
        format!("{}.{}{}", whole, tenths, unit)
    } else {
        format!("{}{}", whole, unit)
    }
}

/// Divides a non-negative value by a positive divisor using half-up rounding.
fn div_round_half_up(value: i128, divisor: i128) -> i128 {
    let quotient = value / divisor;
    let remainder = value % divisor;
    if remainder >= divisor - remainder {
        quotient.saturating_add(1)
    } else {
        quotient
    }
}

/// Formats a non-negative duration expressed as whole seconds.
fn format_duration_seconds(total_seconds: i128) -> String {
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
        format!("{}s", seconds)
    }
}

/// Formats a millisecond duration stored as `i128`.
///
/// This helper keeps [`format_duration_nanos`] from truncating very large
/// nanosecond values before formatting them.
fn format_duration_millis_i128(millis: i128) -> String {
    if millis < 0 {
        return "0 ms".to_string();
    }

    if millis < 1000 {
        return format!("{} ms", millis);
    }

    if millis < 60_000 {
        let tenths = div_round_half_up(millis, 100);
        if tenths >= 600 {
            return format_duration_seconds(tenths / 10);
        }
        return format_tenths(tenths, "s");
    }

    format_duration_seconds(div_round_half_up(millis, 1000))
}

/// Formats a duration in nanoseconds into a human-readable string.
///
/// The format adapts based on the duration:
/// - Less than 1 microsecond: "X ns"
/// - Less than 1 millisecond: "X.Y μs" (rounded to 1 decimal place)
/// - Less than 1 second: "X.Y ms" (rounded to 1 decimal place)
/// - Less than 1 minute: "X.Ys" (rounded to 1 decimal place)
/// - 1 minute or more: "Xm Ys" or "Xh Ym Zs" (rounded to the nearest second)
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
        let tenths = div_round_half_up(nanos, 100);
        if tenths >= 10_000 {
            return format_duration_nanos(tenths * 100);
        }
        format_tenths(tenths, " μs")
    } else if nanos < 1_000_000_000 {
        let tenths = div_round_half_up(nanos, 100_000);
        if tenths >= 10_000 {
            return format_duration_nanos(tenths * 100_000);
        }
        format_tenths(tenths, " ms")
    } else if nanos < 60_000_000_000 {
        let tenths = div_round_half_up(nanos, 100_000_000);
        if tenths >= 600 {
            return format_duration_seconds(tenths / 10);
        }
        format_tenths(tenths, "s")
    } else {
        format_duration_seconds(div_round_half_up(nanos, 1_000_000_000))
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
