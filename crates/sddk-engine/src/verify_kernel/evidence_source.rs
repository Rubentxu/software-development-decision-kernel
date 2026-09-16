// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// verify_kernel/evidence_source.rs — A4-1: thin shared substrate trait.
//
// Cycle: `p-63676b11dc0ef88f/a4-1-generic-verify`
// Spec: `docs/architecture/specs/arch-spec-043-generic-verify.md`
// ADR: `docs/architecture/adrs/ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT.md`
//
// # Purpose
//
// `EvidenceSource` is the **thin shared substrate** between Verify and DebVerify.
// It exposes only `&ObservationSet` + `&[EvidenceRef]` — the universal substrate
// per ADR-0122 consumer-neutrality.
//
// Both kernels consume the same substrate; the *question* (claim vs scope) and
// *result* types are disjoint. This is the Q3 boundary (design.md §Decision Q3).
//
// # Anti-encroachment
//
// This trait cannot be extended to leak `VerificationClaim` or `DebVerifyScope`.
// It is intentionally minimal: adding methods here would create a coupling that
// the Q3 boundary is designed to prevent.

use crate::evidence_ref::EvidenceRef;
use crate::observation::ObservationSet;

/// A source of evidence: observations and evidence references.
///
/// Shared between Verify and DebVerify kernels. Each kernel decides how to
/// interpret the evidence; this trait only provides access to the substrate.
pub trait EvidenceSource {
    /// Returns the observation set this source provides.
    fn observations(&self) -> &ObservationSet;

    /// Returns the evidence references this source provides.
    fn evidence_refs(&self) -> &[EvidenceRef];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::ObservationSet;

    /// A minimal test source for pin tests.
    struct TestEvidenceSource {
        observations: ObservationSet,
        refs: Vec<EvidenceRef>,
    }

    impl EvidenceSource for TestEvidenceSource {
        fn observations(&self) -> &ObservationSet {
            &self.observations
        }
        fn evidence_refs(&self) -> &[EvidenceRef] {
            &self.refs
        }
    }

    #[test]
    fn evidence_source_exposes_observations_and_refs() {
        // ObservationSet uses Vec<SoftwareObservation>, create empty for this test
        let observations = ObservationSet::new();
        let source = TestEvidenceSource {
            observations,
            refs: vec![],
        };
        assert!(source.evidence_refs().is_empty());
    }
}
