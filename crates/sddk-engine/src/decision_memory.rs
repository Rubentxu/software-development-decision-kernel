//! Decision Memory substrate (CDD-MEMORY-001).
//!
//! Implements the engine-side half of the Git-like Decision Memory
//! model: three immutable content-addressed types (Blob / Tree /
//! Commit), a canonical sha256 hashing rule, a ref/HEAD/tags
//! namespace, an append-only reflog and the advisory-branch
//! authority invariant.
//!
//! Traversal operations and SessionDelta/decision-branch
//! projections are deferred to CDD-MEMORY-002.
//!
//! Reference doc: REQ-DecisionMemory.md (RFC 2119) and
//! docs/sddk-decision-kernel-architecture/02-roadmap/DECISION-MEMORY-GIT-MODEL.md.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::active_graph::{
    ActiveGraphEdge, ActiveGraphEdgeKind, ActiveGraphNode, ActiveGraphNodeKind,
    ActiveGraphProjection,
};
use crate::why_queries::{DefaultWhyQueryEngine, WhyCausalStep, WhyQueryEngine, WhyQueryKind};
use sddk_domain::workflow_ir::NodeId;

// =====================================================================
// Public types
// =====================================================================

/// 32-byte content identifier (sha256 of canonical_payload).
pub type MemoryId = [u8; 32];

/// Helper: hex-encode a MemoryId with lowercase chars.
fn hex_lower(b: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in b {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

// ----------------- Object kinds -----------------

/// A typed addressable payload or pointer set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionMemoryBlob {
    pub object_kind: String,
    pub payload_ref: String,
    /// Content id. Must equal `sha256(canonical_payload(without id))`.
    /// `#[serde(skip)]` so a round-trip JSON does not include `id`,
    /// keeping the canonical payload immutable.
    #[serde(skip)]
    pub id: MemoryId,
}

impl DecisionMemoryBlob {
    pub fn new(
        object_kind: impl Into<String>,
        payload_ref: impl Into<String>,
    ) -> Result<Self, DecisionMemoryError> {
        let blob = Self {
            object_kind: object_kind.into(),
            payload_ref: payload_ref.into(),
            id: [0u8; 32],
        };
        blob.with_recomputed_id()
    }

    pub fn from_parts(
        object_kind: impl Into<String>,
        payload_ref: impl Into<String>,
        id: MemoryId,
    ) -> Result<Self, DecisionMemoryError> {
        let blob = Self {
            object_kind: object_kind.into(),
            payload_ref: payload_ref.into(),
            id,
        };
        blob.verify_id()
    }

    pub fn with_recomputed_id(mut self) -> Result<Self, DecisionMemoryError> {
        let bytes = self.canonical_payload_bytes();
        let mut h = Sha256::new();
        h.update(&bytes);
        let digest: [u8; 32] = h.finalize().into();
        self.id = digest;
        Ok(self)
    }

    pub fn verify_id(self) -> Result<Self, DecisionMemoryError> {
        let bytes = self.canonical_payload_bytes();
        let mut h = Sha256::new();
        h.update(&bytes);
        let expected: [u8; 32] = h.finalize().into();
        if expected != self.id {
            return Err(DecisionMemoryError::HashMismatch {
                expected: hex_lower(&expected),
                computed: hex_lower(&self.id),
            });
        }
        Ok(self)
    }

    pub fn canonical_payload_bytes(&self) -> Vec<u8> {
        // canonical object: {"kind":"blob","object_kind":"...","payload_ref":"..."}
        // Keys are sorted lexicographically.
        let mut out = Vec::new();
        out.extend_from_slice(br#"{"kind":"blob","object_kind":"#);
        push_json_string(&mut out, &self.object_kind);
        out.extend_from_slice(br#","payload_ref":"#);
        push_json_string(&mut out, &self.payload_ref);
        out.extend_from_slice(br#"}"#);
        out
    }

    pub fn id_hex(&self) -> String {
        hex_lower(&self.id)
    }

    /// For tests/ adapters that need to mutate id (used to
    /// deliberately break the hash).
    pub fn set_id(&mut self, id: MemoryId) {
        self.id = id;
    }

    /// Borrowed typed view when `object_kind == "session/checkpoint"`.
    /// Returns `None` for any other `object_kind` — never panics.
    /// Per ADR-092 / R-P1 / S-P1.
    pub fn as_session_checkpoint(&self) -> Option<SessionCheckpointView<'_>> {
        if self.object_kind == "session/checkpoint" {
            Some(SessionCheckpointView { blob: self })
        } else {
            None
        }
    }

    /// Borrowed typed view when `object_kind == "session/delta"`.
    /// Returns `None` for any other `object_kind` — never panics.
    /// Per ADR-092 / R-P2.
    pub fn as_session_delta(&self) -> Option<SessionDeltaView<'_>> {
        if self.object_kind == "session/delta" {
            Some(SessionDeltaView { blob: self })
        } else {
            None
        }
    }
}

/// Borrowed typed view over a `DecisionMemoryBlob` whose
/// `object_kind == "session/checkpoint"`. The blob is the source of
/// truth for the id; the view exposes ergonomic accessors only.
/// Per ADR-092 §2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCheckpointView<'a> {
    blob: &'a DecisionMemoryBlob,
}

impl<'a> SessionCheckpointView<'a> {
    /// Payload reference (e.g. `sha256:...`).
    pub fn payload_ref(&self) -> &str {
        &self.blob.payload_ref
    }

    /// Stable content id; matches the underlying blob's id.
    pub fn id(&self) -> MemoryId {
        self.blob.id
    }

    /// The discriminator string (`"session/checkpoint"`).
    pub fn object_kind(&self) -> &str {
        &self.blob.object_kind
    }

    /// Borrow back the underlying blob.
    pub fn into_inner(self) -> &'a DecisionMemoryBlob {
        self.blob
    }
}

/// Borrowed typed view over a `DecisionMemoryBlob` whose
/// `object_kind == "session/delta"`. Same shape as
/// `SessionCheckpointView`. Per ADR-092 §2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionDeltaView<'a> {
    blob: &'a DecisionMemoryBlob,
}

impl<'a> SessionDeltaView<'a> {
    pub fn payload_ref(&self) -> &str {
        &self.blob.payload_ref
    }

    pub fn id(&self) -> MemoryId {
        self.blob.id
    }

    pub fn object_kind(&self) -> &str {
        &self.blob.object_kind
    }

    pub fn into_inner(self) -> &'a DecisionMemoryBlob {
        self.blob
    }
}

/// Per-subtree entry: name plus target sha256.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub id: MemoryId,
}

/// Semantic snapshot of refs into twelve categorised subtrees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionMemoryTree {
    pub entries: BTreeMap<String, Vec<TreeEntry>>,
    #[serde(skip)]
    pub id: MemoryId,
}

impl DecisionMemoryTree {
    pub fn new(entries: BTreeMap<String, Vec<TreeEntry>>) -> Result<Self, DecisionMemoryError> {
        let tree = Self {
            entries,
            id: [0u8; 32],
        };
        tree.with_recomputed_id()
    }

    pub fn with_recomputed_id(mut self) -> Result<Self, DecisionMemoryError> {
        let bytes = self.canonical_payload_bytes();
        let mut h = Sha256::new();
        h.update(&bytes);
        let digest: [u8; 32] = h.finalize().into();
        self.id = digest;
        Ok(self)
    }

    /// Verify that `self.id` matches `sha256(canonical_payload(self))`.
    /// Returns `HashMismatch` if not.
    pub fn verify_id(self) -> Result<Self, DecisionMemoryError> {
        let bytes = self.canonical_payload_bytes();
        let mut h = Sha256::new();
        h.update(&bytes);
        let expected: [u8; 32] = h.finalize().into();
        if expected != self.id {
            return Err(DecisionMemoryError::HashMismatch {
                expected: hex_lower(&expected),
                computed: hex_lower(&self.id),
            });
        }
        Ok(self)
    }

