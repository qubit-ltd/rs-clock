/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Regression tests for public API names documented in README files.

fn readmes() -> [&'static str; 2] {
    [
        include_str!("../README.md"),
        include_str!("../README.zh_CN.md"),
    ]
}

#[test]
fn test_readmes_use_existing_meter_speed_api() {
    for readme in readmes() {
        assert!(!readme.contains("readable_speed("));
        assert!(readme.contains("formatted_speed_per_second("));
    }
}

#[test]
fn test_readmes_use_existing_time_api_names() {
    for readme in readmes() {
        assert!(!readme.contains("nano_time()"));
        assert!(!readme.contains("local_time_in"));
        assert!(readme.contains("time_precise()"));
        assert!(readme.contains("local_time()"));
    }
}

#[test]
fn test_readmes_do_not_reference_removed_mock_time_api() {
    for readme in readmes() {
        assert!(!readme.contains("MockNanoClock"));
        assert!(!readme.contains("MockClockProgression"));
        assert!(!readme.contains("MockSleeper::advance"));
        assert!(!readme.contains("sleeper.advance("));
        assert!(readme.contains("MockTimeline"));
        assert!(readme.contains("MockTime"));
    }
}
