// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// revision_substrate.rs — Cross-domain `Revision<T>` envelope + `Ref` CAS
// primitive per ADR-0097 (Common Revision Substrate).
//
// Provides a generic immutable content-addressed revision envelope that
// any domain can adopt as its canonical revision shape:
//
// ```text
// Revision<T> = {
//     oid:          String,         // sha256:<hex> of canonical payload
//     parents:      Vec<String>,    // parent revision oids (empty for root)
//     payload_ref:  CasRef,         // content-addressed handle to T
//     provenance:   ProvenanceV1,   // when/who/why this revision was created
//     metadata:     BTreeMap<String, String>,  // domain-agnostic tags
// }
// ```
//
// `Ref = { namespace, name, expected_old?, new_oid }` is the CAS handle
// that pointers (branches, refs, watch points) carry. Compare-and-swap
// updates use `expected_old?` to fail-closed on stale writes.
//
// The substrate is domain-agnostic. Domain layers add semantic diff/merge
// rules and shape the canonical payload T. ADR-0097 explicitly says:
//
// > Domain layers add semantic diff/merge rules; the substrate remains
// > domain-agnostic.
//
// Existing domain-specific revisions (`PlanRevisionV1`,
// `ExecutionGraphRevision`, `GraphRevision`) remain the primary
// authoritative revision types in their respective domains. The
// `Revision<T>` envelope is the substrate that future cross-domain work
// (cycle 7+) can adopt without forking. Migration is strangler: the
// existing types stay; new code uses `Revision<T>` when a domain needs
// cross-domain reasoning about revisions (e.g. lineage that spans plan +
// execution graph).
//
// Reference: ADR-0097 (Common content-addressed revision substrate).

use crate::canonical_event_log::CasRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

/// Canonical, deterministic identity for an immutable revision.
///
/// Computed as `sha256(canonical_json(payload))`. Two revisions with the
/// same serialised payload produce the same Oid, regardless of how the
/// payload is wrapped at the call site.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Oid(pub String);

impl Oid {
    /// Compute the Oid of a payload by canonical JSON serialisation.
    /// The payload MUST be `Serialize`; `BTreeMap`/`BTreeSet` fields keep
    /// the serialisation deterministic.
    pub fn of<T: Serialize>(payload: &T) -> Self {
        let bytes = serde_json::to_vec(payload).expect("payload is always serializable");
        let digest = Sha256::digest(&bytes);
        let hex = format!("{:064x}", digest);
        Oid(format!("sha256:{hex}"))
    }
}

/// Provenance metadata common to all revisions: when, who, why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceV1 {
    /// RFC 3339 timestamp at which the revision was created.
    pub created_at: String,
    /// Actor identifier (e.g. `system`, `human:<user>`, `agent:<id>`).
    pub created_by: String,
    /// Free-form reason string. Should be a stable tag (e.g. cycle id).
    pub reason: String,
}

/// Generic content-addressed revision envelope (cross-domain).
///
/// Domain layers wrap their canonical payload `T` in this struct when
/// they need a substrate-level identity that participates in the
/// cross-domain ref/lineage machinery. Domains that have existing
/// specialised revision types (`PlanRevisionV1`, `ExecutionGraphRevision`,
/// `GraphRevision`) do not need to migrate — the substrate is additive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision<T> {
    /// Content identity of the payload T.
    pub oid: Oid,
    /// Parent revision Oids. Empty for the root revision.
    pub parents: Vec<Oid>,
    /// Content-addressed handle to the serialised payload. Equals `oid`
    /// for canonical envelopes where the payload IS the Oid content, but
    /// is retained as a separate field so callers can dispatch by CAS
    /// without re-serialising.
    pub payload_ref: CasRef,
    /// Provenance: when, who, why.
    pub provenance: ProvenanceV1,
    /// Domain-agnostic tags (cycle id, scope, etc.). Free-form string map.
    pub metadata: BTreeMap<String, String>,
    /// The domain payload itself. Wrapped, not embedded in Oid computation.
    #[serde(skip)]
    pub payload: std::marker::PhantomData<T>,
}

impl<T: Serialize> Revision<T> {
    /// Construct a root revision (no parents) for a payload.
    pub fn root(payload: &T, provenance: ProvenanceV1) -> Self {
        let oid = Oid::of(payload);
        let payload_ref = CasRef(oid.0.clone());
        Self {
            oid,
            parents: Vec::new(),
            payload_ref,
            provenance,
            metadata: BTreeMap::new(),
            payload: std::marker::PhantomData,
        }
    }

