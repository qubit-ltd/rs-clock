// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Records the external coverage boundary for the private standard Timer
//! worker guard.
//!
//! Every future-deadline registration exercises worker-guard construction and
//! handoff. The process-worker identity and retention contract is verified by
//! the standalone `std_timer_scheduler_tests` target. Forcing the guard's
//! steady-state drop would require inducing a private scheduler invariant
//! panic, which has no deterministic public trigger. This mirrored test module
//! therefore intentionally avoids duplicating the public worker-retention
//! facade or widening production visibility solely for direct access.
