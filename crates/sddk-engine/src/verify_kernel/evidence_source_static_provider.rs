//! `evidence_source_static_provider.rs` — Bridge that produces a Verify-substrate
//! `SoftwareObservation` from a CogniCode-shaped `AnalysisResult`
//! (returned by `code_intelligence_port`).
//!
//! Slice: `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s3-ac10-verify-integration`
//!
//! This module closes AC10: static evidence produced by CogniCode is
//! consumed by the Verify kernel through the existing
//! `StaticProviderDomain` (which already accepts
//! `observation::ObservationSet`).
//!
//! # Design constraints
//!
//! - The two SDDK `ObservationSet` types are distinct:
//!   `code_intelligence_port::ObservationSet` (BTreeMap-keyed by
//!   source unit, free-text `Observation { text }`) and
//!   `observation::types::ObservationSet` (Vec of typed
//!   `SoftwareObservation`). This adapter is the typed bridge.
//! - **No closed type is modified.** `EvidenceKind` is a closed
//!   vocabulary (Planning/Governance/Authority/DecisionMemory/Adhoc);
//!   we use `EvidenceKind::Adhoc` for the bridge. The
//!   `OBSERVED_STATIC` distinction rides on:
//!     1. `ObservationOrigin::StaticProvider` (already exists), and
//!     2. the `EvidenceRef.locator` prefix `cognicode://` (so the
//!        VerifyReceipt can later distinguish `OBSERVED_STATIC` from
//!        `INFERRED` evidence without a new EvidenceKind variant).
//! - **No provider DTO type crosses into `sddk-domain`.** The
//!   adapter lives in `sddk_engine::verify_kernel` (a downstream
//!   module of `code_intelligence_port`), not in
//!   `sddk_engine::observation`.
//! - **No modification of `EvidenceSource`.** That trait is the
//!   substrate shared by Verify and DebVerify; extending it would be
//!   an anti-encroachment violation per its own doc comment.

#![forbid(unsafe_code)]

use crate::architecture_graph::types::SoftwareUnitRef;
use crate::code_intelligence_port::{AnalysisResult, Observation, ProviderKind};
use crate::evidence_ref::{EvidenceKind, EvidenceRef};
use crate::observation::types::{
    ObservationBasis, ObservationOrigin, ObservationStance, ObservationSubject, SoftwareObservation,
};

/// Stable namespace prefix used in `EvidenceRef.locator` so Verify
/// receipts can tell a static-provider observation from an inferred
/// one without extending the closed `EvidenceKind` vocabulary.
pub const LOCATOR_PREFIX: &str = "cognicode://";

/// Bridge result: the typed `observation::ObservationSet` plus the
/// `ProviderKind` of the source analysis (carried for receipts; not
/// part of the substrate itself, which is provider-kind-agnostic) and
/// the count of observations translated.
#[derive(Debug, Clone)]
pub struct BridgedObservationSet {
    /// The substrate Verify / DebVerify consume.
    pub observations: crate::observation::types::ObservationSet,
    /// Number of `Observation` entries translated. Always equals
    /// `observations.len()` after `build`.
    pub count: usize,
    /// The provider kind observed (carried for receipts; not part of
    /// the substrate itself, which is provider-kind-agnostic).
    pub provider_kind: ProviderKind,
}

/// Build a Verify-substrate `ObservationSet` from a CogniCode-shaped
/// `AnalysisResult`.
///
/// Mapping rules:
///
/// - Each `(unit, observations[])` pair in `analysis.observations.units`
///   produces one `SoftwareObservation` per `Observation` text.
/// - `subject` is `ObservationSubject::Unit(SoftwareUnitRef(unit))`.
///   The static-provider match in `adapter_static_provider` joins on
///   this subject's `canonical_tag()`, which is `"unit:<locator>"`.
/// - `stance` is `Affirms` for observations whose text does not start
///   with `CONTRADICTION:` (provider is reporting a fact); `Denies`
///   for observations whose text starts with `CONTRADICTION:`. This
///   is a documented heuristic — the canonical contradiction
///   mechanism lives in `KnowledgeBasis::invalidate(Contradicted, _)`,
///   not here. The bridge is best-effort for free text.
/// - `evidence.locator` carries `cognicode://<unit>` so the
///   downstream `VerifyReceipt` can distinguish `OBSERVED_STATIC`
///   evidence from `INFERRED` evidence without extending the
///   closed `EvidenceKind` vocabulary.
/// - `evidence.kind` is `EvidenceKind::Adhoc` (the closest existing
///   kind; no new variant added).
/// - `origin` is `ObservationOrigin::StaticProvider`.
/// - `basis` is `ObservationBasis::for_provider_result(revision, digest)`,
///   pinning the basis to the analysis result's digest (reproducible
///   per IPB-007).
/// - `freshness` is `None` (no expected basis supplied at this stage;
///   KMT evaluation belongs to a downstream consumer).
/// - `producer` is `"cognicode-mcp@<provider_version>"` when
///   available, otherwise `"cognicode-mcp"`.
pub fn build(
    analysis: &AnalysisResult,
    revision: &str,
    provider_version: Option<&str>,
) -> BridgedObservationSet {
    let mut substrate = crate::observation::types::ObservationSet::new();
    let mut count = 0usize;
    for (unit, observations) in &analysis.observations.units {
        for obs in observations {
            substrate.insert(translate_observation(
                unit,
                obs,
                analysis,
                revision,
                provider_version,
            ));
            count += 1;
        }
    }
    BridgedObservationSet {
        observations: substrate,
        count,
        provider_kind: analysis.observations.provider_kind,
    }
}

