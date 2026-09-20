//! Agentic Session Binding (arch-spec-024).
//!
//! Host sessions relate to SDDK work through explicit semantic
//! bindings. Session is NOT Run (ASB-001); transcript ownership
//! stays in the host (ASB-004).
//!
//! Upstream: `docs/architecture/specs/arch-spec-024-agentic-session-binding.md`
//! (status: proposed). First implementation slice: J2/J3 of
//! `02-MINI-ROADMAP.md`, covering AW-UAT-020..024 (except the
//! remote-locality part of 023, which is J8).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identity of a host (JCode) session. Distinct from `RunRef`
/// (ASB-001: session is not Run).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AgenticSessionRef(pub String);

impl AgenticSessionRef {
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }
}

/// Identity of an SDDK Run. Never synthesized by session attach
/// (ASB-001).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunRef(pub String);

impl RunRef {
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }
}

/// What a session is bound to (ASB-002). Closed set.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BindingTarget {
    Project {
        project_id: String,
    },
    WorkItem {
        project_id: String,
        work_item_id: String,
    },
    Run {
        run_ref: RunRef,
    },
    Task {
        project_id: String,
        task_id: String,
    },
    Ephemeral,
}

/// The context basis a binding was compiled against (ASB-003), so
/// stale/superseded bases are observable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBasis {
    /// Content-addressed revision the context was compiled from.
    pub revision: String,
    /// Monotonic basis sequence within the binding.
    pub seq: u64,
}

/// One semantic binding: session → target, with its context basis
/// and semantic references (NOT the transcript — ASB-004).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgenticBinding {
    pub session: AgenticSessionRef,
    pub target: BindingTarget,
    /// Semantic references persisted by SDDK (receipts, evidence,
    /// contribution ids). Transcript stays host-owned.
    pub semantic_refs: Vec<String>,
    /// Current context basis (ASB-003).
    pub context_basis: Option<ContextBasis>,
    /// Receipt ids produced by rebinding events (ASB-005 evidence).
    pub receipts: Vec<String>,
}

impl AgenticBinding {
    #[must_use]
    pub fn attach(session: AgenticSessionRef, target: BindingTarget) -> Self {
        Self {
            session,
            target,
            semantic_refs: Vec::new(),
            context_basis: None,
            receipts: Vec::new(),
        }
    }

    /// Rebind to a new target, recording a receipt id. The old
    /// binding state is not silently overwritten in the store —
    /// the receipt makes the change observable (AW-UAT-021).
    #[must_use]
    pub fn rebind(mut self, new_target: BindingTarget, receipt_id: impl Into<String>) -> Self {
        self.receipts.push(receipt_id.into());
        self.target = new_target;
        // Context basis is invalidated by rebinding: the new target
        // may need a different basis.
        self.context_basis = None;
        self
    }

    /// Set/advance the context basis (ASB-003). Sequence must be
    /// monotonic.
    pub fn set_context_basis(
        &mut self,
        revision: impl Into<String>,
        seq: u64,
    ) -> Result<(), AgenticBindingError> {
        if let Some(b) = &self.context_basis
            && seq <= b.seq
        {
            return Err(AgenticBindingError::NonMonotonicBasis {
                current: b.seq,
                requested: seq,
            });
        }
        self.context_basis = Some(ContextBasis {
            revision: revision.into(),
            seq,
        });
        Ok(())
    }

    /// Re-establish a binding from persisted semantic state after a
    /// restart (ASB-005/AW-UAT-022). Transcript is NOT imported —
    /// only semantic refs.
    #[must_use]
    pub fn reattach(persisted: &AgenticBinding) -> Self {
        persisted.clone()
    }
}

/// Closed error surface.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AgenticBindingError {
    #[error("context basis sequence must be monotonic: current {current}, requested {requested}")]
    NonMonotonicBasis { current: u64, requested: u64 },
    #[error("session {session} already bound")]
    DuplicateSession { session: String },
    #[error("session {session} not bound")]
    UnknownSession { session: String },
}

/// Store of bindings, keyed by session identity. Multiple sessions
/// over one project keep independent bindings (ASB-006).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BindingStore {
    bindings: BTreeMap<AgenticSessionRef, AgenticBinding>,
}

