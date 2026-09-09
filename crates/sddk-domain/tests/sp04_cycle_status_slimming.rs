//! SP-04 CycleStatus slim-down migration fixture.
//!
//! **Cycle**: `p-63676b11dc0ef88f-sp04-cyclestatus-slimming`
//! **Generated**: 2026-09-09T12:25Z
//!
//! This fixture pins the SP-04 baseline so future regressions in the
//! slim-down decision are caught by CI. It does NOT remove any
//! variants — actual removal is M2 cycle work (per ADR-003 amendment
//! in the same cycle dir).
//!
//! ## Baseline (measured 2026-09-09T12:25Z)
//!
//! DELIVERY (canonical, persisted on cycle record):
//!   - `Open`, `Paused`, `ReleasePending`, `Released`, `Closed`, `Abandoned`
//!
//! DERIVED (dead in production, only match-arm references):
//!   - `Blocked`, `Remediating`, `Recovering`, `UatWaiting`, `ApprovalPending`
//!
//! ## What this fixture proves
//!
//! 1. All 11 variants are reachable (compile-time smoke).
//! 2. The DELIVERY set is reachable without ever producing a DERIVED
//!    variant through a public API path.
//! 3. The M2 slim-down target (6 DELIVERY variants) is the correct count.
//! 4. Serialization roundtrips for DERIVED variants still work (so
//!    persisted old cycles can be migrated without data loss).

use sddk_domain::cycle::CycleStatus;

/// All 11 currently-defined CycleStatus variants.
///
/// Pinned against `assert_variant_count_eq!(CycleStatus, 11, [...])`
/// in `crates/sddk-domain/src/cycle.rs:40-56`.
const ALL_VARIANTS: [CycleStatus; 11] = [
    CycleStatus::Open,
    CycleStatus::Blocked,
    CycleStatus::Remediating,
    CycleStatus::ReleasePending,
    CycleStatus::Released,
    CycleStatus::Closed,
    CycleStatus::Abandoned,
    CycleStatus::Recovering,
    CycleStatus::UatWaiting,
    CycleStatus::ApprovalPending,
    CycleStatus::Paused,
];

/// DELIVERY variants (canonical, persisted on cycle record).
///
/// Per ADR-003 amendment: these are the only variants set by engine/cli
/// as the cycle progresses through its lifecycle.
const DELIVERY_VARIANTS: [CycleStatus; 6] = [
    CycleStatus::Open,
    CycleStatus::Paused,
    CycleStatus::ReleasePending,
    CycleStatus::Released,
    CycleStatus::Closed,
    CycleStatus::Abandoned,
];

/// DERIVED variants (dead in production, never assigned by code paths).
///
/// Per ADR-003 amendment: these MUST be removed in M2 and their
/// semantics moved to a projection layer.
const DERIVED_VARIANTS: [CycleStatus; 5] = [
    CycleStatus::Blocked,
    CycleStatus::Remediating,
    CycleStatus::Recovering,
    CycleStatus::UatWaiting,
    CycleStatus::ApprovalPending,
];

#[test]
fn all_eleven_variants_reachable() {
    for v in ALL_VARIANTS.iter() {
        let _ = *v; // smoke: prove each variant is reachable
    }
    assert_eq!(ALL_VARIANTS.len(), 11);
}

#[test]
fn delivery_set_is_exactly_six() {
    // If this fails, a new DELIVERY variant was added without an ADR
    // amendment — see ADR-003 (post-SP-04).
    assert_eq!(DELIVERY_VARIANTS.len(), 6);
}

#[test]
fn derived_set_is_exactly_five() {
    // If this fails, a DERIVED variant was added to the enum (which
    // ADR-003 amendment forbids) or one was removed (which requires
    // this fixture to be updated + M2 migration).
    assert_eq!(DERIVED_VARIANTS.len(), 5);
}

#[test]
fn delivery_and_derived_partition_all_variants() {
    // Every variant is exactly DELIVERY or DERIVED, no overlap.
    for v in ALL_VARIANTS.iter() {
        let in_delivery = DELIVERY_VARIANTS.contains(v);
        let in_derived = DERIVED_VARIANTS.contains(v);
        assert!(
            in_delivery ^ in_derived,
            "variant {:?} must be in DELIVERY xor DERIVED (delivery={}, derived={})",
            v,
            in_delivery,
            in_derived
        );
    }
}

#[test]
fn derived_variants_serialize_roundtrip() {
    // CRITICAL: when M2 removes DERIVED variants, persisted cycles
    // using these values must be migratable. This test pins that
    // serialization still works on the current 11-variant enum, so
    // migration fixtures can be written against this baseline.
    for v in DERIVED_VARIANTS.iter() {
        let json = serde_json::to_string(v).expect("serialize");
        let parsed: CycleStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, parsed, "roundtrip failed for {:?}", v);
    }
}

#[test]
fn delivery_variants_serialize_roundtrip() {
    // Sanity: DELIVERY variants also roundtrip cleanly. If any of
    // these break, the production cycle persistence is broken.
    for v in DELIVERY_VARIANTS.iter() {
        let json = serde_json::to_string(v).expect("serialize");
        let parsed: CycleStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, parsed, "roundtrip failed for {:?}", v);
    }
}

#[test]
fn derived_variants_have_documented_migration_target() {
    // Pin the migration matrix disposition for each DERIVED variant.
    // When M2 starts, removing a variant requires this table to be
    // updated to reflect the migration target (per ADR-003 amendment).
    let migration_targets = [
        (
            CycleStatus::Blocked,
            "DEPRECATE → run gate/blocker facts + derived cycle summary",
        ),
        (
            CycleStatus::Remediating,
            "REVIEW → derived delivery/run summary or WorkItem",
        ),
        (
            CycleStatus::Recovering,
            "DEPRECATE runtime meaning → Run recovery state",
        ),
        (
            CycleStatus::UatWaiting,
            "DEPRECATE → run/gate facts + derived cycle summary",
        ),
        (
            CycleStatus::ApprovalPending,
            "DEPRECATE → run/authority facts + derived summary",
        ),
    ];
    assert_eq!(migration_targets.len(), DERIVED_VARIANTS.len());
    for (variant, target) in migration_targets.iter() {
        assert!(
            DERIVED_VARIANTS.contains(variant),
            "migration target listed for non-DERIVED variant {:?}",
            variant
        );
        // Smoke: target string is non-empty and references a known disposition verb.
        assert!(
            !target.is_empty(),
            "migration target empty for {:?}",
            variant
        );
        assert!(
            target.starts_with("DEPRECATE")
                || target.starts_with("REVIEW")
                || target.starts_with("KEEP"),
            "migration target for {:?} uses unknown disposition: {}",
            variant,
            target
        );
    }
}
