//! In-process deterministic `CodeIntelligencePort` implementation
//! used by the CC-S0 falsification battery.
//!
//! Two variants are exported:
//!
//! - [`FakeCodeIntelligenceProvider`] — a configurable provider
//!   that honours the protocol-mismatch, cancellation, and
//!   restart-mid-request failure paths exercised by the
//!   battery.
//! - [`NullCodeIntelligenceProvider`] — a no-op provider that
//!   always reports `BASE` and returns empty observation sets.
//!   Used to verify IPB-010 (Base mode is first-class).
//!
//! Both are SDDK-internal; they do not depend on any CogniCode /
//! chronos / prost / tonic token.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use crate::code_intelligence_port::{
    AnalysisBasis, AnalysisResult, CapabilityProfile, CapabilitySnapshot, CodeIntelligencePort,
    CodeIntelligencePortError, CoverageBasis, CoverageContract, CoverageEvaluation, DigestSha256,
    EvidenceGap, ImpactRequest, Observation, ObservationSet, ProviderKind, ProviderLifecycle,
    ScopeRequest, default_coverage_evaluation,
};

/// Deterministic in-process provider used by the spike tests.
#[derive(Debug, Clone)]
pub struct FakeCodeIntelligenceProvider {
    build: String,
    analyzer_set: Vec<String>,
    forced_protocol_major: Option<u32>,
    cancel_after_units: Option<usize>,
    restart_after_units: Option<usize>,
    sddk_protocol_major: u32,
}

impl FakeCodeIntelligenceProvider {
    /// Construct a new fake provider with the given build
    /// identifier and analyzer set.
    pub fn new(build: &str, analyzer_set: &[&str]) -> Self {
        Self {
            build: build.into(),
            analyzer_set: analyzer_set.iter().map(|s| s.to_string()).collect(),
            forced_protocol_major: None,
            cancel_after_units: None,
            restart_after_units: None,
            sddk_protocol_major: 1,
        }
    }

    /// Force the provider to advertise a different protocol major
    /// than SDDK's. Used by T3 to verify the INCOMPATIBLE path.
    pub fn force_protocol_major(&mut self, major: u32) {
        self.forced_protocol_major = Some(major);
    }

    /// Arm cancellation after N source units have been processed.
    /// Used by T4.
    pub fn arm_cancel_after_one_unit(&mut self) {
        self.cancel_after_units = Some(1);
    }

    /// Arm a provider restart after N source units. Used by T5.
    pub fn arm_restart_after_one_unit(&mut self) {
        self.restart_after_units = Some(1);
    }

    fn snapshot(&self) -> CapabilitySnapshot {
        let analyzer_refs: Vec<&str> = self.analyzer_set.iter().map(String::as_str).collect();
        let profile = if self.forced_protocol_major.is_some() {
            // Protocol-major mismatch: provider cannot advertise
            // STATIC_ENHANCED; suppress the profile (T3).
            CapabilityProfile::Base
        } else {
            CapabilityProfile::StaticEnhanced
        };
        // `self.build` is consumed by `analyze_delta`/`analyze_impact`
        // through the `AnalysisBasis.provider_build` field; it
        // is stored on the struct so the field is observable
        // from tests without re-deriving it from the basis.
        let _ = &self.build;
        CapabilitySnapshot::from_advertised(profile, &analyzer_refs)
    }

    fn build_observations(
        &self,
        request: &ScopeRequest,
        partial: bool,
        restart_observed: bool,
    ) -> ObservationSet {
        let mut units: BTreeMap<String, Vec<Observation>> = BTreeMap::new();
        let mut all = Vec::with_capacity(
            request.added_units.len() + request.modified_units.len() + request.removed_units.len(),
        );
        all.extend(request.added_units.iter().cloned());
        all.extend(request.modified_units.iter().cloned());
        all.extend(request.removed_units.iter().cloned());
        for unit in all {
            units.insert(
                unit.clone(),
                vec![Observation {
                    text: format!("fake:{}:ok", unit),
                }],
            );
        }
        ObservationSet {
            provider_kind: ProviderKind::Fake,
            units,
            restart_observed,
        }
        .tap_partial(partial)
    }

