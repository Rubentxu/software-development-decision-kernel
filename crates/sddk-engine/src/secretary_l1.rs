//! Secretary L1 Closed-Set Proposal substrate — bounded proposals
//! routed through existing CDD/HX seams.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-SecretaryL1ClosedSetProposals.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-090-SECRETARY-L1-CLOSED-SET-PROPOSALS.md

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::risk_approval_policy::RiskTier;

/// Closed-set kinds of L1 proposals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClosedSetKind {
    SuggestCandidate,
    SuggestRehydrationStep,
    SuggestHumanDecision,
    AcknowledgeDecision,
    EscalateProposal,
}

/// Opaque Secretary identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecretaryId(pub String);

impl SecretaryId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Template-defined issuance window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedWindow {
    pub issued_by: SecretaryId,
    pub valid_from_ms: i64,
    pub valid_until_ms: i64,
    pub max_uses: u32,
}

impl BoundedWindow {
    pub fn new(
        issued_by: SecretaryId,
        valid_from_ms: i64,
        valid_until_ms: i64,
        max_uses: u32,
    ) -> Self {
        Self {
            issued_by,
            valid_from_ms,
            valid_until_ms,
            max_uses,
        }
    }
}

/// Templated proposal shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalTemplate {
    pub template_id: String,
    pub kind: ClosedSetKind,
    pub summary: String,
    pub required_evidence_tier: RiskTier,
    pub bounded: BoundedWindow,
}

impl ProposalTemplate {
    pub fn new(
        template_id: impl Into<String>,
        kind: ClosedSetKind,
        summary: impl Into<String>,
        required_evidence_tier: RiskTier,
        bounded: BoundedWindow,
    ) -> Self {
        Self {
            template_id: template_id.into(),
            kind,
            summary: summary.into(),
            required_evidence_tier,
            bounded,
        }
    }
}

/// What was actually observed behind an issuance (ADR-0156, C3m.4).
///
/// Replaces the `confidence: f64` this type carried. That number was measured
/// (INC-DEBT-066) to have **no consumer in production**: the only production
/// caller was the storage-snapshot consumer, which derived `0.95` from
/// `log_head > 0` and `0.5` otherwise, and the only readers of the result were
/// the two tests asserting those two constants. *The number existed because a
/// test asserted it, and the test asserted it because the number existed.*
///
/// The states are the ones the roadmap names for C3m.4, and each one says
/// something a magnitude cannot:
///
/// - `Observed` — the source was there and was read. **Not** "0.95 sure": a
///   snapshot with `log_head = 42` is exactly as observed as one with 43, and
///   no amount of log makes an observation more observed.
/// - `Empty` — the source was there and was read, and it held nothing. This is
///   **not** "half as good as observed": it is a different fact, and calling it
///   0.5 is what made an empty ledger look like a weak observation instead of
///   an absence of one.
/// - `Stale` / `Conflicted` / `Missing` — the source was not usable as-is, and
///   each of those needs a different response from the consumer. A single
///   number cannot distinguish them at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EvidenceState {
    /// No source was consulted: the issuance is not evidence-backed.
    #[default]
    Missing,
    /// The source was read and had content.
    Observed,
    /// The source was read and was empty.
    Empty,
    /// The source was read but is behind the issuing time.
    Stale,
    /// Two sources disagree and nothing adjudicated them.
    Conflicted,
}

impl fmt::Display for EvidenceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EvidenceState::Missing => "MISSING",
            EvidenceState::Observed => "OBSERVED",
            EvidenceState::Empty => "EMPTY",
            EvidenceState::Stale => "STALE",
            EvidenceState::Conflicted => "CONFLICTED",
        })
    }
}

/// An issuance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretaryProposal {
    pub proposal_id: String,
    pub template_id: String,
    pub kind: ClosedSetKind,
    pub summary: String,
    pub coverage_satisfied: Vec<String>,
    pub coverage_missing: Vec<String>,
    pub evidence_refs: Vec<String>,
    /// What was observed behind this issuance (ADR-0156). Replaces the
    /// `confidence: f64` that INC-DEBT-066 measured as having no consumer.
    pub evidence: EvidenceState,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
}

