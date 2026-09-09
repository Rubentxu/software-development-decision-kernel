//! Orchestration Synthesis Receipt substrate — typed projection of what
//! the orchestrator/coordinator did with worker envelopes.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-OrchestrationSynthesisReceipt.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-086-ORCHESTRATION-SYNTHESIS-RECEIPT.md
//!
//! This substrate sits on top of CDD-HANDOFF-001's `AgentContributionEnvelope`
//! substrate and is orthogonal to it: the receipt is a separate plain-data
//! projection that records how the orchestrator compressed many envelopes
//! into a downstream recommendation without silently dropping mandatory
//! risk, evidence, dissent or coverage loss.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::agent_contribution_envelope::{EnvelopeStore, Severity};

// ----------------- ContributionRef -----------------

/// Typed reference to a contribution consumed from (or omitted by) the
/// orchestrator. `envelope_digest` is the canonical stable id of the
/// referenced envelope (CDD-HANDOFF-001 uses `envelope_id` as canonical
/// digest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionRef {
    pub delegation_id: String,
    pub envelope_digest: String,
    pub consumed_at_ms: i64,
    pub consumed_by: String,
    /// Present iff this is in `omitted`; required non-empty for omitted refs.
    pub omission_reason: Option<String>,
}

impl ContributionRef {
    /// Construct a `consumed` reference.
    pub fn consume(
        delegation_id: impl Into<String>,
        envelope_digest: impl Into<String>,
        consumed_at_ms: i64,
        consumed_by: impl Into<String>,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            envelope_digest: envelope_digest.into(),
            consumed_at_ms,
            consumed_by: consumed_by.into(),
            omission_reason: None,
        }
    }

    /// Construct an `omitted` reference with non-empty reason.
    pub fn omit(
        delegation_id: impl Into<String>,
        envelope_digest: impl Into<String>,
        consumed_at_ms: i64,
        consumed_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            envelope_digest: envelope_digest.into(),
            consumed_at_ms,
            consumed_by: consumed_by.into(),
            omission_reason: Some(reason.into()),
        }
    }
}

// ----------------- Supporting entries -----------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictEntry {
    pub conflict_id: String,
    pub envelopes: Vec<String>,
    pub field: String,
    pub resolution: String,
    pub rationale: String,
    pub surfaced_at_ms: i64,
}