    fn check_mismatch(&self) -> Result<(), CodeIntelligencePortError> {
        if let Some(provider_major) = self.forced_protocol_major
            && provider_major != self.sddk_protocol_major
        {
            return Err(CodeIntelligencePortError::ProtocolMajorMismatch {
                provider_major,
                sddk_major: self.sddk_protocol_major,
            });
        }
        Ok(())
    }
}

impl CodeIntelligencePort for FakeCodeIntelligenceProvider {
    fn capabilities(&self) -> CapabilitySnapshot {
        self.snapshot()
    }

    fn lifecycle_state(&self) -> ProviderLifecycle {
        if self.forced_protocol_major.is_some() {
            ProviderLifecycle::Incompatible
        } else {
            ProviderLifecycle::Ready
        }
    }

    fn analyze_delta(
        &self,
        basis: &AnalysisBasis,
        request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        self.check_mismatch()?;
        let total =
            request.added_units.len() + request.modified_units.len() + request.removed_units.len();
        if let Some(n) = self.cancel_after_units
            && total > n
        {
            return Err(CodeIntelligencePortError::Cancelled);
        }
        let partial = self.restart_after_units.map(|n| total > n).unwrap_or(false);
        let observations = self.build_observations(request, partial, partial);
        let digest = compute_digest(basis, request, &observations, partial);
        Ok(AnalysisResult {
            digest,
            partial,
            observations,
        })
    }

    fn analyze_scope(
        &self,
        basis: &AnalysisBasis,
        request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        self.analyze_delta(basis, request)
    }

    fn analyze_impact(
        &self,
        basis: &AnalysisBasis,
        request: &ImpactRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        self.check_mismatch()?;
        let observations = ObservationSet {
            provider_kind: ProviderKind::Fake,
            units: BTreeMap::new(),
            restart_observed: false,
        };
        let synthetic = ScopeRequest {
            added_units: request.changed_units.clone(),
            removed_units: Vec::new(),
            modified_units: Vec::new(),
        };
        let digest = compute_digest(basis, &synthetic, &observations, false);
        Ok(AnalysisResult {
            digest,
            partial: false,
            observations,
        })
    }

    fn coverage_evaluation(
        &self,
        contract: &CoverageContract,
        basis: &CoverageBasis,
    ) -> Result<CoverageEvaluation, EvidenceGap> {
        // AR-4: provider Incompatible (forced protocol mismatch)
        // -> EvidenceGap::ProviderIncompatible.
        if let Some(provider_major) = self.forced_protocol_major {
            return Err(EvidenceGap::ProviderIncompatible {
                provider_major,
                sddk_major: 1,
            });
        }
        // AR-4: provider Ready -> default evaluation.
        Ok(default_coverage_evaluation(
            contract,
            basis,
            &self.snapshot(),
        ))
    }
}

/// Null provider — always advertises `BASE` and returns empty
/// observation sets. Used by T6 to verify that SDDK Verify
/// paths are unchanged when the provider is `UNAVAILABLE`
/// (`arch-spec-021` IPB-010).
#[derive(Debug, Clone, Copy, Default)]
pub struct NullCodeIntelligenceProvider;

impl CodeIntelligencePort for NullCodeIntelligenceProvider {
    fn capabilities(&self) -> CapabilitySnapshot {
        CapabilitySnapshot::from_advertised(CapabilityProfile::Base, &[])
    }

    fn lifecycle_state(&self) -> ProviderLifecycle {
        ProviderLifecycle::Unavailable
    }