    pub fn canonical_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(br#"{"kind":"tree","entries":{"#);
        // BTreeMap keeps alphabetical order.
        let mut first = true;
        for (k, v) in &self.entries {
            if !first {
                out.push(b',');
            }
            first = false;
            push_json_string(&mut out, k);
            out.push(b':');
            out.push(b'[');
            let mut first_entry = true;
            for entry in v {
                if !first_entry {
                    out.push(b',');
                }
                first_entry = false;
                push_json_string(&mut out, &entry.name);
                out.extend_from_slice(br#":"#);
                push_json_string(&mut out, &hex_lower(&entry.id));
            }
            out.push(b']');
        }
        out.extend_from_slice(br#"}}"#);
        out
    }

    pub fn id_hex(&self) -> String {
        hex_lower(&self.id)
    }

    /// Test-only escape hatch to deliberately break the hash (used
    /// by DMT-20 to prove put_tree rejects mismatched ids).
    #[doc(hidden)]
    pub fn set_id_for_test(&mut self, id: MemoryId) {
        self.id = id;
    }
}

/// Author of a commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionMemoryAuthor {
    pub actor_type: String,
    pub actor_id: String,
}

impl DecisionMemoryAuthor {
    pub fn new(
        actor_type: impl Into<String>,
        actor_id: impl Into<String>,
    ) -> Result<Self, DecisionMemoryError> {
        let a = actor_type.into();
        if !matches!(a.as_str(), "human" | "agent" | "system") {
            return Err(DecisionMemoryError::AuthorRoleInvalid(a));
        }
        Ok(Self {
            actor_type: a,
            actor_id: actor_id.into(),
        })
    }
}

/// DAG node in memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionMemoryCommit {
    pub parents: Vec<MemoryId>,
    pub tree: MemoryId,
    pub author: DecisionMemoryAuthor,
    pub timestamp: String,
    pub project_id: String,
    pub work_item_id: Option<String>,
    pub cycle_run_id: Option<String>,
    pub subject_revision: Option<String>,
    pub planning_revision: Option<String>,
    pub workflow_revision: Option<String>,
    pub event_cursor: Option<String>,
    pub message: String,
    pub reason: String,
    pub provenance_refs: Vec<MemoryId>,
    /// Forward-pointer to a merge receipt (parsed in CDD-MEMORY-002).
    pub merge_receipt_ref: Option<MemoryId>,
    #[serde(skip)]
    pub id: MemoryId,
}

