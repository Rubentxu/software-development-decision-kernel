// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/tests.rs — A4-4b internal unit tests.
//
// These exercise the kernel without going through the integration test
// fixture. They are intentionally minimal; the load-bearing pin tests
// (20 in total) live in
// `crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs` so they
// exercise the kernel and registry end-to-end with the reference
// lenses.

use super::error::LensError;
use super::registry::AlignmentLensRegistry;
use super::types::{LensId, LensVersion};
use crate::intent_universal_concern::UniversalConcern;
use std::collections::BTreeSet;

#[test]
fn empty_registry_has_no_concerns() {
    let r = AlignmentLensRegistry::new();
    assert_eq!(r.len(), 0);
    assert!(r.is_empty());
    assert!(r.descriptors_owned().is_empty());
    assert!(
        r.for_concern(UniversalConcern::DependencyDirection)
            .is_empty()
    );
}

#[test]
fn ids_round_trip_through_descriptor() {
    let r = AlignmentLensRegistry::new();
    let id = LensId::new("phantom_test_lens");
    let _version = LensVersion::new(1, 0);
    // Not registered. Just sanity-check the type API.
    assert!(!r.contains(id));
    assert!(r.lookup(id).is_none());
}

#[test]
fn duplicate_lens_id_refused() {
    // Re-registering a lens with the SAME id + version + supported_concerns
    // is idempotent (no kernel churn). To force a typed refusal we register
    // the second time with a DIFFERENT supported_concerns set, which trips
    // `InconsistentSupportedConcerns`. The test asserts the typed error.
    use super::lens::{AlignmentLens, LensEvaluationOutcome};
    use super::types::{LensDescriptor, LensInput};

    #[derive(Debug)]
    struct ZeroLensCohesion;
    impl AlignmentLens for ZeroLensCohesion {
        fn descriptor(&self) -> LensDescriptor {
            let mut s = BTreeSet::new();
            s.insert(UniversalConcern::Cohesion);
            LensDescriptor::new(LensId::new("phantom_zero"), LensVersion::new(1, 0), s)
        }
        fn evaluate(&self, _input: &LensInput) -> LensEvaluationOutcome {
            LensEvaluationOutcome::Refused(LensError::LensRejected {
                id: LensId::new("phantom_zero"),
                concern: UniversalConcern::Cohesion,
            })
        }
    }

    #[derive(Debug)]
    struct ZeroLensFreshness;
    impl AlignmentLens for ZeroLensFreshness {
        fn descriptor(&self) -> LensDescriptor {
            let mut s = BTreeSet::new();
            // Different concern set, same id, same version -> refused.
            s.insert(UniversalConcern::Freshness);
            LensDescriptor::new(LensId::new("phantom_zero"), LensVersion::new(1, 0), s)
        }
        fn evaluate(&self, _input: &LensInput) -> LensEvaluationOutcome {
            LensEvaluationOutcome::Refused(LensError::LensRejected {
                id: LensId::new("phantom_zero"),
                concern: UniversalConcern::Freshness,
            })
        }
    }

    let r1 = AlignmentLensRegistry::new()
        .register(ZeroLensCohesion)
        .expect("first registration succeeds");
    let err = r1
        .register(ZeroLensFreshness)
        .expect_err("different supported_concerns under same id must be refused");
    match err {
        LensError::InconsistentSupportedConcerns { id, .. } => {
            assert_eq!(id.as_str(), "phantom_zero");
        }
        other => panic!("expected InconsistentSupportedConcerns, got {:?}", other),
    }
}