    /// Construct a child revision by appending `payload` to one or more
    /// parents. The Oid is computed over `(parents, payload)` so the
    /// child is unique to its lineage position.
    pub fn child(parents: Vec<Oid>, payload: &T, provenance: ProvenanceV1) -> Self {
        // Compute Oid over a stable structure: parents + payload JSON.
        #[derive(Serialize)]
        struct ChildInput<'a, P: Serialize> {
            parents: &'a [Oid],
            payload: &'a P,
        }
        let input = ChildInput {
            parents: &parents,
            payload,
        };
        let oid = Oid::of(&input);
        let payload_ref = CasRef(oid.0.clone());
        Self {
            oid,
            parents,
            payload_ref,
            provenance,
            metadata: BTreeMap::new(),
            payload: std::marker::PhantomData,
        }
    }

    /// Attach a metadata key/value to this revision. Builder-style.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// CAS handle that points at a revision by namespace + name.
///
/// Compare-and-swap updates: `expected_old = None` allows unconditional
/// set (initial); `expected_old = Some(oid)` requires the current value
/// to match for the swap to succeed. Stale writes return
/// `RefError::Stale` and do not mutate state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ref {
    /// Namespace, e.g. `plan`, `execution_graph`, `decision`.
    pub namespace: String,
    /// Name within the namespace, e.g. `head`, `release-candidate`.
    pub name: String,
    /// The Oid currently stored (None if the ref is unset).
    pub current: Option<Oid>,
}

/// Outcome of a CAS update attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefUpdate {
    /// The swap succeeded; `new_oid` is now the current value.
    Updated { new_oid: Oid },
    /// The swap was rejected because `expected_old` did not match.
    /// `actual` is the current value (caller can decide whether to retry).
    Stale { actual: Option<Oid> },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RefError {
    /// The namespace/name pair is reserved (e.g. starts with `_`).
    #[error("reserved namespace/name: {namespace}/{name} (reserved prefix is _)")]
    Reserved { namespace: String, name: String },
    /// The payload cannot be serialised (should be impossible per ADR-0097
    /// contract).
    #[error("payload serialisation failed: {0}")]
    Serialization(String),
}

/// In-memory `Ref` store with CAS semantics. Thread-safe via Mutex.
///
/// `sddk-storage` may later back this with a persistent CAS store; the
/// in-memory implementation is the cross-domain substrate primitive for
/// v1.168.36 (this cycle).
#[derive(Debug, Default)]
pub struct RefStore {
    inner: std::sync::Mutex<BTreeMap<(String, String), Oid>>,
}

