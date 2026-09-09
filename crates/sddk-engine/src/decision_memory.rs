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
}

#[derive(Debug, Default)]
pub struct InMemoryMemoryStore {
    blobs: Mutex<BTreeMap<MemoryId, DecisionMemoryBlob>>,
    trees: Mutex<BTreeMap<MemoryId, DecisionMemoryTree>>,
    commits: Mutex<BTreeMap<MemoryId, DecisionMemoryCommit>>,
    refs: Mutex<BTreeMap<String, MemoryRef>>,
}

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
        let (old_target, old_seq) = {
            let g = self.refs.lock().expect("InMemoryMemoryStore poisoned");
            let prev = g.get(&path).cloned();
            (prev.as_ref().map(|r| r.id_bytes()), reflog.len() as u64)
        };
        let new_seq = old_seq + 1;
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
        let mut g = self.refs.lock().expect("InMemoryMemoryStore poisoned");
        g.insert(path, r);
        reflog.append_entry(entry);
        Ok(())
    }

    fn resolve_ref(&self, kind: &RefKind) -> Option<MemoryId> {
        let path = kind.ref_path();
        let g = self.refs.lock().ok()?;
        g.get(&path).map(|r| r.id_bytes())
    }
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
    HashMismatch { expected: String, computed: String },
    RefCycle { ref_path: String },
    AdvisoryRefAsCanonical { ref_path: String },
    UnknownMemoryId(String),
    UnknownRef(String),
    ReflogAppendFailed(String),
    AuthorRoleInvalid(String),
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