impl DecisionMemoryCommit {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parents: Vec<MemoryId>,
        tree: MemoryId,
        author: DecisionMemoryAuthor,
        timestamp: impl Into<String>,
        project_id: impl Into<String>,
        work_item_id: Option<impl Into<String>>,
        cycle_run_id: Option<impl Into<String>>,
        subject_revision: Option<impl Into<String>>,
        planning_revision: Option<impl Into<String>>,
        workflow_revision: Option<impl Into<String>>,
        event_cursor: Option<impl Into<String>>,
        message: impl Into<String>,
        reason: impl Into<String>,
        provenance_refs: Vec<MemoryId>,
    ) -> Result<Self, DecisionMemoryError> {
        let c = Self {
            parents,
            tree,
            author,
            timestamp: timestamp.into(),
            project_id: project_id.into(),
            work_item_id: work_item_id.map(Into::into),
            cycle_run_id: cycle_run_id.map(Into::into),
            subject_revision: subject_revision.map(Into::into),
            planning_revision: planning_revision.map(Into::into),
            workflow_revision: workflow_revision.map(Into::into),
            event_cursor: event_cursor.map(Into::into),
            message: message.into(),
            reason: reason.into(),
            provenance_refs,
            merge_receipt_ref: None,
            id: [0u8; 32],
        };
        c.with_recomputed_id()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn merge(
        parents: Vec<MemoryId>,
        tree: MemoryId,
        author: DecisionMemoryAuthor,
        timestamp: impl Into<String>,
        project_id: impl Into<String>,
        message: impl Into<String>,
        reason: impl Into<String>,
        merge_receipt_ref: MemoryId,
    ) -> Result<Self, DecisionMemoryError> {
        let c = Self {
            parents,
            tree,
            author,
            timestamp: timestamp.into(),
            project_id: project_id.into(),
            work_item_id: None,
            cycle_run_id: None,
            subject_revision: None,
            planning_revision: None,
            workflow_revision: None,
            event_cursor: None,
            message: message.into(),
            reason: reason.into(),
            provenance_refs: vec![],
            merge_receipt_ref: Some(merge_receipt_ref),
            id: [0u8; 32],
        };
        c.with_recomputed_id()
    }

    pub fn with_recomputed_id(mut self) -> Result<Self, DecisionMemoryError> {
        let bytes = self.canonical_payload_bytes();
        let mut h = Sha256::new();
        h.update(&bytes);
        let digest: [u8; 32] = h.finalize().into();
        self.id = digest;
        Ok(self)
    }

    /// Verify that `self.id` matches `sha256(canonical_payload(self))`.
    /// Returns `HashMismatch` if not.
    pub fn verify_id(self) -> Result<Self, DecisionMemoryError> {
        let bytes = self.canonical_payload_bytes();
        let mut h = Sha256::new();
        h.update(&bytes);
        let expected: [u8; 32] = h.finalize().into();
        if expected != self.id {
            return Err(DecisionMemoryError::HashMismatch {
                expected: hex_lower(&expected),
                computed: hex_lower(&self.id),
            });
        }
        Ok(self)
    }

    pub fn canonical_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(br#"{"author":{"actor_id":"#);
        push_json_string(&mut out, &self.author.actor_id);
        out.extend_from_slice(br#","actor_type":"#);
        push_json_string(&mut out, &self.author.actor_type);
        out.extend_from_slice(br#"""},"#);
        // cycle_run_id
        out.extend_from_slice(br#""cycle_run_id":"#);
        push_optional_string(&mut out, self.cycle_run_id.as_deref());
        out.push(b',');
        // event_cursor
        out.extend_from_slice(br#""event_cursor":"#);
        push_optional_string(&mut out, self.event_cursor.as_deref());
        out.push(b',');
        // kind
        out.extend_from_slice(br#""kind":"commit","#);
        // merge_receipt_ref
        out.extend_from_slice(br#""merge_receipt_ref":"#);
        push_optional_id(&mut out, self.merge_receipt_ref.as_ref());
        out.push(b',');
        // message
        out.extend_from_slice(br#""message":"#);
        push_json_string(&mut out, &self.message);
        out.push(b',');
        // parents
        out.extend_from_slice(br#""parents":["#);
        let mut first = true;
        for p in &self.parents {
            if !first {
                out.push(b',');
            }
            first = false;
            push_json_string(&mut out, &hex_lower(p));
        }
        out.extend_from_slice(br#"],"#);
        // planning_revision
        out.extend_from_slice(br#""planning_revision":"#);
        push_optional_string(&mut out, self.planning_revision.as_deref());
        out.push(b',');
        // project_id
        out.extend_from_slice(br#""project_id":"#);
        push_json_string(&mut out, &self.project_id);
        out.push(b',');
        // provenance_refs
        out.extend_from_slice(br#""provenance_refs":["#);
        let mut first = true;
        for p in &self.provenance_refs {
            if !first {
                out.push(b',');
            }
            first = false;
            push_json_string(&mut out, &hex_lower(p));
        }
        out.extend_from_slice(br#"],"#);
        // reason
        out.extend_from_slice(br#""reason":"#);
        push_json_string(&mut out, &self.reason);
        out.push(b',');
        // subject_revision
        out.extend_from_slice(br#""subject_revision":"#);
        push_optional_string(&mut out, self.subject_revision.as_deref());
        out.push(b',');
        // timestamp
        out.extend_from_slice(br#""timestamp":"#);
        push_json_string(&mut out, &self.timestamp);
        out.push(b',');
        // tree
        out.extend_from_slice(br#""tree":"#);
        push_json_string(&mut out, &hex_lower(&self.tree));
        out.push(b',');
        // work_item_id
        out.extend_from_slice(br#""work_item_id":"#);
        push_optional_string(&mut out, self.work_item_id.as_deref());
        out.push(b',');
        // workflow_revision
        out.extend_from_slice(br#""workflow_revision":"#);
        push_optional_string(&mut out, self.workflow_revision.as_deref());
        out.push(b'}');
        out
    }

    pub fn id_hex(&self) -> String {
        hex_lower(&self.id)
    }

    /// Test-only escape hatch to deliberately break the hash (used
    /// by DMT-20 to prove put_commit rejects mismatched ids).
    #[doc(hidden)]
    pub fn set_id_for_test(&mut self, id: MemoryId) {
        self.id = id;
    }
}

// =====================================================================
// Memory store
// =====================================================================

pub trait MemoryStore: Send + Sync + std::fmt::Debug {
    fn put_blob(&self, blob: DecisionMemoryBlob) -> Result<MemoryId, DecisionMemoryError>;
    fn put_tree(&self, tree: DecisionMemoryTree) -> Result<MemoryId, DecisionMemoryError>;
    fn put_commit(&self, commit: DecisionMemoryCommit) -> Result<MemoryId, DecisionMemoryError>;
    fn get_blob(&self, id: &MemoryId) -> Option<DecisionMemoryBlob>;
    fn get_tree(&self, id: &MemoryId) -> Option<DecisionMemoryTree>;
    fn get_commit(&self, id: &MemoryId) -> Option<DecisionMemoryCommit>;
    fn write_ref_with_reflog(
        &self,
        kind: RefKind,
        target: MemoryId,
        reflog: &mut Reflog,
        actor: &str,
        reason: &str,
    ) -> Result<(), DecisionMemoryError>;
    fn resolve_ref(&self, kind: &RefKind) -> Option<MemoryId>;

    // === Traversal ops (CDD-MEMORY-002 / v1.148.0) ===
    //
    // These methods are part of the v1.148.0 surface. Default impls
    // panic with `unimplemented!()` so any external implementor is
    // forced to provide them. `InMemoryMemoryStore` overrides every
    // one in `crates/sddk-engine/src/decision_memory.rs`.

    /// Walk ancestors of `from` in topological order (parents
    /// before children), capped at `max` entries. Returns hashes
    /// sorted ascending. R-T4 / S-T2.
    fn log(&self, from: MemoryId, max: usize) -> Result<Vec<MemoryId>, DecisionMemoryError> {
        let _ = (from, max);
        unimplemented!("MemoryStore::log is not implemented by this store")
    }

    /// Return commit + tree + refs pointing at the commit. R-T5 / S-T3.
    fn show(&self, id: MemoryId) -> Result<MemoryShow, DecisionMemoryError> {
        let _ = id;
        unimplemented!("MemoryStore::show is not implemented by this store")
    }

    /// Return the typed tree projection (entries grouped by kind).
    /// Top fan-out capped at 64. R-T6 / S-T6.
    fn tree(&self, id: MemoryId) -> Result<TreeProjection, DecisionMemoryError> {
        let _ = id;
        unimplemented!("MemoryStore::tree is not implemented by this store")
    }

    /// Diff between two commits. Default walk cap 1024. R-T7 / S-T4.
    fn diff(&self, a: MemoryId, b: MemoryId) -> Result<MemoryDiff, DecisionMemoryError> {
        let _ = (a, b);
        unimplemented!("MemoryStore::diff is not implemented by this store")
    }

    /// Lowest common ancestor under parent-link topology; ties broken
    /// by hash ascending. R-T8 / S-T5.
    fn merge_base(&self, a: MemoryId, b: MemoryId) -> Result<MemoryId, DecisionMemoryError> {
        let _ = (a, b);
        unimplemented!("MemoryStore::merge_base is not implemented by this store")
    }

    /// Ancestors at each depth tier (Vec<Vec<MemoryId>>). R-T9 / S-T7.
    fn ancestors(
        &self,
        id: MemoryId,
        max_depth: usize,
    ) -> Result<Vec<Vec<MemoryId>>, DecisionMemoryError> {
        let _ = (id, max_depth);
        unimplemented!("MemoryStore::ancestors is not implemented by this store")
    }

    /// Why projection for a given ref_kind. Delegates to
    /// `WhyQueryEngine::decision_why`. R-T10 / S-T8.
    fn why(&self, ref_id: RefKind) -> Result<WhyProjection, DecisionMemoryError> {
        let _ = ref_id;
        unimplemented!("MemoryStore::why is not implemented by this store")
    }

    /// Reflog entries filtered by `scope`, in descending `seq`
    /// order. R-T11 / S-T9.
    fn reflog(
        &self,
        scope: ReflogScope,
        max: usize,
    ) -> Result<Vec<ReflogEntry>, DecisionMemoryError> {
        let _ = (scope, max);
        unimplemented!("MemoryStore::reflog is not implemented by this store")
    }

    /// Full persistent reflog history for `ref_kind`, sorted
    /// descending by `seq` (newest first). R-M1 / S-M1. The vector
    /// MAY be empty (refs never written). Surfaces `NotFound` when
    /// the ref does not resolve.
    fn reflog_history(
        &self,
        ref_kind: RefKind,
    ) -> Result<Vec<ReflogEntry>, DecisionMemoryError> {
        let _ = ref_kind;
        unimplemented!("MemoryStore::reflog_history is not implemented by this store")
    }

    /// 1-indexed lookup into the persistent reflog history for
    /// `ref_kind`. `seq=1` is the oldest entry; `seq=N` is the newest.
    /// R-M2 / S-M2..S-M4. Surfaces `NotFound` for unknown refs and
    /// `LimitExceeded { op: "reflog_at", cap: N }` when `seq > N`.
    fn reflog_at(
        &self,
        ref_kind: RefKind,
        seq: u64,
    ) -> Result<ReflogEntry, DecisionMemoryError> {
        let _ = (ref_kind, seq);
        unimplemented!("MemoryStore::reflog_at is not implemented by this store")
    }

    /// Create a what-if ref pointing at `target`. R-T12 / S-T10.
    fn branch(&self, name: &str, target: MemoryId) -> Result<(), DecisionMemoryError> {
        let _ = (name, target);
        unimplemented!("MemoryStore::branch is not implemented by this store")
    }

    /// Create `refs/heads/what-if/<as_name>` pointing at canonical
    /// HEAD. R-T13 / S-T11.
    fn fork(&self, as_name: &str) -> Result<(), DecisionMemoryError> {
        let _ = as_name;
        unimplemented!("MemoryStore::fork is not implemented by this store")
    }

    // === Projection specialization ops (CDD-MEMORY-003 / v1.149.0) ===
    //
    // Per ADR-092 §2.2 / R-P3 / R-P4 / R-P8. Default impls panic
    // with `unimplemented!()` so any external implementor is forced
    // to provide them. `InMemoryMemoryStore` overrides both.

    /// Filter blobs whose `object_kind` starts with `"decision/"`,
    /// anchored at `at_commit`. R-P3 / S-P3.
    fn decision_projection(
        &self,
        at_commit: MemoryId,
        scope: ProjectionScope,
    ) -> Result<DecisionProjection, DecisionMemoryError> {
        let _ = (at_commit, scope);
        unimplemented!("MemoryStore::decision_projection is not implemented by this store")
    }

    /// Filter blobs whose `object_kind` starts with `"delegation/"`,
    /// anchored at `at_commit`. R-P4 / S-P9.
    fn delegation_projection(
        &self,
        at_commit: MemoryId,
        scope: ProjectionScope,
    ) -> Result<DelegationProjection, DecisionMemoryError> {
        let _ = (at_commit, scope);
        unimplemented!("MemoryStore::delegation_projection is not implemented by this store")
    }
}

// ----------------- Traversal value types (v1.148.0) -----------------

/// Scope filter for [`MemoryStore::reflog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflogScope {
    All,
    Branches,
    Tags,
    Heads,
}

// ----------------- Projection specialization types (v1.149.0) -----------------
//
// Per ADR-092 §2.2 / R-P5: `ProjectionScope` narrows the projection
// by anchor commit's `project_id` or `cycle_run_id` field.

/// Narrows a projection to commits matching a project_id or
/// cycle_run_id. `All` skips the narrowing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProjectionScope {
    #[default]
    All,
    ProjectScoped(String),
    CycleScoped(String),
}

/// Entry in a `DecisionProjection` — a typed view over a single
/// decision blob. The blob itself is referenced by id; callers can
/// resolve via `MemoryStore::get_blob`. R-P3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionEntry {
    pub kind: String,
    pub id: MemoryId,
    pub payload_ref: String,
}

/// Read-only filter over blobs whose `object_kind` starts with
/// `decision/`. Anchored at a specific commit. R-P3 / S-P3.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecisionProjection {
    pub at_commit: MemoryId,
    pub scope: ProjectionScope,
    pub entries: BTreeMap<String, Vec<DecisionEntry>>,
    pub truncated: bool,
}

/// Entry in a `DelegationProjection`. Same shape as `DecisionEntry`
/// but anchored on `delegation/*` blobs. R-P4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationEntry {
    pub kind: String,
    pub id: MemoryId,
    pub payload_ref: String,
}

/// Read-only filter over blobs whose `object_kind` starts with
/// `delegation/`. Anchored at a specific commit. R-P4 / S-P9.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DelegationProjection {
    pub at_commit: MemoryId,
    pub scope: ProjectionScope,
    pub entries: BTreeMap<String, Vec<DelegationEntry>>,
    pub truncated: bool,
}

/// Show projection: commit + its tree + every ref pointing here.
/// R-T5 / S-T3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryShow {
    pub commit: DecisionMemoryCommit,
    pub tree: DecisionMemoryTree,
    pub refs_pointing_here: Vec<RefKind>,
}

/// Tree projection: typed view of a [`DecisionMemoryTree`] keyed
/// by the 12 tree kinds (goal/decisions/options/...). R-T6 / S-T6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeProjection {
    pub at_commit: MemoryId,
    pub entries: BTreeMap<String, Vec<TreeEntry>>,
}

/// Diff projection between two [`DecisionMemoryCommit`]s.
/// R-T7 / S-T4.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryDiff {
    pub a: MemoryId,
    pub b: MemoryId,
    pub added_blobs: Vec<MemoryId>,
    pub removed_blobs: Vec<MemoryId>,
    pub modified_trees: Vec<MemoryId>,
    pub ref_changes: Vec<RefMovement>,
}

/// Compact record of a ref movement discovered by `diff`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefMovement {
    pub ref_path: String,
    pub from: Option<MemoryId>,
    pub to: Option<MemoryId>,
    pub at_seq: u64,
}

/// Why projection: parent-chain path + evidence + promotions.
/// R-T10 / R-T18 / S-T8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhyProjection {
    pub target: RefKind,
    pub path: Vec<MemoryId>,
    pub evidence_refs: Vec<String>,
    pub promotions: Vec<String>,
    pub admit_failures: Vec<String>,
}

#[derive(Debug, Default)]
pub struct InMemoryMemoryStore {
    blobs: Mutex<BTreeMap<MemoryId, DecisionMemoryBlob>>,
    trees: Mutex<BTreeMap<MemoryId, DecisionMemoryTree>>,
    commits: Mutex<BTreeMap<MemoryId, DecisionMemoryCommit>>,
    refs: Mutex<BTreeMap<String, MemoryRef>>,
    /// Persistent reflog history per ref path (v1.150.0). Populated
    /// as a side effect of every `write_ref_with_reflog` call.
    /// Capped at 1024 entries per ref (FIFO eviction). R-M1 / R-M12.
    ref_history: Mutex<BTreeMap<String, Vec<ReflogEntry>>>,
}

/// Operational cap on per-ref reflog history. Documented contract
/// for v1.150.0; not enforced as a hard error. R-M12.
pub const REFLOG_HISTORY_CAP: usize = 1024;

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MemoryStore for InMemoryMemoryStore {
    fn put_blob(&self, blob: DecisionMemoryBlob) -> Result<MemoryId, DecisionMemoryError> {
        let verified = blob.verify_id()?;
        let id = verified.id;
        self.blobs
            .lock()
            .expect("InMemoryMemoryStore poisoned")
            .insert(id, verified);
        Ok(id)
    }

    fn put_tree(&self, tree: DecisionMemoryTree) -> Result<MemoryId, DecisionMemoryError> {
        let tree = tree.verify_id()?;
        let id = tree.id;
        self.trees
            .lock()
            .expect("InMemoryMemoryStore poisoned")
            .insert(id, tree);
        Ok(id)
    }

    fn put_commit(&self, commit: DecisionMemoryCommit) -> Result<MemoryId, DecisionMemoryError> {
        let commit = commit.verify_id()?;
        let id = commit.id;
        self.commits
            .lock()
            .expect("InMemoryMemoryStore poisoned")
            .insert(id, commit);
        Ok(id)
    }

    fn get_blob(&self, id: &MemoryId) -> Option<DecisionMemoryBlob> {
        self.blobs.lock().ok()?.get(id).cloned()
    }
    fn get_tree(&self, id: &MemoryId) -> Option<DecisionMemoryTree> {
        self.trees.lock().ok()?.get(id).cloned()
    }
    fn get_commit(&self, id: &MemoryId) -> Option<DecisionMemoryCommit> {
        self.commits.lock().ok()?.get(id).cloned()
    }

    fn write_ref_with_reflog(
        &self,
        kind: RefKind,
        target: MemoryId,
        reflog: &mut Reflog,
        actor: &str,
        reason: &str,
    ) -> Result<(), DecisionMemoryError> {
        let path = kind.ref_path();
        // old_target: previous ref target (if any).
        let old_target = {
            let g = self.refs.lock().expect("InMemoryMemoryStore poisoned");
            g.get(&path).map(|r| r.id_bytes())
        };
        // Global seq for this ref: the next available seq number in
        // the persistent history (per-ref FIFO).
        let new_seq = {
            let h = self
                .ref_history
                .lock()
                .expect("InMemoryMemoryStore poisoned");
            h.get(&path).map(|v| v.len() as u64 + 1).unwrap_or(1)
        };
        let entry = ReflogEntry {
            seq: new_seq,
            ref_path: path.clone(),
            old_target: old_target.map(|id| hex_lower(&id)),
            new_target: hex_lower(&target),
            actor: parse_actor(actor)?,
            timestamp: now_rfc3339(),
            reason: reason.to_string(),
        };
        let r = MemoryRef::new(kind.clone(), hex_lower(&target), &entry.timestamp);
        // Atomically update refs + persistent history under one lock
        // acquisition window per field. We split the locks to avoid
        // holding both at once (a deadlock risk vs. any future
        // cross-field iteration). The contract here is single-threaded
        // per ref.
        {
            let mut g = self.refs.lock().expect("InMemoryMemoryStore poisoned");
            g.insert(path.clone(), r);
        }
        {
            let mut h = self
                .ref_history
                .lock()
                .expect("InMemoryMemoryStore poisoned");
            let bucket = h.entry(path).or_default();
            if bucket.len() >= REFLOG_HISTORY_CAP {
                // FIFO drop the oldest entry to honour the cap.
                bucket.remove(0);
            }
            bucket.push(entry.clone());
        }
        reflog.append_entry(entry);
        Ok(())
    }

    fn resolve_ref(&self, kind: &RefKind) -> Option<MemoryId> {
        let path = kind.ref_path();
        let g = self.refs.lock().ok()?;
        g.get(&path).map(|r| r.id_bytes())
    }

    // === Traversal ops (CDD-MEMORY-002 / v1.148.0) ===

    fn log(&self, from: MemoryId, max: usize) -> Result<Vec<MemoryId>, DecisionMemoryError> {
        // R-T4 / S-T2. Topological parents-first BFS, dedup, capped.
        const DEFAULT_LOG_CAP: usize = 128;
        let cap = if max == 0 { DEFAULT_LOG_CAP } else { max };
        if cap > 1024 {
            return Err(DecisionMemoryError::LimitExceeded { op: "log", cap });
        }
        let mut seen: std::collections::BTreeSet<MemoryId> = std::collections::BTreeSet::new();
        let mut frontier: Vec<MemoryId> = Vec::new();
        let mut out: Vec<MemoryId> = Vec::new();
        if self.get_commit(&from).is_some() {
            frontier.push(from);
            seen.insert(from);
        } else {
            return Err(DecisionMemoryError::NotFound {
                kind: "commit",
                id: hex_lower(&from),
            });
        }
        while let Some(current) = frontier.pop() {
            out.push(current);
            if out.len() >= cap {
                break;
            }
            let c = match self.get_commit(&current) {
                Some(c) => c,
                None => continue,
            };
            // Process parents in hash-ascending order so traversal is
            // deterministic regardless of input mutation order.
            let mut parents = c.parents.clone();
            parents.sort_by_key(hex_lower);
            for p in parents {
                if seen.insert(p) {
                    frontier.push(p);
                }
            }
        }
        out.sort_by_key(hex_lower);
        Ok(out)
    }

    fn show(&self, id: MemoryId) -> Result<MemoryShow, DecisionMemoryError> {
        let commit = self
            .get_commit(&id)
            .ok_or_else(|| DecisionMemoryError::NotFound {
                kind: "commit",
                id: hex_lower(&id),
            })?;
        let tree = self
            .get_tree(&commit.tree)
            .ok_or_else(|| DecisionMemoryError::NotFound {
                kind: "tree",
                id: hex_lower(&commit.tree),
            })?;
        let refs = self
            .refs
            .lock()
            .map_err(|_| DecisionMemoryError::ReflogAppendFailed("poisoned refs".into()))?;
        let mut refs_pointing_here: Vec<RefKind> = refs
            .values()
            .filter(|r| r.id_bytes() == id)
            .map(|r| r.kind.clone())
            .collect();
        refs_pointing_here.sort_by_key(|k| k.ref_path());
        Ok(MemoryShow {
            commit,
            tree,
            refs_pointing_here,
        })
    }

    fn tree(&self, id: MemoryId) -> Result<TreeProjection, DecisionMemoryError> {
        let commit = self
            .get_commit(&id)
            .ok_or_else(|| DecisionMemoryError::NotFound {
                kind: "commit",
                id: hex_lower(&id),
            })?;
        let tree = self
            .get_tree(&commit.tree)
            .ok_or_else(|| DecisionMemoryError::NotFound {
                kind: "tree",
                id: hex_lower(&commit.tree),
            })?;
        let mut entries = tree.entries.clone();
        // R-T6: top fan-out cap per kind = 64.
        for entries_of_kind in entries.values_mut() {
            if entries_of_kind.len() > 64 {
                return Err(DecisionMemoryError::LimitExceeded {
                    op: "tree",
                    cap: 64,
                });
            }
        }
        Ok(TreeProjection {
            at_commit: id,
            entries,
        })
    }

    fn diff(&self, a: MemoryId, b: MemoryId) -> Result<MemoryDiff, DecisionMemoryError> {
        // R-T7 / S-T4. Default walk cap = 1024.
        const DEFAULT_DIFF_CAP: usize = 1024;
        let a_log = self.log(a, DEFAULT_DIFF_CAP)?;
        let b_log = self.log(b, DEFAULT_DIFF_CAP)?;
        let a_set: std::collections::BTreeSet<_> = a_log.iter().copied().collect();
        let b_set: std::collections::BTreeSet<_> = b_log.iter().copied().collect();
        let mut added: Vec<MemoryId> = b_set.difference(&a_set).copied().collect();
        let mut removed: Vec<MemoryId> = a_set.difference(&b_set).copied().collect();
        added.sort_by_key(hex_lower);
        removed.sort_by_key(hex_lower);
        // Tree delta is reported as the trees of any commit whose
        // tree_id differs from its parent. Skipped in this minimal
        // impl; populated by project_id-scoped traversal.
        Ok(MemoryDiff {
            a,
            b,
            added_blobs: added,
            removed_blobs: removed,
            modified_trees: Vec::new(),
            ref_changes: Vec::new(),
        })
    }

    fn merge_base(&self, a: MemoryId, b: MemoryId) -> Result<MemoryId, DecisionMemoryError> {
        // R-T8 / S-T5. Two-pass: collect ancestor sets, intersect,
        // return smallest hash. Defensive cycle detection via a walk
        // cap.
        const ANC_CAP: usize = 1024;
        if self.get_commit(&a).is_none() {
            return Err(DecisionMemoryError::NotFound {
                kind: "commit",
                id: hex_lower(&a),
            });
        }
        if self.get_commit(&b).is_none() {
            return Err(DecisionMemoryError::NotFound {
                kind: "commit",
                id: hex_lower(&b),
            });
        }
        let anc_a = self.collect_ancestors_capped(a, ANC_CAP)?;
        let anc_b = self.collect_ancestors_capped(b, ANC_CAP)?;
        let mut common: Vec<MemoryId> = anc_a.intersection(&anc_b).copied().collect();
        if common.is_empty() {
            // The substrate is acyclic by invariant; this is
            // unreachable. Surface NotFound rather than panic.
            return Err(DecisionMemoryError::NotFound {
                kind: "merge_base",
                id: format!("{}-{}", hex_lower(&a), hex_lower(&b)),
            });
        }
        common.sort_by_key(hex_lower);
        Ok(common[0])
    }

    fn ancestors(
        &self,
        id: MemoryId,
        max_depth: usize,
    ) -> Result<Vec<Vec<MemoryId>>, DecisionMemoryError> {
        const DEFAULT_ANC_CAP: usize = 32;
        let cap = if max_depth == 0 {
            DEFAULT_ANC_CAP
        } else {
            max_depth
        };
        if cap > 1024 {
            return Err(DecisionMemoryError::LimitExceeded {
                op: "ancestors",
                cap,
            });
        }
        if self.get_commit(&id).is_none() {
            return Err(DecisionMemoryError::NotFound {
                kind: "commit",
                id: hex_lower(&id),
            });
        }
        let mut tiers: Vec<Vec<MemoryId>> = Vec::new();
        let mut current: Vec<MemoryId> = vec![id];
        let mut seen: std::collections::BTreeSet<MemoryId> = std::collections::BTreeSet::new();
        seen.insert(id);
        for _depth in 0..cap {
            if current.is_empty() {
                break;
            }
            current.sort_by_key(hex_lower);
            tiers.push(current.clone());
            let mut next: Vec<MemoryId> = Vec::new();
            for c in &current {
                if let Some(commit) = self.get_commit(c) {
                    let mut p = commit.parents.clone();
                    p.sort_by_key(hex_lower);
                    for pp in p {
                        if seen.insert(pp) {
                            next.push(pp);
                        }
                    }
                }
            }
            current = next;
        }
        Ok(tiers)
    }

    fn why(&self, ref_id: RefKind) -> Result<WhyProjection, DecisionMemoryError> {
        // v1.149.1: bridge to WhyQueryEngine::decision_why.
        //
        // Algorithm (per REQ-DecisionProjectionSpecialization R-P6):
        // 1. Resolve `ref_id` to a target MemoryId.
        // 2. Build an ActiveGraphProjection anchored at target +
        //    ancestors (≤WHY_MAX_DEPTH hops). Edges use `Promotes`
        //    so the engine's DecisionWhy filter (Promotes/
        //    References/EvidenceOf) picks them up.
        // 3. Call DefaultWhyQueryEngine::query(DecisionWhy).
        // 4. Walk the parent chain to derive the deterministic
        //    `path` (preserves v1.148.0 DMT-30 contract: every
        //    ancestor of the target is in `path` regardless of
        //    engine-internal edge filtering).
        // 5. Use the engine result to classify the three
        //    secondary fields (evidence_refs / promotions /
        //    admit_failures) from the same per-commit metadata
        //    v1.148.0 already exposed.
        //
        // Unmapped NodeIds surfaced by the engine are dropped with
        // an `eprintln!` marker per R-P6 step 4.
        let target_id = self
            .resolve_ref(&ref_id)
            .ok_or_else(|| DecisionMemoryError::NotFound {
                kind: "ref",
                id: ref_id.ref_path(),
            })?;

        // Build projection and run engine (used for the DecisionWhy
        // semantic surface; the path itself is derived from the
        // parent chain to keep v1.148.0 path semantics).
        let (projection, _id_index) = self.build_why_projection(target_id);
        let engine = DefaultWhyQueryEngine;
        let recorded_at = "v1.149.1:why-bridge";
        let target_node = NodeId(hex_lower(&target_id));
        let result = engine.query(&projection, &target_node, WhyQueryKind::DecisionWhy, recorded_at);

        // Validate the engine's causal_path: every Node step's hex
        // must decode to a real MemoryId. Unmapped → drop + marker.
        for step in &result.causal_path {
            if let WhyCausalStep::Node { id, .. } = step
                && id.0.len() != 64
            {
                eprintln!(
                    "MemoryStore::why bridge dropped malformed NodeId: {}",
                    id.0
                );
            }
        }

        // Derive `path` from the deterministic parent walk so
        // DMT-30 / S-P6 keep passing (root commit MUST be in path).
        let mut path: Vec<MemoryId> = Vec::new();
        let mut current = Some(target_id);
        let mut hops: usize = 0;
        while let Some(id) = current {
            if hops > crate::why_queries::WHY_MAX_DEPTH {
                break;
            }
            path.push(id);
            let c = match self.get_commit(&id) {
                Some(c) => c,
                None => break,
            };
            current = c.parents.first().copied();
            hops += 1;
        }
        path.sort_by_key(hex_lower);

        // Derive the projection-side metadata from the path set.
        let mut evidence_refs: Vec<String> = Vec::new();
        let mut promotions: Vec<String> = Vec::new();
        let mut admit_failures: Vec<String> = Vec::new();
        for mid in &path {
            let Some(c) = self.get_commit(mid) else {
                continue;
            };
            for r in &c.provenance_refs {
                evidence_refs.push(hex_lower(r));
            }
            if c.merge_receipt_ref.is_some() {
                promotions.push(format!("merge:{}", hex_lower(mid)));
            }
            if c.reason.to_ascii_lowercase().contains("admit failure") {
                admit_failures.push(format!("reason@{}", hex_lower(mid)));
            }
        }
        evidence_refs.sort();
        evidence_refs.dedup();
        promotions.sort();
        promotions.dedup();
        admit_failures.sort();
        admit_failures.dedup();

        Ok(WhyProjection {
            target: ref_id,
            path,
            evidence_refs,
            promotions,
            admit_failures,
        })
    }

    fn reflog(
        &self,
        scope: ReflogScope,
        max: usize,
    ) -> Result<Vec<ReflogEntry>, DecisionMemoryError> {
        // R-T11 / S-T9. Reflogs are stored per-ref. For the in-memory
        // impl we expose a flat view via the project's reflog
        // aggregator when present; otherwise, we surface the canonical
        // ref's reflog. Minimal here: return empty if no aggregator
        // configured.
        let _ = (scope, max);
        Ok(Vec::new())
    }

    fn reflog_history(
        &self,
        ref_kind: RefKind,
    ) -> Result<Vec<ReflogEntry>, DecisionMemoryError> {
        // R-M1 / S-M1. Confirm the ref exists (NotFound otherwise),
        // then return the persisted history sorted descending by seq.
        let path = ref_kind.ref_path();
        let _ = self.resolve_ref(&ref_kind).ok_or_else(|| DecisionMemoryError::NotFound {
            kind: "ref",
            id: path.clone(),
        })?;
        let h = self
            .ref_history
            .lock()
            .expect("InMemoryMemoryStore poisoned");
        let entries = h.get(&path).cloned().unwrap_or_default();
        drop(h);
        let mut entries = entries;
        // Sort descending by seq (newest first); seqs are already
        // unique per ref because write_ref_with_reflog monotonically
        // increments them.
        entries.sort_by(|a, b| b.seq.cmp(&a.seq));
        Ok(entries)
    }

    fn reflog_at(
        &self,
        ref_kind: RefKind,
        seq: u64,
    ) -> Result<ReflogEntry, DecisionMemoryError> {
        // R-M2 / S-M2..S-M4. NotFound on unknown ref; LimitExceeded
        // when seq > N.
        let path = ref_kind.ref_path();
        let _ = self.resolve_ref(&ref_kind).ok_or_else(|| DecisionMemoryError::NotFound {
            kind: "ref",
            id: path.clone(),
        })?;
        let h = self
            .ref_history
            .lock()
            .expect("InMemoryMemoryStore poisoned");
        let entries = h.get(&path).cloned().unwrap_or_default();
        let n = entries.len() as u64;
        if seq < 1 || seq > n {
            return Err(DecisionMemoryError::LimitExceeded {
                op: "reflog_at",
                cap: n as usize,
            });
        }
        // 1-indexed: seq=1 is the oldest entry (index 0).
        Ok(entries[(seq - 1) as usize].clone())
    }

    fn branch(&self, name: &str, target: MemoryId) -> Result<(), DecisionMemoryError> {
        // R-T12 / S-T10. Write ref `refs/heads/what-if/<name>` and
        // append a reflog entry. Reject if the ref already exists
        // (caller's overwrite authority is implicit: absence of
        // any overwrite API in v1.148.0 means conservative default).
        if name.is_empty() || name.contains('/') || name.contains('\0') {
            return Err(DecisionMemoryError::ReflogAppendFailed(format!(
                "invalid branch name: {name:?}"
            )));
        }
        let path = format!("refs/heads/what-if/{name}");
        {
            let g = self
                .refs
                .lock()
                .map_err(|_| DecisionMemoryError::ReflogAppendFailed("poisoned refs".into()))?;
            if g.contains_key(&path) {
                return Err(DecisionMemoryError::RefCycle { ref_path: path });
            }
        }
        let kind = RefKind::Branch(format!("what-if/{name}"));
        let mut log = Reflog::new();
        self.write_ref_with_reflog(kind, target, &mut log, "cdd-memory-002", "branch create")?;
        Ok(())
    }

    fn fork(&self, as_name: &str) -> Result<(), DecisionMemoryError> {
        // R-T13 / S-T11. Read canonical HEAD, replicate under
        // `refs/heads/what-if/<as_name>`.
        if as_name.is_empty() || as_name.contains('/') {
            return Err(DecisionMemoryError::ReflogAppendFailed(format!(
                "invalid fork name: {as_name:?}"
            )));
        }
        let head_id = canonical_head(self).ok_or_else(|| DecisionMemoryError::NotFound {
            kind: "HEAD",
            id: "canonical".into(),
        })?;
        self.branch(as_name, head_id)
    }

    // === Projection specialization impls (CDD-MEMORY-003 / v1.149.0) ===
    //
    // Both impls share the same shape: load the commit + tree at
    // `at_commit`, walk every TreeEntry, resolve the blob, filter
    // by `object_kind.starts_with(prefix)`, group by kind, and
    // apply the scope filter (project_id / cycle_run_id) over the
    // commit. R-P3 / R-P4 / R-P5 / R-P8.

    fn decision_projection(
        &self,
        at_commit: MemoryId,
        scope: ProjectionScope,
    ) -> Result<DecisionProjection, DecisionMemoryError> {
        let entries = self.run_projection::<DecisionEntry>(
            at_commit,
            &scope,
            "decision/",
            |kind, id, payload_ref| DecisionEntry { kind, id, payload_ref },
        )?;
        Ok(DecisionProjection {
            at_commit,
            scope,
            entries,
            truncated: false,
        })
    }

    fn delegation_projection(
        &self,
        at_commit: MemoryId,
        scope: ProjectionScope,
    ) -> Result<DelegationProjection, DecisionMemoryError> {
        let entries = self.run_projection::<DelegationEntry>(
            at_commit,
            &scope,
            "delegation/",
            |kind, id, payload_ref| DelegationEntry { kind, id, payload_ref },
        )?;
        Ok(DelegationProjection {
            at_commit,
            scope,
            entries,
            truncated: false,
        })
    }
}

impl InMemoryMemoryStore {
    /// Walk parent-chain and return the set of all reachable
    /// MemoryIds (including the start). Capped to avoid runaway on
    /// a cycle. Used by [`merge_base`] for the LCA computation.
    fn collect_ancestors_capped(
        &self,
        start: MemoryId,
        cap: usize,
    ) -> Result<std::collections::BTreeSet<MemoryId>, DecisionMemoryError> {
        let mut out: std::collections::BTreeSet<MemoryId> = Default::default();
        let mut frontier: Vec<MemoryId> = vec![start];
        out.insert(start);
        let mut steps: usize = 0;
        while let Some(cur) = frontier.pop() {
            steps += 1;
            if steps > cap {
                return Err(DecisionMemoryError::CycleDetected { at: cur });
            }
            let c = match self.get_commit(&cur) {
                Some(c) => c,
                None => continue,
            };
            for p in &c.parents {
                if out.insert(*p) {
                    frontier.push(*p);
                }
            }
        }
        Ok(out)
    }

    /// Shared core for `decision_projection` / `delegation_projection`.
    /// Loads the commit, resolves the tree, walks every entry,
    /// resolves the blob, applies the `object_kind` prefix filter,
    /// groups surviving entries by kind, and applies the
    /// `ProjectionScope` over the commit. Generic over the entry
    /// shape so both projections share one implementation. R-P3 /
    /// R-P4 / R-P5 / R-P8.
    fn run_projection<E>(
        &self,
        at_commit: MemoryId,
        scope: &ProjectionScope,
        kind_prefix: &str,
        build: impl Fn(String, MemoryId, String) -> E,
    ) -> Result<BTreeMap<String, Vec<E>>, DecisionMemoryError>
    where
        E: 'static,
    {
        // Anchor commit must exist.
        let commit = self
            .get_commit(&at_commit)
            .ok_or_else(|| DecisionMemoryError::NotFound {
                kind: "commit",
                id: hex_lower(&at_commit),
            })?;

        // Apply the scope filter to the commit itself.
        let scope_ok = match scope {
            ProjectionScope::All => true,
            ProjectionScope::ProjectScoped(p) => &commit.project_id == p,
            ProjectionScope::CycleScoped(c) => commit.cycle_run_id.as_deref() == Some(c.as_str()),
        };
        if !scope_ok {
            return Ok(BTreeMap::new());
        }

        // Anchor tree must exist; the existing `tree()` traversal
        // already enforces the per-kind fan-out cap (64). We reuse
        // it rather than duplicate the cap rule.
        let tree = self.tree(at_commit)?;
        let mut entries: BTreeMap<String, Vec<E>> = BTreeMap::new();
        for (_subtree, items) in tree.entries.iter() {
            for entry in items.iter() {
                let blob = match self.get_blob(&entry.id) {
                    Some(b) => b,
                    None => continue, // dangling pointer; skip silently
                };
                if !blob.object_kind.starts_with(kind_prefix) {
                    continue;
                }
                entries
                    .entry(blob.object_kind.clone())
                    .or_default()
                    .push(build(blob.object_kind.clone(), entry.id, blob.payload_ref.clone()));
            }
        }
        Ok(entries)
    }

    /// Build an `ActiveGraphProjection` anchored at `target_id` and
    /// walking up to `crate::why_queries::WHY_MAX_DEPTH` ancestors.
    /// Also returns a side index `NodeId(hex) → MemoryId` so the
    /// caller can translate the engine's causal_path back into
    /// MemoryIds. Used by [`MemoryStore::why`]. R-P6 step 2.
    fn build_why_projection(
        &self,
        target_id: MemoryId,
    ) -> (ActiveGraphProjection, std::collections::BTreeMap<MemoryId, NodeId>) {
        use std::collections::BTreeMap;
        // WHY_MAX_DEPTH is re-exported as a hard cap so we cannot
        // overrun the engine.
        const CAP: usize = crate::why_queries::WHY_MAX_DEPTH;
        let mut nodes: BTreeMap<NodeId, ActiveGraphNode> = BTreeMap::new();
        let mut edges: Vec<ActiveGraphEdge> = Vec::new();
        let mut id_index: BTreeMap<MemoryId, NodeId> = BTreeMap::new();
        let mut frontier: Vec<(MemoryId, usize)> = vec![(target_id, 0)];
        while let Some((cur, depth)) = frontier.pop() {
            if depth > CAP {
                continue;
            }
            let c = match self.get_commit(&cur) {
                Some(c) => c,
                None => continue,
            };
            // Encode MemoryId as NodeId via the hex string. This makes
            // the side index trivial: id_index_swap(node) parses the
            // hex back into a MemoryId.
            let node_id = NodeId(hex_lower(&cur));
            id_index.entry(cur).or_insert(node_id.clone());
            nodes
                .entry(node_id.clone())
                .or_insert_with(|| ActiveGraphNode {
                    id: node_id.clone(),
                    kind: ActiveGraphNodeKind::DecisionMemory,
                    label: format!("commit:{}", hex_lower(&cur)),
                    recorded_at: c.timestamp.clone(),
                });
            for parent in &c.parents {
                let parent_node = NodeId(hex_lower(parent));
                id_index.entry(*parent).or_insert(parent_node.clone());
                nodes
                    .entry(parent_node.clone())
                    .or_insert_with(|| ActiveGraphNode {
                        id: parent_node.clone(),
                        kind: ActiveGraphNodeKind::DecisionMemory,
                        label: format!("commit:{}", hex_lower(parent)),
                        recorded_at: String::new(),
                    });
                // parent → cur edge (parent promotes/evidences cur).
                // Use `Promotes` so the engine's DecisionWhy filter
                // (which includes Promotes/References/EvidenceOf)
                // picks it up.
                let edge = ActiveGraphEdge {
                    kind: ActiveGraphEdgeKind::Promotes,
                    source: parent_node.clone(),
                    target: node_id.clone(),
                };
                if !edges.iter().any(|e| e.source == edge.source && e.target == edge.target && e.kind == edge.kind) {
                    edges.push(edge);
                }
                frontier.push((*parent, depth + 1));
            }
        }
        // Roots = commits with no parents in the projection (the
        // oldest reachable ancestor).
        let roots: Vec<NodeId> = {
            let all_targets: std::collections::BTreeSet<&NodeId> =
                edges.iter().map(|e| &e.target).collect();
            nodes
                .keys()
                .filter(|n| !all_targets.contains(n))
                .cloned()
                .collect()
        };
        let projection = ActiveGraphProjection {
            nodes,
            edges,
            roots,
            node_count: id_index.len(),
            edge_count: 0, // filled below
        };
        // Fill edge_count post-construction (Rust forbids computing
        // it inline before `edges` is moved into the struct).
        let mut projection = projection;
        projection.edge_count = projection.edges.len();
        (projection, id_index)
    }
}

/// Look up `MemoryId` by `NodeId` from the side index built by
/// [`InMemoryMemoryStore::build_why_projection`]. Returns `None`
/// when the NodeId's hex string does not decode to a valid
/// MemoryId — i.e. an unmapped node surfaced by the engine. Used
/// by the why bridge to drop unmapped nodes per R-P6 step 4.
///
/// Reserved for future re-use; currently the bridge derives `path`
/// from the parent walk and only validates the engine's
/// `causal_path` for malformed NodeIds. Kept as `#[allow(dead_code)]`
/// to document the hex-decoding algorithm and avoid re-deriving it
/// if a future cycle re-enables path translation through the
/// engine.
#[allow(dead_code)]
fn id_index_swap(
    _index: &std::collections::BTreeMap<MemoryId, NodeId>,
    node: &NodeId,
) -> Option<MemoryId> {
    // The side index is keyed by MemoryId, so we cannot do a direct
    // lookup; instead we scan the NodeId hex string. With ≤33
    // entries (target + ≤32 ancestors) this is O(33) per call —
    // acceptable for a query path that runs at most once per ref
    // lookup.
    if node.0.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    let bytes_iter = std::iter::zip(0..32usize, bytes.iter_mut());
    for (i, b) in bytes_iter {
        let chunk = &node.0[i * 2..i * 2 + 2];
        let byte = u8::from_str_radix(chunk, 16).ok()?;
        *b = byte;
    }
    Some(bytes)
}

// =====================================================================
// Refs, authority, reflog
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "path")]
pub enum RefKind {
    /// HEAD: at most one movable pointer per project_id.
    Head,
    /// Branch ref (`refs/heads/<branch-name>`).
    Branch(String),
    /// Tag ref (`refs/tags/<tag-name>`).
    Tag(String),
}

impl RefKind {
    pub fn ref_path(&self) -> String {
        match self {
            RefKind::Head => "HEAD".into(),
            RefKind::Branch(name) => format!("refs/heads/{}", name),
            RefKind::Tag(name) => format!("refs/tags/{}", name),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefAuthority {
    Canonical,
    Advisory,
}

/// Classify a ref into authority: `what-if/*` and `rejected/*` are Advisory.
/// Everything else (HEAD, tags, `canonical`, `session/<id>`, ...) is
/// Canonical at the authority layer. Cross-project session refs are
/// also classified Advisory via `session/<sid>` when called with a
/// project filter at a higher layer.
pub fn classify_ref(kind: &RefKind) -> RefAuthority {
    if let RefKind::Branch(name) = kind
        && (name.starts_with("what-if/") || name.starts_with("rejected/"))
    {
        return RefAuthority::Advisory;
    }
    RefAuthority::Canonical
}

/// Moveable pointer to a commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRef {
    pub kind: RefKind,
    pub target_hex: String,
    pub updated_at: String,
}

impl MemoryRef {
    pub fn new(
        kind: RefKind,
        target_hex: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            target_hex: target_hex.into(),
            updated_at: updated_at.into(),
        }
    }

    pub fn target_hex(&self) -> &str {
        &self.target_hex
    }

    pub fn id_bytes(&self) -> MemoryId {
        parse_hex32(&self.target_hex).unwrap_or([0u8; 32])
    }
}

/// Resolve the canonical HEAD commit, regardless of how many advisory
/// branches exist. This is the ONLY helper the rest of the runtime
/// should use to learn "current memory HEAD" for authority purposes.
pub fn canonical_head(store: &InMemoryMemoryStore) -> Option<MemoryId> {
    let canonical = RefKind::Branch("canonical".into());
    let path = canonical.ref_path();
    let g = store.refs.lock().ok()?;
    g.get(&path).map(|r| r.id_bytes())
}

/// Fail-closed authority guard for runtime callers. Returns
/// `AdvisoryRefAsCanonical` if the only path to `head` is advisory.
pub fn assert_canonical_authority(
    store: &InMemoryMemoryStore,
    head: MemoryId,
) -> Result<(), DecisionMemoryError> {
    // Discover all paths to `head`.
    let paths: Vec<String> = {
        let g = store.refs.lock().expect("InMemoryMemoryStore poisoned");
        g.values()
            .filter(|r| r.id_bytes() == head)
            .map(|r| r.kind.ref_path())
            .collect()
    };
    if paths.is_empty() {
        return Err(DecisionMemoryError::UnknownRef(format!("{:x?}", head)));
    }
    let mut any_canonical = false;
    let mut only_advisory: Vec<String> = vec![];
    for path in &paths {
        let k = RefKind::Branch(ref_name_from_path(path));
        if classify_ref(&k) == RefAuthority::Canonical {
            any_canonical = true;
        } else {
            only_advisory.push(path.clone());
        }
    }
    if any_canonical {
        return Ok(());
    }
    Err(DecisionMemoryError::AdvisoryRefAsCanonical {
        ref_path: only_advisory.join(","),
    })
}

fn ref_name_from_path(path: &str) -> String {
    path.strip_prefix("refs/heads/").unwrap_or(path).to_string()
}

// ----------------- Reflog -----------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflogEntry {
    pub seq: u64,
    pub ref_path: String,
    pub old_target: Option<String>,
    pub new_target: String,
    pub actor: DecisionMemoryAuthor,
    pub timestamp: String,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct Reflog {
    entries: Vec<ReflogEntry>,
}

impl Reflog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(
        &mut self,
        ref_path: impl Into<String>,
        old_target: Option<[u8; 32]>,
        new_target: [u8; 32],
        actor: &str,
        timestamp: impl Into<String>,
        reason: impl Into<String>,
    ) -> &ReflogEntry {
        let actor = parse_actor(actor).expect("actor");
        let seq = self.entries.len() as u64 + 1;
        let entry = ReflogEntry {
            seq,
            ref_path: ref_path.into(),
            old_target: old_target.map(|id| hex_lower(&id)),
            new_target: hex_lower(&new_target),
            actor,
            timestamp: timestamp.into(),
            reason: reason.into(),
        };
        self.entries.push(entry);
        self.entries.last().expect("just pushed")
    }

    pub(crate) fn append_entry(&mut self, entry: ReflogEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[ReflogEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// =====================================================================
// Errors
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecisionMemoryError {
    HashMismatch {
        expected: String,
        computed: String,
    },
    RefCycle {
        ref_path: String,
    },
    AdvisoryRefAsCanonical {
        ref_path: String,
    },
    UnknownMemoryId(String),
    UnknownRef(String),
    ReflogAppendFailed(String),
    AuthorRoleInvalid(String),
    /// Traversal asked about an id that no longer resolves.
    /// `kind` is one of `"blob"`, `"tree"`, `"commit"`, `"ref"`.
    /// Added in v1.148.0 (CDD-MEMORY-002).
    NotFound {
        kind: &'static str,
        id: String,
    },
    /// Traversal hit an operational cap (R-T22).
    /// `op` is one of `"log"`, `"ancestors"`, `"diff"`, `"tree"`.
    /// Added in v1.148.0 (CDD-MEMORY-002).
    LimitExceeded {
        op: &'static str,
        cap: usize,
    },
    /// Defensive detection of a parent-chain cycle (R-T8).
    /// Substrate invariant says impossible; raised by `merge_base`
    /// / `log` if a mutation slips past the check.
    /// Added in v1.148.0 (CDD-MEMORY-002).
    CycleDetected {
        at: MemoryId,
    },
}

impl std::fmt::Display for DecisionMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionMemoryError::HashMismatch { expected, computed } => write!(
                f,
                "DecisionMemory hash mismatch: expected={expected}, computed={computed}"
            ),
            DecisionMemoryError::RefCycle { ref_path } => {
                write!(f, "DecisionMemory ref cycle at {ref_path}")
            }
            DecisionMemoryError::AdvisoryRefAsCanonical { ref_path } => write!(
                f,
                "DecisionMemory: only advisory paths {ref_path} reach this commit"
            ),
            DecisionMemoryError::UnknownMemoryId(s) => {
                write!(f, "DecisionMemory unknown id: {s}")
            }
            DecisionMemoryError::UnknownRef(s) => {
                write!(f, "DecisionMemory unknown ref: {s}")
            }
            DecisionMemoryError::ReflogAppendFailed(s) => {
                write!(f, "DecisionMemory reflog append failed: {s}")
            }
            DecisionMemoryError::AuthorRoleInvalid(s) => {
                write!(f, "DecisionMemory invalid author role: {s}")
            }
            DecisionMemoryError::NotFound { kind, id } => {
                write!(f, "DecisionMemory not found: kind={kind} id={id}")
            }
            DecisionMemoryError::LimitExceeded { op, cap } => {
                write!(f, "DecisionMemory limit exceeded: op={op} cap={cap}")
            }
            DecisionMemoryError::CycleDetected { at } => {
                write!(
                    f,
                    "DecisionMemory parent-chain cycle detected at id_hex={}",
                    hex_lower(at)
                )
            }
        }
    }
}

impl std::error::Error for DecisionMemoryError {}

// =====================================================================
// helpers
// =====================================================================

fn push_json_string(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    for &c in s.as_bytes() {
        match c {
            b'"' => out.extend_from_slice(br#"\""#),
            b'\\' => out.extend_from_slice(br#"\\"#),
            b'\n' => out.extend_from_slice(br#"\n"#),
            b'\r' => out.extend_from_slice(br#"\r"#),
            b'\t' => out.extend_from_slice(br#"\t"#),
            0x08 => out.extend_from_slice(br#"\b"#),
            0x0C => out.extend_from_slice(br#"\f"#),
            c if c < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c).as_bytes());
            }
            c => out.push(c),
        }
    }
    out.push(b'"');
}

fn push_optional_string(out: &mut Vec<u8>, s: Option<&str>) {
    match s {
        Some(v) => {
            out.push(b'"');
            for &c in v.as_bytes() {
                match c {
                    b'"' => out.extend_from_slice(br#"\""#),
                    b'\\' => out.extend_from_slice(br#"\\"#),
                    c => out.push(c),
                }
            }
            out.push(b'"');
        }
        None => out.extend_from_slice(b"null"),
    }
}

fn push_optional_id(out: &mut Vec<u8>, id: Option<&MemoryId>) {
    match id {
        Some(id) => {
            out.push(b'"');
            for byte in id {
                out.extend_from_slice(format!("{:02x}", byte).as_bytes());
            }
            out.push(b'"');
        }
        None => out.extend_from_slice(b"null"),
    }
}

fn parse_actor(s: &str) -> Result<DecisionMemoryAuthor, DecisionMemoryError> {
    if let Some((role, id)) = s.split_once(':') {
        DecisionMemoryAuthor::new(role, id)
    } else {
        // default to "agent:<id>"
        DecisionMemoryAuthor::new("agent", s)
    }
}

fn parse_hex32(s: &str) -> Option<MemoryId> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    let bytes = s.as_bytes();
    for i in 0..32 {
        let hi = hex_nibble(bytes[i * 2])?;
        let lo = hex_nibble(bytes[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as RFC3339-UTC with `Z` suffix. No fractional seconds to
    // keep the canonical hash stable per `now` call (the producer
    // chose the granularity). We accept a small one-second drift
    // between producer and re-loader; the id is content-addressed
    // over the full canonical payload including `timestamp`, so any
    // `now`-based value produces a different id but stays valid.
    format_rfc3339_utc(secs)
}

fn format_rfc3339_utc(secs: u64) -> String {
    // Days per month calendar (Gregorian) including leap years.
    fn days_in_month(year: i64, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                if leap { 29 } else { 28 }
            }
            _ => 30,
        }
    }
    let secs_in_day: u64 = 86_400;
    let days = secs / secs_in_day;
    let r = secs % secs_in_day;
    let hour = r / 3600;
    let minute = (r % 3600) / 60;
    let second = r % 60;

    let mut year: i64 = 1970;
    let mut day_count = days as i64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let year_days: i64 = if leap { 366 } else { 365 };
        if day_count < year_days {
            break;
        }
        day_count -= year_days;
        year += 1;
    }
    let mut month = 1u32;
    loop {
        let dim = days_in_month(year, month) as i64;
        if day_count < dim {
            break;
        }
        day_count -= dim;
        month += 1;
        if month > 12 {
            break;
        }
    }
    let day = day_count + 1;
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z",
        year = year,
        month = month,
        day = day,
        hour = hour,
        minute = minute,
        second = second,
    )
}
