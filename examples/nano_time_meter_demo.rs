/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Demonstration of NanoTimeMeter usage.
//!
//! This example shows how to use NanoTimeMeter for high-precision
//! time measurement with nanosecond precision.

use prism3_clock::meter::NanoTimeMeter;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== NanoTimeMeter Demo ===\n");

    // Example 1: Basic usage
    println!("1. Basic usage:");
    let mut meter = NanoTimeMeter::new();
    meter.start();
    simulate_work(100);
    meter.stop();
    println!("   Elapsed: {}", meter.readable_duration());
    println!("   Nanoseconds: {}", meter.nanos());
    println!("   Microseconds: {}", meter.micros());
    println!("   Milliseconds: {}", meter.millis());
    println!();

    // Example 2: High-precision measurement
    println!("2. High-precision measurement:");
    let mut meter = NanoTimeMeter::start_now();
    // Very short operation
    for _ in 0..1000 {
        let _ = 1 + 1;
    }
    meter.stop();
    println!("   Elapsed: {}", meter.readable_duration());
    println!("   Nanoseconds: {}", meter.nanos());
    println!();

    // Example 3: Comparing precision
    println!("3. Comparing precision:");
    let mut meter = NanoTimeMeter::start_now();
    simulate_work(150);
    meter.stop();
    println!("   Readable: {}", meter.readable_duration());
    println!("   Nanos: {} ns", meter.nanos());
    println!("   Micros: {} μs", meter.micros());
    println!("   Millis: {} ms", meter.millis());
    println!();

    // Example 4: Speed calculation
    println!("4. Speed calculation:");
    let mut meter = NanoTimeMeter::start_now();
    let items_processed = 10000;
    simulate_work(1000); // 1 second
    meter.stop();
    println!(
        "   Processed {} items in {}",
        items_processed,
        meter.readable_duration()
    );
    println!(
        "   Speed: {}",
        meter.formatted_speed_per_second(items_processed)
    );
    println!();

    // Example 5: Microbenchmarking
    println!("5. Microbenchmarking:");
    let iterations = 10;
    let mut total_nanos = 0i128;

    for i in 1..=iterations {
        let mut meter = NanoTimeMeter::start_now();
        // Simulate a very fast operation
        for _ in 0..100 {
            let _ = 1 + 1;
        }
        meter.stop();
        let nanos = meter.nanos();
        total_nanos += nanos;
        println!("   Iteration {}: {} ns", i, nanos);
    }

    let avg_nanos = total_nanos / iterations;
    println!("   Average: {} ns", avg_nanos);
    println!();

    // Example 6: Real-time monitoring
    println!("6. Real-time monitoring:");
    let meter = NanoTimeMeter::start_now();
    for i in 1..=5 {
        simulate_work(50);
        println!(
            "   Iteration {}: {} ({} ns)",
            i,
            meter.readable_duration(),
            meter.nanos()
        );
    }
}

fn simulate_work(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}
