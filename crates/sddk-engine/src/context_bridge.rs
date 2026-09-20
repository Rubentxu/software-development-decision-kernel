//! Context Delta Delivery / ContextBridge (arch-spec-026, J4).
//!
//! Las sesiones agénticas reciben bootstrap una vez por basis y
//! deltas compactos después (CDD-001/002). El contenido advisory
//! NUNCA es InstructionSource (CDD-004) y los deltas stale se
//! rechazan explícitamente (CDD-005).
//!
//! Upstream: `docs/architecture/specs/arch-spec-026-context-delta-delivery.md`
//! (status: proposed). Segundo slice del track J2→J4.

use crate::agentic_session_binding::{AgenticBinding, BindingTarget};
use serde::{Deserialize, Serialize};

/// Compact bootstrap compiled from the current ContextBasis
/// (CDD-001).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBootstrapContext {
    /// The basis this bootstrap was compiled from.
    pub basis_revision: String,
    /// Advisory-only alignment content (CDD-004: never an
    /// InstructionSource; never mutates EffectiveInstructionSet).
    pub advisory: Vec<String>,
    /// Compact canonical facts (bindings, contracts in force).
    pub facts: Vec<String>,
}

/// One compact, attributable context update (CDD-002).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextDelta {
    /// Basis the delta applies FROM.
    pub from_revision: String,
    /// Basis the delta compiles TO.
    pub to_revision: String,
    /// Why this delta is relevant to the active binding (CDD-002).
    pub relevance_reason: String,
    /// Compact content additions. Deletions are listed separately.
    pub additions: Vec<String>,
    pub deletions: Vec<String>,
    /// Monotonic delta sequence within the binding stream.
    pub seq: u64,
    /// Whether the content is advisory-only (always true for
    /// alignment content — CDD-004).
    pub advisory_only: bool,
}

/// Closed error surface.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ContextDeltaError {
    #[error("delta from_revision {from} does not match current basis {current}")]
    StaleFrom { from: String, current: String },
    #[error("delta sequence must be monotonic: current {current}, requested {requested}")]
    NonMonotonicSeq { current: u64, requested: u64 },
    #[error("advisory content cannot be delivered as instruction authority")]
    AdvisoryIsNotInstruction,
}

/// The ContextBridge: compiles bootstrap and applies deltas for one
/// binding, tracking the delivered stream state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextBridge {
    current_revision: String,
    last_seq: u64,
    delivered: Vec<ContextDelta>,
    advisory: Vec<String>,
    facts: Vec<String>,
}

impl ContextBridge {
    /// Bootstrap once per binding basis (CDD-001). Re-bootstrap is
    /// only valid for a NEW basis — call `bootstrap` on
    /// re-established bindings, not as a resend mechanism.
    #[must_use]
    pub fn bootstrap(
        binding: &AgenticBinding,
        basis_revision: impl Into<String>,
    ) -> (Self, SessionBootstrapContext) {
        let revision = basis_revision.into();
        let advisory: Vec<String> = binding
            .semantic_refs
            .iter()
            .filter(|r| r.starts_with("advisory:"))
            .cloned()
            .collect();
        let facts: Vec<String> = binding
            .semantic_refs
            .iter()
            .filter(|r| !r.starts_with("advisory:"))
            .cloned()
            .collect();
        let b = SessionBootstrapContext {
            basis_revision: revision.clone(),
            advisory: advisory.clone(),
            facts: facts.clone(),
        };
        (
            Self {
                current_revision: revision,
                last_seq: 0,
                delivered: Vec::new(),
                advisory,
                facts,
            },
            b,
        )
    }

    /// Current delivered basis.
    #[must_use]
    pub fn current_revision(&self) -> &str {
        &self.current_revision
    }