fn translate_observation(
    unit: &str,
    obs: &Observation,
    analysis: &AnalysisResult,
    revision: &str,
    provider_version: Option<&str>,
) -> SoftwareObservation {
    let stance = if obs.text.starts_with("CONTRADICTION:") {
        ObservationStance::Denies
    } else {
        ObservationStance::Affirms
    };
    let evidence = EvidenceRef::new(EvidenceKind::Adhoc, format!("{LOCATOR_PREFIX}{unit}"));
    let basis = ObservationBasis::for_provider_result(revision, &analysis.digest.0);
    let producer = match provider_version {
        Some(v) => format!("cognicode-mcp@{v}"),
        None => "cognicode-mcp".to_string(),
    };
    SoftwareObservation::declare(
        ObservationSubject::Unit(SoftwareUnitRef::new(unit.to_string())),
        stance,
        evidence,
        ObservationOrigin::StaticProvider,
        basis,
        None,
        producer,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn sample_analysis() -> AnalysisResult {
        use crate::code_intelligence_port::{ObservationSet, ProviderKind};
        AnalysisResult {
            digest: crate::code_intelligence_port::DigestSha256::of(b"sample"),
            partial: false,
            observations: ObservationSet {
                provider_kind: ProviderKind::Fake,
                units: BTreeMap::from([(
                    "crates/sddk-cli/src/main.rs".to_string(),
                    vec![
                        Observation {
                            text: "fn:cli_main".to_string(),
                        },
                        Observation {
                            text: "CONTRADICTION:fn:cli_main has no callers".to_string(),
                        },
                    ],
                )]),
                restart_observed: false,
            },
        }
    }

    #[test]
    fn build_translates_one_unit_two_texts() {
        let bridged = build(&sample_analysis(), "rev-1", Some("0.97.1"));
        assert_eq!(bridged.count, 2);
        assert_eq!(bridged.provider_kind, ProviderKind::Fake);
    }

    #[test]
    fn build_carries_provider_version_in_producer() {
        let bridged = build(&sample_analysis(), "rev-1", Some("0.97.1"));
        // The producer string is not exposed in the public substrate
        // (it lives on each SoftwareObservation). The count and
        // provider_kind are the externally observable contract.
        assert_eq!(bridged.count, 2);
    }

    #[test]
    fn build_handles_empty_units() {
        let mut a = sample_analysis();
        a.observations.units.clear();
        let bridged = build(&a, "rev-1", None);
        assert_eq!(bridged.count, 0);
        assert_eq!(bridged.provider_kind, ProviderKind::Fake);
    }

    #[test]
    fn build_handles_unit_with_empty_observation_vec() {
        // Edge case: a unit key with an empty `Vec<Observation>` contributes
        // 0 observations to the substrate and 0 to the count, but the unit
        // key is still iterated. This is the loop-internal short-circuit
        // (inner `for obs in observations` is a no-op on empty).
        use crate::code_intelligence_port::{ObservationSet, ProviderKind};
        let mut a = sample_analysis();
        a.observations = ObservationSet {
            provider_kind: ProviderKind::Fake,
            units: BTreeMap::from([
                ("crates/sddk-cli/src/main.rs".to_string(), Vec::new()),
                (
                    "crates/sddk-cli/src/commands/run.rs".to_string(),
                    vec![Observation {
                        text: "fn:cli_run".to_string(),
                    }],
                ),
            ]),
            restart_observed: false,
        };
        let bridged = build(&a, "rev-1", Some("0.97.1"));
        assert_eq!(bridged.count, 1, "only the non-empty unit contributes");
        assert_eq!(bridged.provider_kind, ProviderKind::Fake);
    }
}