    fn analyze_delta(
        &self,
        _basis: &AnalysisBasis,
        _request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        Ok(AnalysisResult {
            digest: DigestSha256::of(b"null:delta"),
            partial: false,
            observations: ObservationSet {
                provider_kind: ProviderKind::Null,
                units: BTreeMap::new(),
                restart_observed: false,
            },
        })
    }

    fn analyze_scope(
        &self,
        _basis: &AnalysisBasis,
        _request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        Ok(AnalysisResult {
            digest: DigestSha256::of(b"null:scope"),
            partial: false,
            observations: ObservationSet {
                provider_kind: ProviderKind::Null,
                units: BTreeMap::new(),
                restart_observed: false,
            },
        })
    }

    fn analyze_impact(
        &self,
        _basis: &AnalysisBasis,
        _request: &ImpactRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        Ok(AnalysisResult {
            digest: DigestSha256::of(b"null:impact"),
            partial: false,
            observations: ObservationSet {
                provider_kind: ProviderKind::Null,
                units: BTreeMap::new(),
                restart_observed: false,
            },
        })
    }

    fn coverage_evaluation(
        &self,
        _contract: &CoverageContract,
        _basis: &CoverageBasis,
    ) -> Result<CoverageEvaluation, EvidenceGap> {
        // AR-4: provider Unavailable -> EvidenceGap, never Satisfied.
        Err(EvidenceGap::ProviderUnavailable {
            lifecycle: ProviderLifecycle::Unavailable,
        })
    }
}

fn compute_digest(
    basis: &AnalysisBasis,
    request: &ScopeRequest,
    observations: &ObservationSet,
    partial: bool,
) -> DigestSha256 {
    let mut buf = Vec::new();
    buf.extend_from_slice(basis.source_revision.as_bytes());
    buf.push(b'|');
    buf.extend_from_slice(basis.request_scope.as_bytes());
    buf.push(b'|');
    buf.extend_from_slice(basis.provider_build.as_bytes());
    buf.push(b'|');
    buf.extend_from_slice(basis.analyzer_set_digest.0.as_bytes());
    buf.push(b'|');
    for u in &request.added_units {
        buf.extend_from_slice(u.as_bytes());
        buf.push(b',');
    }
    buf.push(b';');
    for u in &request.removed_units {
        buf.extend_from_slice(u.as_bytes());
        buf.push(b',');
    }
    buf.push(b';');
    for u in &request.modified_units {
        buf.extend_from_slice(u.as_bytes());
        buf.push(b',');
    }
    buf.push(b';');
    buf.push(if partial { b'1' } else { b'0' });
    buf.push(b'|');
    for (k, vs) in &observations.units {
        buf.extend_from_slice(k.as_bytes());
        buf.push(b'=');
        for v in vs {
            buf.extend_from_slice(v.text.as_bytes());
            buf.push(b',');
        }
        buf.push(b';');
    }
    DigestSha256::of(&buf)
}

trait ObservationSetExt {
    fn tap_partial(self, partial: bool) -> Self;
}

impl ObservationSetExt for ObservationSet {
    fn tap_partial(mut self, partial: bool) -> Self {
        // `partial` is already on `AnalysisResult`. For the
        // observation set, we record the `restart_observed`
        // signal separately when the result is partial.
        if partial {
            self.restart_observed = true;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_advertises_static_enhanced_by_default() {
        let fake = FakeCodeIntelligenceProvider::new("b", &["rust"]);
        assert_eq!(
            fake.capabilities().profile,
            CapabilityProfile::StaticEnhanced
        );
        assert_eq!(fake.lifecycle_state(), ProviderLifecycle::Ready);
    }

    #[test]
    fn null_advertises_base_and_unavailable() {
        let null = NullCodeIntelligenceProvider;
        assert_eq!(null.capabilities().profile, CapabilityProfile::Base);
        assert_eq!(null.lifecycle_state(), ProviderLifecycle::Unavailable);
    }
}