    /// Apply a delta (CDD-002/005): the from_revision must match the
    /// current basis and the sequence must be monotonic. Duplicate
    /// or stale deltas are explicitly rejected, never silently
    /// applied.
    pub fn apply(&mut self, delta: ContextDelta) -> Result<(), ContextDeltaError> {
        if delta.from_revision != self.current_revision {
            return Err(ContextDeltaError::StaleFrom {
                from: delta.from_revision,
                current: self.current_revision.clone(),
            });
        }
        if delta.seq <= self.last_seq {
            return Err(ContextDeltaError::NonMonotonicSeq {
                current: self.last_seq,
                requested: delta.seq,
            });
        }
        // Apply content.
        for d in &delta.deletions {
            self.facts.retain(|f| f != d);
            self.advisory.retain(|a| a != d);
        }
        if delta.advisory_only {
            self.advisory.extend(delta.additions.iter().cloned());
        } else {
            self.facts.extend(delta.additions.iter().cloned());
        }
        self.current_revision = delta.to_revision.clone();
        self.last_seq = delta.seq;
        self.delivered.push(delta);
        Ok(())
    }

    /// CDD-003 (relevance filter, caller side): the bridge exposes
    /// whether a change set touches the binding's target namespace.
    /// Changes that do not affect the active binding produce NO
    /// host injection.
    #[must_use]
    pub fn relevant_to(target: &BindingTarget, changed_namespaces: &[&str]) -> bool {
        match target {
            BindingTarget::Project { project_id } => changed_namespaces
                .iter()
                .any(|n| n == project_id || n.starts_with(&format!("{project_id}/"))),
            BindingTarget::WorkItem {
                project_id,
                work_item_id,
            } => changed_namespaces.iter().any(|n| {
                n == &format!("{project_id}/{work_item_id}")
                    || n.starts_with(&format!("{project_id}/{work_item_id}/"))
            }),
            BindingTarget::Task {
                project_id,
                task_id,
            } => changed_namespaces.iter().any(|n| {
                n == &format!("{project_id}/{task_id}")
                    || n.starts_with(&format!("{project_id}/{task_id}/"))
            }),
            BindingTarget::Run { .. } => true,
            BindingTarget::Ephemeral => false,
        }
    }

    /// Advisory content snapshot. Advisory is NEVER instruction
    /// authority (CDD-004): there is no API here that returns an
    /// InstructionSource or mutates an EffectiveInstructionSet.
    #[must_use]
    pub fn advisory(&self) -> &[String] {
        &self.advisory
    }