impl BindingStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a session to a target. NEVER creates a Run (ASB-001):
    /// runs only appear when the target explicitly is
    /// `BindingTarget::Run` chosen by a workflow operation.
    pub fn attach(&mut self, binding: AgenticBinding) -> Result<(), AgenticBindingError> {
        if self.bindings.contains_key(&binding.session) {
            return Err(AgenticBindingError::DuplicateSession {
                session: binding.session.0.clone(),
            });
        }
        self.bindings.insert(binding.session.clone(), binding);
        Ok(())
    }

    pub fn rebind(
        &mut self,
        session: &AgenticSessionRef,
        new_target: BindingTarget,
        receipt_id: impl Into<String>,
    ) -> Result<AgenticBinding, AgenticBindingError> {
        let current = self.bindings.get(session).cloned().ok_or_else(|| {
            AgenticBindingError::UnknownSession {
                session: session.0.clone(),
            }
        })?;
        let next = current.rebind(new_target, receipt_id);
        self.bindings.insert(session.clone(), next.clone());
        Ok(next)
    }

    #[must_use]
    pub fn get(&self, session: &AgenticSessionRef) -> Option<&AgenticBinding> {
        self.bindings.get(session)
    }

    /// Reattach after restart from a persisted snapshot (ASB-005).
    pub fn reattach(&mut self, persisted: AgenticBinding) -> Result<(), AgenticBindingError> {
        self.attach(persisted)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_and_run_are_distinct_identities() {
        let s = AgenticSessionRef::new("sess-1");
        let r = RunRef::new("sess-1");
        // Different types by construction; assert distinct identity
        // domains via serialization tags.
        assert_eq!(serde_json::to_string(&s).unwrap(), "\"sess-1\"");
        assert_eq!(serde_json::to_string(&r).unwrap(), "\"sess-1\"");
        // The distinction is enforced at the type level (ASB-001):
        // no conversion function exists between them.
    }

    #[test]
    fn attach_project_creates_no_run() {
        let mut store = BindingStore::new();
        let b = AgenticBinding::attach(
            AgenticSessionRef::new("sess-1"),
            BindingTarget::Project {
                project_id: "p-1".into(),
            },
        );
        store.attach(b).unwrap();
        let stored = store.get(&AgenticSessionRef::new("sess-1")).unwrap();
        // No RunRef anywhere in the binding (ASB-001).
        assert!(!matches!(stored.target, BindingTarget::Run { .. }));
        assert!(stored.semantic_refs.is_empty(), "no transcript imported");
    }

    #[test]
    fn rebind_records_receipt_and_invalidates_basis() {
        let mut store = BindingStore::new();
        let sess = AgenticSessionRef::new("sess-1");
        store
            .attach(AgenticBinding::attach(
                sess.clone(),
                BindingTarget::WorkItem {
                    project_id: "p-1".into(),
                    work_item_id: "wi-1".into(),
                },
            ))
            .unwrap();
        let mut b = store.get(&sess).unwrap().clone();
        b.set_context_basis("rev-a", 1).unwrap();
        store.bindings.insert(sess.clone(), b);

        let next = store
            .rebind(
                &sess,
                BindingTarget::Task {
                    project_id: "p-1".into(),
                    task_id: "t-9".into(),
                },
                "receipt-rebind-1",
            )
            .unwrap();
        assert_eq!(next.receipts, vec!["receipt-rebind-1".to_string()]);
        assert!(next.context_basis.is_none(), "rebinding invalidates basis");
        assert!(matches!(next.target, BindingTarget::Task { .. }));
    }

    #[test]
    fn basis_sequence_is_monotonic() {
        let mut b = AgenticBinding::attach(AgenticSessionRef::new("s"), BindingTarget::Ephemeral);
        b.set_context_basis("rev-1", 1).unwrap();
        b.set_context_basis("rev-2", 2).unwrap();
        assert_eq!(
            b.set_context_basis("rev-3", 1),
            Err(AgenticBindingError::NonMonotonicBasis {
                current: 2,
                requested: 1
            })
        );
    }

    #[test]
    fn reattach_from_persisted_semantic_state() {
        let mut original = AgenticBinding::attach(
            AgenticSessionRef::new("sess-1"),
            BindingTarget::Project {
                project_id: "p-1".into(),
            },
        );
        original.semantic_refs = vec!["receipt:abc".into(), "contribution:42".into()];
        original.set_context_basis("rev-9", 3).unwrap();

        // Restart: reconstruct from persisted snapshot only.
        let mut store = BindingStore::new();
        store.reattach(AgenticBinding::reattach(&original)).unwrap();
        let restored = store.get(&AgenticSessionRef::new("sess-1")).unwrap();
        assert_eq!(restored, &original, "semantic state intact");
        assert_eq!(restored.semantic_refs.len(), 2, "no transcript duplication");
    }

    #[test]
    fn multi_session_isolation_same_project() {
        let mut store = BindingStore::new();
        let s1 = AgenticSessionRef::new("sess-A");
        let s2 = AgenticSessionRef::new("sess-B");
        store
            .attach(AgenticBinding::attach(
                s1.clone(),
                BindingTarget::Task {
                    project_id: "p-1".into(),
                    task_id: "t-1".into(),
                },
            ))
            .unwrap();
        store
            .attach(AgenticBinding::attach(
                s2.clone(),
                BindingTarget::Task {
                    project_id: "p-1".into(),
                    task_id: "t-2".into(),
                },
            ))
            .unwrap();
        let mut a = store.get(&s1).unwrap().clone();
        a.set_context_basis("rev-a", 1).unwrap();
        store.bindings.insert(s1.clone(), a);
        // Session B's basis is untouched by session A's advance.
        assert!(store.get(&s2).unwrap().context_basis.is_none());
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn duplicate_attach_rejected() {
        let mut store = BindingStore::new();
        let s = AgenticSessionRef::new("sess-1");
        store
            .attach(AgenticBinding::attach(s.clone(), BindingTarget::Ephemeral))
            .unwrap();
        assert_eq!(
            store.attach(AgenticBinding::attach(s.clone(), BindingTarget::Ephemeral)),
            Err(AgenticBindingError::DuplicateSession {
                session: "sess-1".into()
            })
        );
    }
}