/// Error taxonomy (closed-set, `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum SecretaryL1Error {
    UnknownTemplate {
        template_id: String,
    },
    TemplateExpired {
        template_id: String,
        valid_until_ms: i64,
    },
    MaxUsesReached {
        template_id: String,
        max_uses: u32,
    },
    EmptyTemplateId,
    OutOfWindow {
        now_ms: i64,
        valid_from_ms: i64,
        valid_until_ms: i64,
    },
    InvalidConfidence {
        value: f64,
    },
    EmptyCoverage {
        template_id: String,
    },
    InvalidWindow {
        valid_from_ms: i64,
        valid_until_ms: i64,
    },
}

impl std::fmt::Display for SecretaryL1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretaryL1Error::UnknownTemplate { template_id } => {
                write!(f, "unknown template {template_id}")
            }
            SecretaryL1Error::TemplateExpired {
                template_id,
                valid_until_ms,
            } => write!(f, "template {template_id} expired at {valid_until_ms}"),
            SecretaryL1Error::MaxUsesReached {
                template_id,
                max_uses,
            } => write!(f, "template {template_id} reached {max_uses} uses"),
            SecretaryL1Error::EmptyTemplateId => write!(f, "template_id is empty"),
            SecretaryL1Error::OutOfWindow {
                now_ms,
                valid_from_ms,
                valid_until_ms,
            } => write!(
                f,
                "now {now_ms} outside [{valid_from_ms}, {valid_until_ms}]"
            ),
            SecretaryL1Error::InvalidConfidence { value } => {
                write!(f, "confidence {value} out of [0,1]")
            }
            SecretaryL1Error::EmptyCoverage { template_id } => {
                write!(f, "template {template_id} requires non-empty coverage")
            }
            SecretaryL1Error::InvalidWindow {
                valid_from_ms,
                valid_until_ms,
            } => write!(f, "invalid window [{}, {}]", valid_from_ms, valid_until_ms),
        }
    }
}

impl std::error::Error for SecretaryL1Error {}

/// Deterministic L1 engine.
#[derive(Debug, Default)]
pub struct SecretaryL1Engine {
    templates: Mutex<BTreeMap<String, ProposalTemplate>>,
    proposals: Mutex<BTreeMap<String, SecretaryProposal>>,
    uses: Mutex<BTreeMap<String, u32>>,
}

