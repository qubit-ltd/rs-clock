// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use crate::support::manual_waiter_registry::ManualWaiterRegistry;
use crate::support::manual_waiter_registry::allocate_identifier;
use std::time::Duration;

#[test]
fn test_manual_registry_latches_reached_observer_after_waiter_unregisters() {
    let mut registry = ManualWaiterRegistry::new();
    let observer_id = registry
        .register_observer(1, registry.count())
        .expect("an unsatisfied observer should be registered");
    let waiter_id = registry.register_async(Duration::from_secs(1));

    assert!(registry.reached_observer_wakers().is_empty());
    assert!(registry.unregister_async(waiter_id));

    assert!(!registry.contains_observer(observer_id));
    assert_eq!(0, registry.count());
}

#[test]
fn test_manual_registry_identifier_allocates_maximum_before_exhaustion() {
    let mut next_identifier = u64::MAX;

    assert_eq!(
        u64::MAX,
        allocate_identifier(&mut next_identifier, "identifiers exhausted"),
    );
    assert_eq!(0, next_identifier);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        allocate_identifier(&mut next_identifier, "identifiers exhausted")
    }));
    assert!(result.is_err());
}