    /// Deltas delivered so far (provenance trail, CDD-002).
    #[must_use]
    pub fn delivered(&self) -> &[ContextDelta] {
        &self.delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_session_binding::{AgenticSessionRef, BindingTarget};

    fn bound_session() -> AgenticBinding {
        let mut b = AgenticBinding::attach(
            AgenticSessionRef::new("sess-j4"),
            BindingTarget::Task {
                project_id: "p-1".into(),
                task_id: "t-1".into(),
            },
        );
        b.semantic_refs = vec![
            "advisory:alignment-lens-note".into(),
            "fact:contract-c-42".into(),
        ];
        b
    }

    fn delta(from: &str, to: &str, seq: u64, advisory_only: bool) -> ContextDelta {
        ContextDelta {
            from_revision: from.into(),
            to_revision: to.into(),
            relevance_reason: "contract c-42 status changed".into(),
            additions: vec!["fact:contract-c-42-verified".into()],
            deletions: vec![],
            seq,
            advisory_only,
        }
    }

    /// CDD-001: bootstrap compiled once from the current basis,
    /// separating advisory from facts.
    #[test]
    fn cdd001_bootstrap_once_per_basis() {
        let (bridge, boot) = ContextBridge::bootstrap(&bound_session(), "rev-1");
        assert_eq!(boot.basis_revision, "rev-1");
        assert_eq!(
            boot.advisory,
            vec!["advisory:alignment-lens-note".to_string()]
        );
        assert_eq!(boot.facts, vec!["fact:contract-c-42".to_string()]);
        assert_eq!(bridge.current_revision(), "rev-1");
    }

    /// CDD-002: deltas carry from/to basis, provenance (delivered
    /// trail) and relevance reason.
    #[test]
    fn cdd002_delta_carries_provenance() {
        let (mut bridge, _) = ContextBridge::bootstrap(&bound_session(), "rev-1");
        let d = delta("rev-1", "rev-2", 1, false);
        bridge.apply(d.clone()).unwrap();
        assert_eq!(bridge.current_revision(), "rev-2");
        assert_eq!(bridge.delivered(), &[d]);
        assert!(bridge.delivered()[0].relevance_reason.contains("contract"));
    }

    /// CDD-003: changes outside the binding's namespace produce no
    /// injection (relevance filter).
    #[test]
    fn cdd003_irrelevant_changes_no_injection() {
        let target = BindingTarget::Task {
            project_id: "p-1".into(),
            task_id: "t-1".into(),
        };
        assert!(ContextBridge::relevant_to(&target, &["p-1/t-1/src"]));
        assert!(!ContextBridge::relevant_to(&target, &["p-9/other"]));
        assert!(!ContextBridge::relevant_to(
            &BindingTarget::Ephemeral,
            &["anything"]
        ));
    }

    /// CDD-004: advisory content stays advisory — no API exposes it
    /// as instruction authority.
    #[test]
    fn cdd004_advisory_never_instruction() {
        let (mut bridge, _) = ContextBridge::bootstrap(&bound_session(), "rev-1");
        let adv = delta("rev-1", "rev-2", 1, true);
        bridge.apply(adv).unwrap();
        assert!(
            bridge
                .advisory()
                .contains(&"fact:contract-c-42-verified".to_string())
        );
        // Structural pin: ContextBridge has no instruction/set_instruction
        // API. The advisory view is read-only (&[String]).
        let _: &[String] = bridge.advisory();
    }

    /// CDD-005: duplicate (same seq) and stale (old from) deltas are
    /// explicitly rejected; no silent out-of-order mutation.
    #[test]
    fn cdd005_duplicate_and_stale_rejected() {
        let (mut bridge, _) = ContextBridge::bootstrap(&bound_session(), "rev-1");
        bridge.apply(delta("rev-1", "rev-2", 1, false)).unwrap();
        // Duplicate seq.
        assert_eq!(
            bridge.apply(delta("rev-2", "rev-3", 1, false)),
            Err(ContextDeltaError::NonMonotonicSeq {
                current: 1,
                requested: 1
            })
        );
        // Stale from_revision.
        assert_eq!(
            bridge.apply(delta("rev-1", "rev-3", 2, false)),
            Err(ContextDeltaError::StaleFrom {
                from: "rev-1".into(),
                current: "rev-2".into()
            })
        );
        // State untouched by the rejected deltas.
        assert_eq!(bridge.current_revision(), "rev-2");
        assert_eq!(bridge.delivered().len(), 1);
    }

    /// CDD-006: no full resend — the stream only grows with compact
    /// deltas; bootstrap is not re-emitted by apply.
    #[test]
    fn cdd006_no_full_context_resend() {
        let (mut bridge, boot) = ContextBridge::bootstrap(&bound_session(), "rev-1");
        let facts_before = boot.facts.len();
        bridge.apply(delta("rev-1", "rev-2", 1, false)).unwrap();
        bridge.apply(delta("rev-2", "rev-3", 2, false)).unwrap();
        // Only deltas are appended; the original bootstrap capsule is
        // never re-emitted (no capsule in the delivered stream).
        assert_eq!(bridge.delivered().len(), 2);
        assert!(bridge.delivered().iter().all(|d| d.seq > 0));
        // Bootstrap facts remain compact (CDD-006: no churn).
        assert_eq!(facts_before, 1);
    }
}