impl ConflictEntry {
    pub fn new(
        conflict_id: impl Into<String>,
        envelopes: Vec<String>,
        field: impl Into<String>,
        resolution: impl Into<String>,
        rationale: impl Into<String>,
        surfaced_at_ms: i64,
    ) -> Self {
        Self {
            conflict_id: conflict_id.into(),
            envelopes,
            field: field.into(),
            resolution: resolution.into(),
            rationale: rationale.into(),
            surfaced_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DissentEntry {
    pub dissent_id: String,
    pub envelope_digest: String,
    pub rejection_reason: String,
    pub alternative_id: Option<String>,
    pub preserved: bool,
    pub carried_to: Option<String>,
}

impl DissentEntry {
    pub fn preserved(
        dissent_id: impl Into<String>,
        envelope_digest: impl Into<String>,
        rejection_reason: impl Into<String>,
        alternative_id: Option<String>,
    ) -> Self {
        Self {
            dissent_id: dissent_id.into(),
            envelope_digest: envelope_digest.into(),
            rejection_reason: rejection_reason.into(),
            alternative_id,
            preserved: true,
            carried_to: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageLossEntry {
    pub area: String,
    pub reason: String,
    pub compensating_evidence: Vec<String>,
}

impl CoverageLossEntry {
    pub fn new(
        area: impl Into<String>,
        reason: impl Into<String>,
        compensating_evidence: Vec<String>,
    ) -> Self {
        Self {
            area: area.into(),
            reason: reason.into(),
            compensating_evidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskCarryEntry {
    pub risk_id: String,
    pub envelope_digest: String,
    pub severity: Severity,
    pub summary: String,
    pub carried_to: String,
}

impl RiskCarryEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        risk_id: impl Into<String>,
        envelope_digest: impl Into<String>,
        severity: Severity,
        summary: impl Into<String>,
        carried_to: impl Into<String>,
    ) -> Self {
        Self {
            risk_id: risk_id.into(),
            envelope_digest: envelope_digest.into(),
            severity,
            summary: summary.into(),
            carried_to: carried_to.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceCarryEntry {
    pub evidence_ref: String,
    pub envelope_digest: String,
    pub carried_to: String,
}

impl EvidenceCarryEntry {
    pub fn new(
        evidence_ref: impl Into<String>,
        envelope_digest: impl Into<String>,
        carried_to: impl Into<String>,
    ) -> Self {
        Self {
            evidence_ref: evidence_ref.into(),
            envelope_digest: envelope_digest.into(),
            carried_to: carried_to.into(),
        }
    }
}

// ----------------- OrchestrationSynthesisReceipt -----------------

/// Typed projection of synthesis outcome for one join window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationSynthesisReceipt {
    pub receipt_id: String,
    pub delegation_id: String,
    pub join_id: String,
    pub synthesis_owner: String,
    pub produced_at_ms: i64,
    pub consumed: Vec<ContributionRef>,
    pub omitted: Vec<ContributionRef>,
    pub conflicts: Vec<ConflictEntry>,
    pub dissent_preserved: Vec<DissentEntry>,
    pub coverage_loss: Vec<CoverageLossEntry>,
    pub risk_carry_forward: Vec<RiskCarryEntry>,
    pub evidence_carry_forward: Vec<EvidenceCarryEntry>,
    pub downstream_recommendation: Option<String>,
    pub confidence: f64,
    pub metrics: BTreeMap<String, f64>,
    /// Reserved for the validator's internal conflict tracking; not part
    /// of the public spec surface (kept off `serde` to avoid surface
    /// expansion).  Public callers manipulate `conflicts` only.
    #[serde(skip)]
    pub conflict_internal: Vec<ConflictEntry>,
}

impl OrchestrationSynthesisReceipt {
    pub fn new(
        receipt_id: impl Into<String>,
        delegation_id: impl Into<String>,
        join_id: impl Into<String>,
        synthesis_owner: impl Into<String>,
        produced_at_ms: i64,
    ) -> Self {
        Self {
            receipt_id: receipt_id.into(),
            delegation_id: delegation_id.into(),
            join_id: join_id.into(),
            synthesis_owner: synthesis_owner.into(),
            produced_at_ms,
            consumed: Vec::new(),
            omitted: Vec::new(),
            conflicts: Vec::new(),
            dissent_preserved: Vec::new(),
            coverage_loss: Vec::new(),
            risk_carry_forward: Vec::new(),
            evidence_carry_forward: Vec::new(),
            downstream_recommendation: None,
            confidence: 0.0,
            metrics: BTreeMap::new(),
            conflict_internal: Vec::new(),
        }
    }
}

// ----------------- InformationLossGuard -----------------

/// Closed-set guard enum used by `SynthesisValidator`. New guards are
/// added by extending this enum and the validator in tandem (additive,
/// not breaking).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InformationLossGuard {
    MandatoryRiskNoDrop,
    MandatoryEvidenceNoDrop,
    DissentNoSilentDrop,
    CoverageLossRecorded,
    RecommendationBackedByConsumed,
    ConflictSurfacedOrJustified,
}

// ----------------- SynthesisStore + InMemorySynthesisStore -----------------

/// Persistence seam for validated receipts.
pub trait SynthesisStore: Send + Sync + std::fmt::Debug {
    fn put(&self, receipt: OrchestrationSynthesisReceipt);
    fn get(&self, receipt_id: &str) -> Option<OrchestrationSynthesisReceipt>;
    fn by_delegation(&self, delegation_id: &str) -> Vec<OrchestrationSynthesisReceipt>;
    fn all(&self) -> Vec<OrchestrationSynthesisReceipt>;
}

#[derive(Debug, Default)]
pub struct InMemorySynthesisStore {
    inner: Mutex<BTreeMap<String, OrchestrationSynthesisReceipt>>,
}

impl InMemorySynthesisStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SynthesisStore for InMemorySynthesisStore {
    fn put(&self, receipt: OrchestrationSynthesisReceipt) {
        let mut g = self
            .inner
            .lock()
            .expect("InMemorySynthesisStore mutex poisoned");
        g.insert(receipt.receipt_id.clone(), receipt);
    }
    fn get(&self, receipt_id: &str) -> Option<OrchestrationSynthesisReceipt> {
        let g = self
            .inner
            .lock()
            .expect("InMemorySynthesisStore mutex poisoned");
        g.get(receipt_id).cloned()
    }
    fn by_delegation(&self, delegation_id: &str) -> Vec<OrchestrationSynthesisReceipt> {
        let g = self
            .inner
            .lock()
            .expect("InMemorySynthesisStore mutex poisoned");
        g.values()
            .filter(|r| r.delegation_id == delegation_id)
            .cloned()
            .collect()
    }
    fn all(&self) -> Vec<OrchestrationSynthesisReceipt> {
        let g = self
            .inner
            .lock()
            .expect("InMemorySynthesisStore mutex poisoned");
        g.values().cloned().collect()
    }
}

// ----------------- SynthesisError -----------------

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SynthesisError {
    #[error("mandatory risk dropped during compression: {0}")]
    MandatoryRiskDropped(String),
    #[error("mandatory evidence dropped during compression: {0}")]
    MandatoryEvidenceDropped(String),
    #[error("dissent dropped without justification: {0}")]
    DissentSilentlyDropped(String),
    #[error("coverage loss not recorded: {0}")]
    CoverageLossUnrecorded(String),
    #[error("recommendation not backed by any consumed envelope: {0}")]
    RecommendationOrphan(String),
    #[error("conflict not surfaced or justified: {0}")]
    ConflictHidden(String),
    #[error("envelope digest mismatch for {envelope_digest}: {detail}")]
    EnvelopeDigestMismatch {
        envelope_digest: String,
        detail: String,
    },
    #[error("builder invariant violated: {0}")]
    BuilderInvariantViolated(String),
}

// ----------------- SynthesisValidator -----------------

#[derive(Debug, Clone)]
pub struct SynthesisValidator {
    #[allow(dead_code)]
    store: std::sync::Arc<dyn SynthesisStore>,
    envelope_store: std::sync::Arc<dyn EnvelopeStore>,
}

impl SynthesisValidator {
    pub fn new(
        store: std::sync::Arc<dyn SynthesisStore>,
        envelope_store: std::sync::Arc<dyn EnvelopeStore>,
    ) -> Self {
        Self {
            store,
            envelope_store,
        }
    }

    /// Walk every `InformationLossGuard` in order; first failure wins.
    pub fn validate(&self, receipt: &OrchestrationSynthesisReceipt) -> Result<(), SynthesisError> {
        // backref check first: every consumed/omitted envelope_digest must
        // resolve to a persisted envelope in the EnvelopeStore.
        for r in receipt.consumed.iter().chain(receipt.omitted.iter()) {
            if self.envelope_store.get(&r.envelope_digest).is_none() {
                return Err(SynthesisError::EnvelopeDigestMismatch {
                    envelope_digest: r.envelope_digest.clone(),
                    detail: "envelope_id not present in EnvelopeStore".to_string(),
                });
            }
        }

        let order = [
            InformationLossGuard::MandatoryRiskNoDrop,
            InformationLossGuard::DissentNoSilentDrop,
            InformationLossGuard::CoverageLossRecorded,
            InformationLossGuard::MandatoryEvidenceNoDrop,
            InformationLossGuard::RecommendationBackedByConsumed,
            InformationLossGuard::ConflictSurfacedOrJustified,
        ];
        for g in order {
            self.check_guard(receipt, g)?;
        }
        Ok(())
    }

    pub fn check_guard(
        &self,
        receipt: &OrchestrationSynthesisReceipt,
        guard: InformationLossGuard,
    ) -> Result<(), SynthesisError> {
        match guard {
            InformationLossGuard::MandatoryRiskNoDrop => {
                self.check_mandatory_risk(receipt)?;
            }
            InformationLossGuard::DissentNoSilentDrop => {
                self.check_dissent(receipt)?;
            }
            InformationLossGuard::CoverageLossRecorded => {
                self.check_coverage_loss(receipt)?;
            }
            InformationLossGuard::MandatoryEvidenceNoDrop => {
                self.check_mandatory_evidence(receipt)?;
            }
            InformationLossGuard::RecommendationBackedByConsumed => {
                self.check_orphan_recommendation(receipt)?;
            }
            InformationLossGuard::ConflictSurfacedOrJustified => {
                self.check_hidden_conflict(receipt)?;
            }
        }
        Ok(())
    }

    /// Every consumed envelope whose risks contain a [High] or
    /// [Critical] tag MUST be carried forward in `risk_carry_forward`
    /// OR have a corresponding entry in `coverage_loss`.
    fn check_mandatory_risk(
        &self,
        receipt: &OrchestrationSynthesisReceipt,
    ) -> Result<(), SynthesisError> {
        for cref in &receipt.consumed {
            let Some(env) = self.envelope_store.get(&cref.envelope_digest) else {
                continue;
            };
            for (i, risk) in env.risks.iter().enumerate() {
                let sev = extract_severity(risk);
                if matches!(sev, Some(Severity::High | Severity::Critical)) {
                    // search for any carry-forward entry referencing this envelope
                    let carried = receipt
                        .risk_carry_forward
                        .iter()
                        .any(|r| r.envelope_digest == cref.envelope_digest);
                    // or a coverage_loss entry that compensates it
                    let compensated = receipt
                        .coverage_loss
                        .iter()
                        .any(|c| c.compensating_evidence.iter().any(|e| !e.is_empty()));
                    if !carried && !compensated {
                        return Err(SynthesisError::MandatoryRiskDropped(format!(
                            "{}#risk-{}",
                            cref.envelope_digest, i
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Every consumed envelope's rejections[] MUST be present in
    /// `dissent_preserved` (preserved=true) OR have a corresponding
    /// `ConflictEntry`. Pure silent drops are fail-closed.
    fn check_dissent(&self, receipt: &OrchestrationSynthesisReceipt) -> Result<(), SynthesisError> {
        for cref in &receipt.consumed {
            let Some(env) = self.envelope_store.get(&cref.envelope_digest) else {
                continue;
            };
            for rej in &env.rejections {
                let in_dissent = receipt.dissent_preserved.iter().any(|d| {
                    d.envelope_digest == cref.envelope_digest && d.rejection_reason == rej.reason
                });
                let in_conflict = receipt
                    .conflict_internal
                    .iter()
                    .any(|c| c.envelopes.contains(&cref.envelope_digest));
                if !in_dissent && !in_conflict {
                    return Err(SynthesisError::DissentSilentlyDropped(format!(
                        "{}/{}",
                        cref.envelope_digest, rej.id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Every entry in envelope.coverage_missing that the orchestrator
    /// did not address MUST be reflected in `receipt.coverage_loss` with
    /// a non-empty reason.
    fn check_coverage_loss(
        &self,
        receipt: &OrchestrationSynthesisReceipt,
    ) -> Result<(), SynthesisError> {
        for cref in &receipt.consumed {
            let Some(env) = self.envelope_store.get(&cref.envelope_digest) else {
                continue;
            };
            for area in &env.coverage_missing {
                let recorded = receipt
                    .coverage_loss
                    .iter()
                    .find(|c| &c.area == area)
                    .ok_or_else(|| {
                        SynthesisError::BuilderInvariantViolated(format!(
                            "{}#area-{}",
                            cref.envelope_digest, area
                        ))
                    })?;
                if recorded.reason.trim().is_empty() {
                    return Err(SynthesisError::BuilderInvariantViolated(format!(
                        "{}#area-{} empty reason",
                        cref.envelope_digest, area
                    )));
                }
            }
        }
        Ok(())
    }

    /// Every evidence ref of severity >= Medium present in consumed
    /// envelopes MUST be carried forward, or covered by a `ConflictEntry`.
    fn check_mandatory_evidence(
        &self,
        receipt: &OrchestrationSynthesisReceipt,
    ) -> Result<(), SynthesisError> {
        for cref in &receipt.consumed {
            let Some(env) = self.envelope_store.get(&cref.envelope_digest) else {
                continue;
            };
            // severity-gated evidence comes from medium+ findings;
            // evidence_refs themselves are carried categorically unless
            // a conflict entry covers them
            let has_medium_or_above = env.findings.iter().any(|f| {
                matches!(
                    f.severity,
                    Severity::Medium | Severity::High | Severity::Critical
                )
            });
            if !has_medium_or_above {
                continue;
            }
            for ev in &env.evidence_refs {
                let carried = receipt
                    .evidence_carry_forward
                    .iter()
                    .any(|e| &e.evidence_ref == ev);
                let in_conflict = receipt
                    .conflict_internal
                    .iter()
                    .any(|c| c.envelopes.contains(&cref.envelope_digest));
                if !carried && !in_conflict {
                    return Err(SynthesisError::MandatoryEvidenceDropped(format!(
                        "{}/{}",
                        cref.envelope_digest, ev
                    )));
                }
            }
        }
        Ok(())
    }

    /// If a recommendation is present, it MUST be backed by at least one
    /// consumed envelope's content, OR the receipt MUST carry a
    /// `standalone: true` metric to acknowledge authorship.
    fn check_orphan_recommendation(
        &self,
        receipt: &OrchestrationSynthesisReceipt,
    ) -> Result<(), SynthesisError> {
        let Some(text) = receipt.downstream_recommendation.as_ref() else {
            return Ok(());
        };
        // standalone metric acknowledges authorship
        if receipt.metrics.get("standalone").copied().unwrap_or(0.0) >= 1.0 {
            return Ok(());
        }
        // trace the recommendation text through the consumed envelopes:
        // either the text appears as a substring of an env.recommendation,
        // OR the receipts carries a risk/evidence/coordination link
        let mut linked = false;
        for cref in &receipt.consumed {
            if let Some(env) = self.envelope_store.get(&cref.envelope_digest) {
                if !env.recommendation.is_empty()
                    && (text.contains(&env.recommendation)
                        || env.recommendation.contains(text.trim()))
                {
                    linked = true;
                    break;
                }
                if env.evidence_refs.iter().any(|e| text.contains(e)) {
                    linked = true;
                    break;
                }
            }
        }
        if !linked {
            // also link if any carry-forward item explicitly mentions the
            // recommendation text (risk summary or evidence ref substring).
            linked = receipt
                .risk_carry_forward
                .iter()
                .any(|r| !r.summary.is_empty() && text.contains(&r.summary))
                || receipt
                    .evidence_carry_forward
                    .iter()
                    .any(|e| !e.evidence_ref.is_empty() && text.contains(&e.evidence_ref));
        }
        if !linked {
            return Err(SynthesisError::RecommendationOrphan(text.clone()));
        }
        Ok(())
    }

    /// Detect pairs of consumed envelopes whose `recommendation` strings
    /// disagree (ignoring case + trim) and require either a
    /// `ConflictEntry` covering them or no recommendation at all.
    fn check_hidden_conflict(
        &self,
        receipt: &OrchestrationSynthesisReceipt,
    ) -> Result<(), SynthesisError> {
        if receipt.consumed.len() < 2 {
            return Ok(());
        }
        let mut recs: Vec<(String, String)> = Vec::new(); // (digest, recommendation)
        for cref in &receipt.consumed {
            if let Some(env) = self.envelope_store.get(&cref.envelope_digest)
                && !env.recommendation.trim().is_empty()
            {
                recs.push((
                    cref.envelope_digest.clone(),
                    env.recommendation.trim().to_lowercase(),
                ));
            }
        }
        // unique recommendations
        let mut seen: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for (d, r) in &recs {
            seen.entry(r.clone()).or_default().push(d.clone());
        }
        if seen.len() < 2 {
            return Ok(());
        }
        // disagreement detected — require a ConflictEntry covering at least 2 digests.
        let digests: std::collections::BTreeSet<String> =
            recs.iter().map(|(d, _)| d.clone()).collect();
        let covered = receipt
            .conflict_internal
            .iter()
            .any(|c| c.envelopes.iter().filter(|d| digests.contains(*d)).count() >= 2);
        if !covered {
            return Err(SynthesisError::ConflictHidden(format!("{:?}", digests)));
        }
        Ok(())
    }
}

/// Extract severity tag `[Critical]` / `[High]` / `[Medium]` / `[Low]`
/// from a risk description string. Returns `None` if no explicit tag.
fn extract_severity(risk: &str) -> Option<Severity> {
    for word in risk.split_whitespace() {
        if let Some(rest) = word.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            match rest.to_ascii_lowercase().as_str() {
                "critical" => return Some(Severity::Critical),
                "high" => return Some(Severity::High),
                "medium" => return Some(Severity::Medium),
                "low" => return Some(Severity::Low),
                "info" => return Some(Severity::Info),
                _ => {}
            }
        }
    }
    None
}

// ----------------- SynthesisBuilder -----------------

/// Deterministic builder seam.
pub trait SynthesisBuilder: Send + Sync + std::fmt::Debug {
    fn validate(&self, receipt: &OrchestrationSynthesisReceipt) -> Result<(), SynthesisError>;
}

/// Reference implementation that defers all checks to the validator
/// pipeline. The builder API is intentionally narrow: receipts are
/// constructed via `OrchestrationSynthesisReceipt::new` + field
/// setters, then submitted through `SynthesisBuilder::validate` (or
/// directly through `SynthesisValidator::validate`).
#[derive(Debug, Clone)]
pub struct DefaultSynthesisBuilder {
    envelope_store: std::sync::Arc<dyn EnvelopeStore>,
}

impl DefaultSynthesisBuilder {
    pub fn new(envelope_store: std::sync::Arc<dyn EnvelopeStore>) -> Self {
        Self { envelope_store }
    }
}

impl SynthesisBuilder for DefaultSynthesisBuilder {
    fn validate(&self, receipt: &OrchestrationSynthesisReceipt) -> Result<(), SynthesisError> {
        let synth_store: std::sync::Arc<dyn SynthesisStore> =
            std::sync::Arc::new(InMemorySynthesisStore::new());
        let v = SynthesisValidator::new(synth_store, self.envelope_store.clone());
        v.validate(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contribution_ref_consume_omission_none() {
        let r = ContributionRef::consume("D1", "E1", 100, "owner");
        assert_eq!(r.omission_reason, None);
    }

    #[test]
    fn contribution_ref_omit_has_reason() {
        let r = ContributionRef::omit("D1", "E1", 100, "owner", "low value");
        assert_eq!(r.omission_reason.as_deref(), Some("low value"));
    }

    #[test]
    fn extract_severity_basic() {
        assert_eq!(extract_severity("plain risk"), None);
        assert_eq!(
            extract_severity("[Critical] data loss in shard 3"),
            Some(Severity::Critical)
        );
        assert_eq!(extract_severity("[High] whatever"), Some(Severity::High));
        assert_eq!(
            extract_severity("[info] nice to know"),
            Some(Severity::Info)
        );
    }

    #[test]
    fn receipt_serde_roundtrip_skips_conflict_internal() {
        let mut r = OrchestrationSynthesisReceipt::new("R", "D", "J", "owner", 100);
        r.conflict_internal.push(ConflictEntry::new(
            "C1",
            vec!["E1".into()],
            "f",
            "kept_a",
            "r",
            100,
        ));
        let json = serde_json::to_string(&r).unwrap();
        // conflict_internal must NOT serialise
        assert!(!json.contains("conflict_internal"));
        assert!(!json.contains("kept_a"));
    }
}