impl SecretaryL1Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_template(&self, t: ProposalTemplate) -> Result<(), SecretaryL1Error> {
        if t.template_id.trim().is_empty() {
            return Err(SecretaryL1Error::EmptyTemplateId);
        }
        if t.bounded.valid_from_ms > t.bounded.valid_until_ms {
            return Err(SecretaryL1Error::InvalidWindow {
                valid_from_ms: t.bounded.valid_from_ms,
                valid_until_ms: t.bounded.valid_until_ms,
            });
        }
        let mut g = self
            .templates
            .lock()
            .expect("SecretaryL1Engine templates poisoned");
        g.insert(t.template_id.clone(), t);
        Ok(())
    }

    pub fn list_templates(&self) -> Vec<ProposalTemplate> {
        let g = self
            .templates
            .lock()
            .expect("SecretaryL1Engine templates poisoned");
        g.values().cloned().collect()
    }

    pub fn proposal_count(&self) -> usize {
        let g = self
            .proposals
            .lock()
            .expect("SecretaryL1Engine proposals poisoned");
        g.len()
    }

    /// Issue a proposal under a registered template.
    #[allow(clippy::too_many_arguments)]
    pub fn propose(
        &self,
        now_ms: i64,
        template_id: &str,
        evidence_refs: Vec<String>,
        coverage_satisfied: Vec<String>,
        coverage_missing: Vec<String>,
        summary: impl Into<String>,
        evidence: EvidenceState,
    ) -> Result<SecretaryProposal, SecretaryL1Error> {
        // Sin validacion de rango: un enum es valido o no lo es. El campo que
        // ocupaba este sitio (`confidence: f64`) tenia una comprobacion
        // `0.0..=1.0` y un error `InvalidConfidence` que solo existia para
        // decidir si un numero arbitrario caia dentro de un intervalo. La
        // comprobacion desaparece CON el numero, no por descuido: no hay ningun
        // rango que un estado evidencial pueda incumplir.
        let g = self
            .templates
            .lock()
            .expect("SecretaryL1Engine templates poisoned");
        let t = g
            .get(template_id)
            .ok_or_else(|| SecretaryL1Error::UnknownTemplate {
                template_id: template_id.to_string(),
            })?;
        // (8) out-of-window
        if now_ms < t.bounded.valid_from_ms || now_ms > t.bounded.valid_until_ms {
            return Err(SecretaryL1Error::OutOfWindow {
                now_ms,
                valid_from_ms: t.bounded.valid_from_ms,
                valid_until_ms: t.bounded.valid_until_ms,
            });
        }
        // (3) coverage required
        if coverage_satisfied.is_empty() && coverage_missing.is_empty() {
            return Err(SecretaryL1Error::EmptyCoverage {
                template_id: template_id.to_string(),
            });
        }
        // (4) tier-gated evidence
        if matches!(t.required_evidence_tier, RiskTier::High) && evidence_refs.is_empty() {
            return Err(SecretaryL1Error::EmptyCoverage {
                template_id: template_id.to_string(),
            });
        }
        // (7) max_uses check
        let mut uses_g = self.uses.lock().expect("SecretaryL1Engine uses poisoned");
        let used = uses_g.get(template_id).copied().unwrap_or(0);
        if t.bounded.max_uses != u32::MAX && used >= t.bounded.max_uses {
            return Err(SecretaryL1Error::MaxUsesReached {
                template_id: template_id.to_string(),
                max_uses: t.bounded.max_uses,
            });
        }
        uses_g.insert(template_id.to_string(), used + 1);

        let proposal = SecretaryProposal {
            proposal_id: format!("{}-{:08x}", template_id, used + 1),
            template_id: template_id.to_string(),
            kind: t.kind,
            summary: summary.into(),
            coverage_satisfied,
            coverage_missing,
            evidence_refs,
            evidence,
            issued_at_ms: now_ms,
            expires_at_ms: t.bounded.valid_until_ms,
        };
        drop(uses_g);
        drop(g);

        let mut p_g = self
            .proposals
            .lock()
            .expect("SecretaryL1Engine proposals poisoned");
        p_g.insert(proposal.proposal_id.clone(), proposal.clone());
        Ok(proposal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_with_uses(template_id: &str, max_uses: u32) -> ProposalTemplate {
        ProposalTemplate::new(
            template_id,
            ClosedSetKind::SuggestCandidate,
            "suggest a candidate",
            RiskTier::Trivial,
            BoundedWindow::new(SecretaryId::new("sec-1"), 1_000, 10_000, max_uses),
        )
    }

    #[test]
    fn empty_template_id_rejected() {
        let e = SecretaryL1Engine::new();
        let t = ProposalTemplate::new(
            "",
            ClosedSetKind::SuggestCandidate,
            "x",
            RiskTier::Trivial,
            BoundedWindow::new(SecretaryId::new("sec-1"), 0, 1, 1),
        );
        let err = e.register_template(t).unwrap_err();
        assert!(matches!(err, SecretaryL1Error::EmptyTemplateId));
    }

    #[test]
    fn invalid_window_rejected() {
        let e = SecretaryL1Engine::new();
        let t = ProposalTemplate::new(
            "t1",
            ClosedSetKind::SuggestCandidate,
            "x",
            RiskTier::Trivial,
            BoundedWindow::new(SecretaryId::new("sec-1"), 2_000, 1_000, 1),
        );
        let err = e.register_template(t).unwrap_err();
        assert!(matches!(err, SecretaryL1Error::InvalidWindow { .. }));
    }

    #[test]
    fn propose_in_window() {
        let e = SecretaryL1Engine::new();
        e.register_template(template_with_uses("t1", u32::MAX))
            .unwrap();
        let p = e
            .propose(
                1_500,
                "t1",
                vec!["cid://a".into()],
                vec!["c1".into()],
                Vec::new(),
                "first proposal",
                EvidenceState::Observed,
            )
            .unwrap();
        assert_eq!(p.kind, ClosedSetKind::SuggestCandidate);
        assert_eq!(p.evidence, EvidenceState::Observed);
        assert_eq!(e.proposal_count(), 1);
    }

    #[test]
    fn out_of_window_rejected() {
        let e = SecretaryL1Engine::new();
        e.register_template(template_with_uses("t1", u32::MAX))
            .unwrap();
        let err = e
            .propose(
                50_000,
                "t1",
                Vec::new(),
                vec!["c1".into()],
                Vec::new(),
                "x",
                EvidenceState::Observed,
            )
            .unwrap_err();
        assert!(matches!(err, SecretaryL1Error::OutOfWindow { .. }));
    }

    #[test]
    fn max_uses_reached() {
        let e = SecretaryL1Engine::new();
        e.register_template(template_with_uses("t1", 1)).unwrap();
        e.propose(
            1_500,
            "t1",
            Vec::new(),
            vec!["c1".into()],
            Vec::new(),
            "x",
            EvidenceState::Observed,
        )
        .unwrap();
        let err = e
            .propose(
                1_500,
                "t1",
                Vec::new(),
                vec!["c2".into()],
                Vec::new(),
                "y",
                EvidenceState::Observed,
            )
            .unwrap_err();
        assert!(matches!(err, SecretaryL1Error::MaxUsesReached { .. }));
    }

    #[test]
    fn evidence_state_survives_to_the_issuance() {
        let e = SecretaryL1Engine::new();
        e.register_template(template_with_uses("t1", u32::MAX))
            .unwrap();
        let p = e
            .propose(
                1_500,
                "t1",
                Vec::new(),
                vec!["c1".into()],
                Vec::new(),
                "x",
                EvidenceState::Empty,
            )
            .expect("un estado evidencial es valido por construccion");
        // Este test se llamaba `invalid_confidence_rejected` y afirmaba que
        // `propose(1.5, …)` devolvía `InvalidConfidence`. Con ADR-0156 esa
        // comprobación desaparece CON el número: un enum no tiene rango que
        // incumplir, y el error que existía solo para decidir si un `f64`
        // caía en `[0,1]` ya no es alcanzable desde `propose()`.
        //
        // No se borra y se deja un hueco: se sustituye por la propiedad que
        // sustituye a la anterior. La anterior comprobaba que un número
        // arbitrario fuera rechazado; la nueva comprueba que el estado
        // evidencial que declara el llamador llega intacto a la emisión, que es
        // lo que ahora puede estar mal y lo que un consumidor necesita saber.
        assert_eq!(p.evidence, EvidenceState::Empty);
    }

    #[test]
    fn empty_coverage_rejected() {
        let e = SecretaryL1Engine::new();
        e.register_template(template_with_uses("t1", u32::MAX))
            .unwrap();
        let err = e
            .propose(
                1_500,
                "t1",
                Vec::new(),
                Vec::new(),
                Vec::new(),
                "x",
                EvidenceState::Observed,
            )
            .unwrap_err();
        assert!(matches!(err, SecretaryL1Error::EmptyCoverage { .. }));
    }

    #[test]
    fn kind_binding_preserved() {
        let e = SecretaryL1Engine::new();
        let t = ProposalTemplate::new(
            "t-ack",
            ClosedSetKind::AcknowledgeDecision,
            "ack",
            RiskTier::Trivial,
            BoundedWindow::new(SecretaryId::new("s"), 0, 100, u32::MAX),
        );
        e.register_template(t).unwrap();
        let p = e
            .propose(
                50,
                "t-ack",
                Vec::new(),
                vec!["c1".into()],
                Vec::new(),
                "x",
                EvidenceState::Observed,
            )
            .unwrap();
        assert_eq!(p.kind, ClosedSetKind::AcknowledgeDecision);
    }

    #[test]
    fn unknown_template_rejected() {
        let e = SecretaryL1Engine::new();
        let err = e
            .propose(
                1_500,
                "missing",
                Vec::new(),
                vec!["c1".into()],
                Vec::new(),
                "x",
                EvidenceState::Observed,
            )
            .unwrap_err();
        assert!(matches!(err, SecretaryL1Error::UnknownTemplate { .. }));
    }

    #[test]
    fn template_window_end() {
        let e = SecretaryL1Engine::new();
        e.register_template(template_with_uses("t1", u32::MAX))
            .unwrap();
        let err = e
            .propose(
                1_500,
                "t1",
                Vec::new(),
                vec!["c1".into()],
                Vec::new(),
                "x",
                EvidenceState::Observed,
            )
            .ok();
        assert!(err.is_some());
        // Boundary: exactly valid_until_ms should still succeed.
        let ok = e
            .propose(
                10_000,
                "t1",
                Vec::new(),
                vec!["c1".into()],
                Vec::new(),
                "x",
                EvidenceState::Observed,
            )
            .ok();
        assert!(ok.is_some());
        let err = e
            .propose(
                10_001,
                "t1",
                Vec::new(),
                vec!["c1".into()],
                Vec::new(),
                "x",
                EvidenceState::Observed,
            )
            .unwrap_err();
        assert!(matches!(err, SecretaryL1Error::OutOfWindow { .. }));
    }
}
