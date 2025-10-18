/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Demonstration of TimeMeter usage.
//!
//! This example shows how to use TimeMeter for measuring elapsed time
//! with millisecond precision.

use prism3_clock::meter::TimeMeter;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== TimeMeter Demo ===\n");

    // Example 1: Basic usage
    println!("1. Basic usage:");
    let mut meter = TimeMeter::new();
    meter.start();
    simulate_work(100);
    meter.stop();
    println!("   Elapsed: {}", meter.readable_duration());
    println!("   Milliseconds: {}", meter.millis());
    println!();

    // Example 2: Auto-start
    println!("2. Auto-start:");
    let mut meter = TimeMeter::start_now();
    simulate_work(200);
    meter.stop();
    println!("   Elapsed: {}", meter.readable_duration());
    println!();

    // Example 3: Real-time monitoring
    println!("3. Real-time monitoring (without stop):");
    let meter = TimeMeter::start_now();
    for i in 1..=5 {
        simulate_work(50);
        println!("   Iteration {}: {}", i, meter.readable_duration());
    }
    println!();

    // Example 4: Speed calculation
    println!("4. Speed calculation:");
    let mut meter = TimeMeter::start_now();
    let items_processed = 1000;
    simulate_work(2000); // 2 seconds
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
    println!(
        "   Speed: {}",
        meter.formatted_speed_per_minute(items_processed)
    );
    println!();

    // Example 5: Restart
    println!("5. Restart:");
    let mut meter = TimeMeter::start_now();
    simulate_work(100);
    meter.stop();
    println!("   First run: {}", meter.readable_duration());

    meter.restart();
    simulate_work(150);
    meter.stop();
    println!("   Second run: {}", meter.readable_duration());
    println!();

    // Example 6: Multiple measurements
    println!("6. Multiple measurements:");
    let mut meter = TimeMeter::new();

    for i in 1..=3 {
        meter.start();
        simulate_work(100);
        meter.stop();
        println!(
            "   Measurement {}: {} ({} ms)",
            i,
            meter.readable_duration(),
            meter.millis()
        );
        meter.reset();
    }
}

fn simulate_work(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}