impl RefStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the current Oid for a `Ref`. Returns `None` if unset.
    pub fn get(&self, namespace: &str, name: &str) -> Result<Option<Oid>, RefError> {
        Self::check_reserved(namespace, name)?;
        let state = self.inner.lock().expect("ref-store mutex poisoned");
        Ok(state
            .get(&(namespace.to_string(), name.to_string()))
            .cloned())
    }

    /// Compare-and-swap a ref to `new_oid`.
    ///
    /// If `expected_old` is `None`, sets unconditionally. If `Some(oid)`,
    /// requires the current value to match `oid` exactly; returns
    /// `RefUpdate::Stale` otherwise.
    pub fn cas(
        &self,
        namespace: &str,
        name: &str,
        expected_old: Option<&Oid>,
        new_oid: &Oid,
    ) -> Result<RefUpdate, RefError> {
        Self::check_reserved(namespace, name)?;
        let mut state = self.inner.lock().expect("ref-store mutex poisoned");
        let key = (namespace.to_string(), name.to_string());
        let actual = state.get(&key).cloned();
        if actual != expected_old.cloned() {
            return Ok(RefUpdate::Stale { actual });
        }
        state.insert(key, new_oid.clone());
        Ok(RefUpdate::Updated {
            new_oid: new_oid.clone(),
        })
    }

    fn check_reserved(namespace: &str, name: &str) -> Result<(), RefError> {
        if namespace.starts_with('_') || name.starts_with('_') {
            return Err(RefError::Reserved {
                namespace: namespace.to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oid_is_deterministic_for_same_payload() {
        let payload = serde_json::json!({"key": "value", "n": 42});
        let a = Oid::of(&payload);
        let b = Oid::of(&payload);
        assert_eq!(a, b);
        assert!(a.0.starts_with("sha256:"));
        assert_eq!(a.0.len(), "sha256:".len() + 64);
    }

    #[test]
    fn oid_differs_for_different_payload() {
        let a = Oid::of(&serde_json::json!({"k": "v1"}));
        let b = Oid::of(&serde_json::json!({"k": "v2"}));
        assert_ne!(a, b);
    }

    #[test]
    fn root_revision_has_no_parents_and_empty_metadata() {
        let payload = serde_json::json!({"plan": "x"});
        let prov = ProvenanceV1 {
            created_at: "2026-09-12T00:00:00Z".into(),
            created_by: "test".into(),
            reason: "root".into(),
        };
        let r = Revision::<serde_json::Value>::root(&payload, prov);
        assert!(r.parents.is_empty());
        assert!(r.metadata.is_empty());
        assert_eq!(r.payload_ref.digest(), r.oid.0);
    }

    #[test]
    fn child_revision_oid_depends_on_parents() {
        let payload = serde_json::json!({"plan": "y"});
        let prov = ProvenanceV1 {
            created_at: "2026-09-12T00:00:00Z".into(),
            created_by: "test".into(),
            reason: "child".into(),
        };
        let parent1 = Oid::of(&serde_json::json!({"parent": 1}));
        let parent2 = Oid::of(&serde_json::json!({"parent": 2}));
        let c1 =
            Revision::<serde_json::Value>::child(vec![parent1.clone()], &payload, prov.clone());
        let c2 = Revision::<serde_json::Value>::child(vec![parent2], &payload, prov);
        assert_ne!(c1.oid, c2.oid, "child Oid must depend on parents");
    }

    #[test]
    fn with_metadata_attaches_keys() {
        let payload = serde_json::json!({"k": "v"});
        let prov = ProvenanceV1 {
            created_at: "2026-09-12T00:00:00Z".into(),
            created_by: "test".into(),
            reason: "meta".into(),
        };
        let r = Revision::<serde_json::Value>::root(&payload, prov)
            .with_metadata("cycle", "test-cycle")
            .with_metadata("scope", "uat");
        assert_eq!(
            r.metadata.get("cycle").map(|s| s.as_str()),
            Some("test-cycle")
        );
        assert_eq!(r.metadata.get("scope").map(|s| s.as_str()), Some("uat"));
    }

    #[test]
    fn ref_store_cas_unconditional_set_when_empty() {
        let store = RefStore::new();
        let new = Oid::of(&serde_json::json!({"x": 1}));
        let outcome = store.cas("plan", "head", None, &new).expect("cas ok");
        assert_eq!(
            outcome,
            RefUpdate::Updated {
                new_oid: new.clone()
            }
        );
        let got = store.get("plan", "head").expect("get ok");
        assert_eq!(got, Some(new));
    }

    #[test]
    fn ref_store_cas_succeeds_when_expected_old_matches() {
        let store = RefStore::new();
        let old = Oid::of(&serde_json::json!({"x": 1}));
        let new = Oid::of(&serde_json::json!({"x": 2}));
        store.cas("plan", "head", None, &old).expect("init");
        let outcome = store.cas("plan", "head", Some(&old), &new).expect("cas ok");
        assert_eq!(
            outcome,
            RefUpdate::Updated {
                new_oid: new.clone()
            }
        );
        let got = store.get("plan", "head").expect("get ok");
        assert_eq!(got, Some(new));
    }

    #[test]
    fn ref_store_cas_fails_closed_on_stale_expected() {
        let store = RefStore::new();
        let actual = Oid::of(&serde_json::json!({"x": 1}));
        let stale = Oid::of(&serde_json::json!({"x": 0}));
        let new = Oid::of(&serde_json::json!({"x": 2}));
        store.cas("plan", "head", None, &actual).expect("init");
        let outcome = store
            .cas("plan", "head", Some(&stale), &new)
            .expect("cas ok (returns stale)");
        assert_eq!(
            outcome,
            RefUpdate::Stale {
                actual: Some(actual.clone())
            }
        );
        // Ref must not have been mutated.
        let got = store.get("plan", "head").expect("get ok");
        assert_eq!(got, Some(actual), "stale write must not mutate the ref");
    }

    #[test]
    fn ref_store_rejects_reserved_namespace() {
        let store = RefStore::new();
        let new = Oid::of(&serde_json::json!({"x": 1}));
        let result = store.cas("_reserved", "head", None, &new);
        assert!(matches!(result, Err(RefError::Reserved { .. })));
    }

    #[test]
    fn ref_store_rejects_reserved_name() {
        let store = RefStore::new();
        let new = Oid::of(&serde_json::json!({"x": 1}));
        let result = store.cas("plan", "_internal", None, &new);
        assert!(matches!(result, Err(RefError::Reserved { .. })));
    }

    #[test]
    fn ref_store_multiple_namespaces_isolated() {
        let store = RefStore::new();
        let a = Oid::of(&serde_json::json!({"ns": "plan"}));
        let b = Oid::of(&serde_json::json!({"ns": "graph"}));
        store.cas("plan", "head", None, &a).expect("plan head");
        store.cas("graph", "head", None, &b).expect("graph head");
        assert_eq!(store.get("plan", "head").unwrap(), Some(a));
        assert_eq!(store.get("graph", "head").unwrap(), Some(b));
    }
}
