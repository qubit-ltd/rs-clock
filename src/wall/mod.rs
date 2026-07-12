// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Wall-clock abstractions and implementations.

mod fixed_wall_clock;
mod manual_wall_clock;
mod std_wall_clock;
mod wall_clock;

pub use fixed_wall_clock::FixedWallClock;
pub use manual_wall_clock::ManualWallClock;
pub use std_wall_clock::StdWallClock;
pub use wall_clock::WallClock;
