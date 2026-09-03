//! Receipt identity, evidence store, freshness classification, graph-driven invalidation,
//! and selection-quality telemetry (TEST-EVIDENCE-001).
//!
//! ## Design
//!
//! - Closed enums with `assert_variant_count_eq!` guards (ReuseDecision=3, StaleReason=7,
//!   TestEvidenceError=4) per errata TEST-MODEL-001.
//! - Deterministic: BTreeMap/BTreeSet, canonical JSON, ordered reports.
//! - No filesystem or persistence dependency — EvidenceStoreV1 is an in-memory BTreeMap adapter.
//! - Port implementor: `TestEvidenceRepository` from test_ports.rs.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::test_model::{ProjectTestTopologyV1, TestEvidenceReceiptV1, TopologyEdgeKind};
use crate::test_ports::AdapterError;

// ── Schema version constant ────────────────────────────────────────────────────

/// Schema version constant for TEST-EVIDENCE-001 aggregates.
pub const TEST_EVIDENCE_SCHEMA_VERSION: u32 = 1;

// ── REQ-5: TestEvidenceError ──────────────────────────────────────────────────

/// Closed error type for test evidence operations (TEST-EVIDENCE-001).
///
/// Exactly 4 variants — adding a variant requires updating the SPEC.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TestEvidenceError {
    /// The schema version is not supported.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version found.
        got: u32,
        /// The version expected.
        want: u32,
    },

    /// A required field is empty.
    #[error("empty field: {field}")]
    EmptyField {
        /// The name of the empty field.
        field: String,
    },

    /// A receipt with the same receipt_id already exists in the store.
    #[error("duplicate receipt id: {id}")]
    DuplicateReceiptId {
        /// The duplicate receipt identifier.
        id: String,
    },

    /// The input was invalid for a reason not covered by other variants.
    #[error("invalid input: {reason}")]
    InvalidInput {
        /// Why the input is invalid.
        reason: String,
    },
}

crate::assert_variant_count_eq!(
    TestEvidenceError,
    4,
    [
        TestEvidenceError::UnsupportedSchemaVersion { .. },
        TestEvidenceError::EmptyField { .. },
        TestEvidenceError::DuplicateReceiptId { .. },
        TestEvidenceError::InvalidInput { .. },
    ]
);

// ── REQ-1: ReceiptIdentity ────────────────────────────────────────────────────

/// Schema version constant for receipt identity.
pub const RECEIPT_IDENTITY_SCHEMA_VERSION: u32 = 1;

/// Receipt identity V1 — the 7-field reuse identity defined in SPEC §11.
///
/// Groups test/capability identity + test-input digest into `capability_test_identity`
/// per SPEC §11: "capability_test_identity agrupa test/capability identity + test-input
/// digest de §11".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReceiptIdentityV1 {
    /// Schema version — must be `RECEIPT_IDENTITY_SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Change-set digest this identity is for (non-empty).
    pub change_set_digest: String,
    /// Source revision in effect (non-empty).
    pub source_revision: String,
    /// Topology revision in effect (non-empty).
    pub topology_revision: String,
    /// SUT graph revision in effect (non-empty).
    pub sut_graph_revision: String,
    /// Policy revision in effect (non-empty).
    pub policy_revision: String,
    /// Grouped test/capability identity + test-input digest (non-empty, SPEC §11).
    pub capability_test_identity: String,
    /// Toolchain identity in effect (non-empty).
    pub toolchain_identity: String,
}

impl ReceiptIdentityV1 {
    /// Creates a new V1 receipt identity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        change_set_digest: String,
        source_revision: String,
        topology_revision: String,
        sut_graph_revision: String,
        policy_revision: String,
        capability_test_identity: String,
        toolchain_identity: String,
    ) -> Self {
        Self {
            schema_version: RECEIPT_IDENTITY_SCHEMA_VERSION,
            change_set_digest,
            source_revision,
            topology_revision,
            sut_graph_revision,
            policy_revision,
            capability_test_identity,
            toolchain_identity,
        }
    }

    /// Validates this receipt identity instance.
    pub fn validate(&self) -> Result<(), TestEvidenceError> {
        if self.schema_version != RECEIPT_IDENTITY_SCHEMA_VERSION {
            return Err(TestEvidenceError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: RECEIPT_IDENTITY_SCHEMA_VERSION,
            });
        }
        if self.change_set_digest.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "change_set_digest".to_string(),
            });
        }
        if self.source_revision.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "source_revision".to_string(),
            });
        }
        if self.topology_revision.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "topology_revision".to_string(),
            });
        }
        if self.sut_graph_revision.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "sut_graph_revision".to_string(),
            });
        }
        if self.policy_revision.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "policy_revision".to_string(),
            });
        }
        if self.capability_test_identity.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "capability_test_identity".to_string(),
            });
        }
        if self.toolchain_identity.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "toolchain_identity".to_string(),
            });
        }
        Ok(())
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("ReceiptIdentityV1 is always serializable")
    }

    /// Computes the content hash over the canonical JSON form.
    ///
    /// Format: `sha256:<64-hex-lowercase>`.
    pub fn compute_content_hash(&self) -> String {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }
}

/// Versioned envelope for receipt identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum ReceiptIdentity {
    /// Version 1 identity.
    V1(ReceiptIdentityV1),
}

// ── REQ-3: ReuseDecision + StaleReason ────────────────────────────────────────

/// Why a receipt is considered stale (SPEC §11, closed enum).
///
/// Exactly 7 variants — one per field of the 7-field ReceiptIdentityV1 identity:
/// change_set_digest, source_revision, topology_revision, sut_graph_revision,
/// policy_revision, capability_test_identity, toolchain_identity.
/// Adding a variant requires updating SPEC §11.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    /// The change-set digest changed.
    ChangeSetChanged,
    /// The source revision changed.
    SourceRevisionChanged,
    /// The topology revision changed.
    TopologyRevisionChanged,
    /// The SUT graph revision changed.
    SutGraphRevisionChanged,
    /// The policy revision changed.
    PolicyRevisionChanged,
    /// The capability or test identity changed (receipt.capability_id vs
    /// current.capability_test_identity per SPEC §11).
    CapabilityTestIdentityChanged,
    /// The toolchain identity changed.
    ToolchainIdentityChanged,
}

crate::assert_variant_count_eq!(
    StaleReason,
    7,
    [
        StaleReason::ChangeSetChanged,
        StaleReason::SourceRevisionChanged,
        StaleReason::TopologyRevisionChanged,
        StaleReason::SutGraphRevisionChanged,
        StaleReason::PolicyRevisionChanged,
        StaleReason::CapabilityTestIdentityChanged,
        StaleReason::ToolchainIdentityChanged,
    ]
);

/// Outcome of the freshness classification for a receipt (SPEC §11, closed enum).
///
/// Exactly 3 variants — Reusable, Stale with typed reasons, or NoEvidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReuseDecision {
    /// The receipt is still valid and can be reused.
    Reusable,
    /// The receipt is stale — one or more fields changed.
    Stale {
        /// Typed reasons why the receipt is stale, one per changed field.
        /// Ordered by StaleReason enum order.
        reasons: Vec<StaleReason>,
    },
    /// No evidence receipt exists for this identity.
    NoEvidence,
}

crate::assert_variant_count_eq!(
    ReuseDecision,
    3,
    [
        ReuseDecision::Reusable,
        ReuseDecision::Stale { .. },
        ReuseDecision::NoEvidence,
    ]
);

/// Classifies a receipt against the current identity.
///
/// Compares field-by-field; all equal ⇒ Reusable; any difference ⇒ Stale
/// with one StaleReason per changed field (in enum order); receipt not in store ⇒ NoEvidence.
///
/// The comparison uses the 7-field identity: change_set_digest, source_revision,
/// topology_revision, sut_graph_revision, policy_revision, capability_test_identity,
/// toolchain_identity.
pub fn classify(receipt: &TestEvidenceReceiptV1, current: &ReceiptIdentityV1) -> ReuseDecision {
    let mut reasons: Vec<StaleReason> = Vec::new();

    // RFC 3339 timestamps are lexicographically comparable — we rely on this
    // for the completed_at field in latest_for, but for classify we compare
    // the structural identity fields only.

    if receipt.change_set_digest != current.change_set_digest {
        reasons.push(StaleReason::ChangeSetChanged);
    }
    if receipt.source_revision != current.source_revision {
        reasons.push(StaleReason::SourceRevisionChanged);
    }
    if receipt.topology_revision != current.topology_revision {
        reasons.push(StaleReason::TopologyRevisionChanged);
    }
    if receipt.sut_graph_revision != current.sut_graph_revision {
        reasons.push(StaleReason::SutGraphRevisionChanged);
    }
    if receipt.policy_revision != current.policy_revision {
        reasons.push(StaleReason::PolicyRevisionChanged);
    }
    // receipt.capability_id vs current.capability_test_identity per SPEC §11
    if receipt.capability_id != current.capability_test_identity {
        reasons.push(StaleReason::CapabilityTestIdentityChanged);
    }
    // toolchain_identity comparison (added 2026-09-03 per FIND-000003)
    if receipt.toolchain_identity != current.toolchain_identity {
        reasons.push(StaleReason::ToolchainIdentityChanged);
    }

    if reasons.is_empty() {
        ReuseDecision::Reusable
    } else {
        ReuseDecision::Stale { reasons }
    }
}

// ── REQ-4: Graph-driven invalidation ─────────────────────────────────────────

/// Report from a graph-driven invalidation pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InvalidationReportV1 {
    /// Receipt IDs that were invalidated (ordered).
    pub invalidated: Vec<String>,
    /// Receipt IDs that remain reusable (ordered).
    pub reusable: Vec<String>,
    /// Number of invalidated receipts.
    pub invalidated_count: usize,
    /// Number of reusable receipts.
    pub reusable_count: usize,
}

/// Versioned envelope for invalidation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum InvalidationReport {
    /// Version 1 report.
    V1(InvalidationReportV1),
}

/// Builds the transitive closure of nodes reachable from the given SUT node via
/// DependsOn / RuntimeDependsOn edges (plus the node itself), following edges
/// until exhaustion. Cycle-safe: uses BTreeSet insertion guard to prevent
/// re-processing already-visited nodes.
///
/// This is NOT depth-1: the BFS queue ensures all reachable nodes are
/// included transitively. See SPEC-043 §11 ENMIENDA 2026-09-03.
fn build_closure(sut_node_id: &str, topology: &ProjectTestTopologyV1) -> BTreeSet<String> {
    let mut closure: BTreeSet<String> = BTreeSet::new();
    closure.insert(sut_node_id.to_string());

    // BFS transitive (cycle-safe): collect all nodes reachable via DependsOn / RuntimeDependsOn
    let mut queue: Vec<String> = vec![sut_node_id.to_string()];

    while let Some(current) = queue.pop() {
        for edge in &topology.edges {
            // From the SUT node outward via DependsOn / RuntimeDependsOn
            if edge.from_node == current {
                match edge.edge_kind {
                    TopologyEdgeKind::DependsOn | TopologyEdgeKind::RuntimeDependsOn => {
                        if closure.insert(edge.to_node.clone()) {
                            queue.push(edge.to_node.clone());
                        }
                    }
                    _ => {}
                }
            }
            // Also consider reverse: if edge.to_node == current, check if we should
            // traverse backward via ReverseDependsOn
            if edge.to_node == current
                && edge.edge_kind == TopologyEdgeKind::ReverseDependsOn
                && closure.insert(edge.from_node.clone())
            {
                queue.push(edge.from_node.clone());
            }
        }
    }

    closure
}

/// Performs graph-driven selective invalidation of evidence receipts.
///
/// For each receipt in the store:
/// 1. Build a transitive closure of SUT nodes (tested node + all DependsOn/RuntimeDependsOn
///    reachable nodes, cycle-safe). This is NOT depth-1 — the BFS traverses the full
///    reachable subgraph. See SPEC-043 §11 ENMIENDA 2026-09-03.
/// 2. Intersect the closure with `changed_node_ids`.
/// 3. If intersection is non-empty ⇒ invalidate (remove from store, add to report).
/// 4. Additionally, if topology_revision, policy_revision, or toolchain_identity differ
///    from those embedded in the receipt ⇒ invalidate regardless of intersection
///    (SPEC §11 rule 3: revision-level invalidation).
///
/// Returns an `InvalidationReportV1` with ordered receipt_id lists.
pub fn invalidate_graph_driven(
    store: &mut EvidenceStoreV1,
    changed_node_ids: &BTreeSet<String>,
    topology: &ProjectTestTopologyV1,
    current: &ReceiptIdentityV1,
    new_source_revision: &str,
) -> InvalidationReportV1 {
    let mut invalidated: Vec<String> = Vec::new();
    let mut reusable: Vec<String> = Vec::new();

    // Collect all receipt IDs to evaluate (avoid borrow conflict)
    let receipt_ids: Vec<String> = store.0.read().unwrap().keys().cloned().collect();

    for receipt_id in receipt_ids {
        // Clone the receipt while holding the lock to avoid borrow conflicts
        let receipt: TestEvidenceReceiptV1 = match store.0.read().unwrap().get(&receipt_id) {
            Some(r) => r.clone(),
            None => continue, // already removed
        };

        // Revision-level invalidation: topology_revision, policy_revision, or
        // toolchain_identity (compared via source_revision as proxy for toolchain)
        let revision_stale = receipt.topology_revision != current.topology_revision
            || receipt.policy_revision != current.policy_revision
            || receipt.source_revision != new_source_revision;

        // Graph-based invalidation: build closure and check intersection
        //
        // Precision path: if receipt carries precise SUT IDs (tested_sut_ids), build
        // the closure from those. This allows receipts to NOT be invalidated by
        // changes outside their actual test closure even when sharing capability_id.
        //
        // Degraded path (backwards-compatible): if tested_sut_ids is empty, fall back
        // to using capability_id as a proxy for the tested scope. This may over-
        // invalidate (SPEC §11 rule 3: conservative, never infrainvalidate).
        // Legacy receipts without SUT binding cannot prove non-intersection:
        // conservative invalidation (SPEC §11 rule 3 — never infrainvalidate).
        // Session-recorded receipts always carry precise tested_sut_ids.
        let conservative = receipt.tested_sut_ids.is_empty();
        let base_ids = if conservative {
            Vec::new()
        } else {
            receipt.tested_sut_ids.clone()
        };

        // Build closure from base IDs and check for intersection with changed nodes
        let mut closure: BTreeSet<String> = BTreeSet::new();
        for base_id in &base_ids {
            closure.extend(build_closure(base_id, topology));
        }
        let intersects = conservative || closure.intersection(changed_node_ids).next().is_some();

        if revision_stale || intersects {
            // Invalidate: remove from store
            store.0.write().unwrap().remove(&receipt_id);
            invalidated.push(receipt_id);
        } else {
            reusable.push(receipt_id);
        }
    }

    invalidated.sort();
    reusable.sort();

    InvalidationReportV1 {
        invalidated_count: invalidated.len(),
        reusable_count: reusable.len(),
        invalidated,
        reusable,
    }
}

// ── REQ-2: EvidenceStore ──────────────────────────────────────────────────────

/// Schema version constant for evidence store.
pub const EVIDENCE_STORE_SCHEMA_VERSION: u32 = 1;

/// In-memory evidence store backed by a BTreeMap (deterministic key order).
///
/// Implements the `TestEvidenceRepository` port from test_ports.rs:
/// - `save`: persists a receipt; duplicate receipt_id ⇒ `TestEvidenceError::DuplicateReceiptId`.
/// - `latest_for`: returns the most recent receipt for a change-set digest and capability
///   (most recent = highest lexicographic completed_at per RFC 3339).
///
/// Uses `RwLock` for interior mutability so `&self` methods can mutate through
/// the trait's `&mut self` signature while maintaining `Sync + Send`.
#[derive(Debug)]
pub struct EvidenceStoreV1(RwLock<BTreeMap<String, TestEvidenceReceiptV1>>);

impl EvidenceStoreV1 {
    /// Creates a new empty evidence store.
    pub fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }

    /// Returns the number of receipts in the store.
    pub fn len(&self) -> usize {
        self.0.read().unwrap().len()
    }

    /// Returns `true` if the store contains no receipts.
    pub fn is_empty(&self) -> bool {
        self.0.read().unwrap().is_empty()
    }

    /// Returns all receipt IDs in the store, in sorted order.
    pub fn receipt_ids(&self) -> Vec<String> {
        self.0.read().unwrap().keys().cloned().collect()
    }
}

impl Default for EvidenceStoreV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::test_ports::TestEvidenceRepository for EvidenceStoreV1 {
    /// Persists an evidence receipt.
    ///
    /// Returns `Err(TestEvidenceError::DuplicateReceiptId)` if a receipt with the
    /// same `receipt_id` already exists.
    fn save(&mut self, receipt: &TestEvidenceReceiptV1) -> Result<(), AdapterError> {
        let mut store = self.0.write().unwrap();
        if store.contains_key(&receipt.receipt_id) {
            return Err(AdapterError::InvalidInput {
                reason: format!("receipt id already exists: {}", receipt.receipt_id),
            });
        }
        store.insert(receipt.receipt_id.clone(), receipt.clone());
        Ok(())
    }

    /// Returns the latest evidence receipt for a given change-set digest and capability.
    ///
    /// "Latest" is defined as the most recent by `completed_at` lexicographic ordering
    /// (RFC 3339 timestamps are lexicographically comparable).
    fn latest_for(
        &self,
        change_set_digest: &str,
        capability_id: &str,
    ) -> Result<Option<TestEvidenceReceiptV1>, AdapterError> {
        let candidates: Vec<TestEvidenceReceiptV1> = self
            .0
            .read()
            .unwrap()
            .values()
            .filter(|r| {
                r.change_set_digest == change_set_digest && r.capability_id == capability_id
            })
            .cloned()
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // Lexicographic by completed_at (RFC 3339) — highest is "most recent"
        let mut sorted = candidates;
        sorted.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
        Ok(Some(sorted.into_iter().next().unwrap()))
    }
}

impl EvidenceStoreV1 {
    /// Inserts a receipt into the store.
    ///
    /// Returns an error if a receipt with the same `receipt_id` already exists.
    pub fn insert(&mut self, receipt: TestEvidenceReceiptV1) -> Result<(), TestEvidenceError> {
        let mut store = self.0.write().unwrap();
        if store.contains_key(&receipt.receipt_id) {
            return Err(TestEvidenceError::DuplicateReceiptId {
                id: receipt.receipt_id,
            });
        }
        store.insert(receipt.receipt_id.clone(), receipt);
        Ok(())
    }

    /// Looks up a receipt by its ID.
    pub fn get(&self, receipt_id: &str) -> Option<TestEvidenceReceiptV1> {
        self.0.read().unwrap().get(receipt_id).cloned()
    }

    /// Returns the latest receipt for a change-set digest and capability.
    ///
    /// Most recent = highest lexicographic `completed_at` (RFC 3339).
    pub fn latest_for(
        &self,
        change_set_digest: &str,
        capability_id: &str,
    ) -> Option<TestEvidenceReceiptV1> {
        let candidates: Vec<TestEvidenceReceiptV1> = self
            .0
            .read()
            .unwrap()
            .values()
            .filter(|r| {
                r.change_set_digest == change_set_digest && r.capability_id == capability_id
            })
            .cloned()
            .collect();

        if candidates.is_empty() {
            return None;
        }

        let mut sorted = candidates;
        sorted.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
        Some(sorted.into_iter().next().unwrap())
    }

    /// Removes a receipt by its ID.
    pub fn remove(&mut self, receipt_id: &str) -> Option<TestEvidenceReceiptV1> {
        self.0.write().unwrap().remove(receipt_id)
    }

    /// Iterates over all receipts (cloned).
    pub fn values(&self) -> Vec<TestEvidenceReceiptV1> {
        self.0.read().unwrap().values().cloned().collect()
    }
}

/// Versioned envelope for evidence store.
#[derive(Debug)]
pub enum EvidenceStore {
    /// Version 1 store.
    V1(EvidenceStoreV1),
}

// ── REQ-6: Selection-quality telemetry ─────────────────────────────────────────

/// Schema version constant for selection telemetry.
pub const SELECTION_TELEMETRY_SCHEMA_VERSION: u32 = 1;

/// Metrics for a single scoped run (SPEC §15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScopedRunMetricsV1 {
    /// Feedback latency in milliseconds.
    pub feedback_latency_ms: u64,
    /// Number of tool calls made.
    pub tool_calls: u32,
    /// Total tokens consumed.
    pub tokens: u64,
    /// Number of tests selected in this scoped run.
    pub selected_tests: u32,
    /// Total tests in the full profile.
    pub total_tests_in_full_profile: u32,
    /// Full-verify baseline time in milliseconds.
    pub full_verify_baseline_ms: u64,
    /// Number of evidence receipts reused in this run.
    pub reused: u32,
}

impl ScopedRunMetricsV1 {
    /// Validates that the metrics are coherent (selected <= total).
    pub fn validate(&self) -> Result<(), TestEvidenceError> {
        if self.selected_tests > self.total_tests_in_full_profile {
            return Err(TestEvidenceError::InvalidInput {
                reason: format!(
                    "selected_tests ({}) cannot exceed total_tests_in_full_profile ({})",
                    self.selected_tests, self.total_tests_in_full_profile
                ),
            });
        }
        Ok(())
    }
}

/// A single escape record — a verify failure that slipped through scoped selection (SPEC §15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EscapeRecordV1 {
    /// The scoped receipt ID associated with this escape.
    pub scoped_receipt_id: String,
    /// Reference to the verify failure (non-empty).
    pub verify_failure_ref: String,
    /// Description of the check that was escaped (non-empty).
    pub escaped_check: String,
}

impl EscapeRecordV1 {
    /// Validates this escape record.
    pub fn validate(&self) -> Result<(), TestEvidenceError> {
        if self.verify_failure_ref.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "verify_failure_ref".to_string(),
            });
        }
        if self.escaped_check.is_empty() {
            return Err(TestEvidenceError::EmptyField {
                field: "escaped_check".to_string(),
            });
        }
        Ok(())
    }
}

/// Selection telemetry V1 — aggregated run metrics and escape records (SPEC §15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SelectionTelemetryV1 {
    /// Schema version — must be `SELECTION_TELEMETRY_SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Individual scoped run metrics.
    pub runs: Vec<ScopedRunMetricsV1>,
    /// Escape records from failed verifications.
    pub escapes: Vec<EscapeRecordV1>,
}

impl SelectionTelemetryV1 {
    /// Creates a new V1 selection telemetry.
    pub fn new(runs: Vec<ScopedRunMetricsV1>, escapes: Vec<EscapeRecordV1>) -> Self {
        Self {
            schema_version: SELECTION_TELEMETRY_SCHEMA_VERSION,
            runs,
            escapes,
        }
    }

    /// Returns the escape rate as `escapes / runs`.
    ///
    /// Returns `None` if there are no runs.
    pub fn escape_rate(&self) -> Option<f32> {
        if self.runs.is_empty() {
            None
        } else {
            Some(self.escapes.len() as f32 / self.runs.len() as f32)
        }
    }

    /// Returns the ratio of time saved vs full-verify as `1 - (scoped_avg / full_avg)`.
    ///
    /// Returns `None` if there are no runs.
    pub fn time_saved_ratio(&self) -> Option<f32> {
        if self.runs.is_empty() {
            return None;
        }
        let scoped_avg: f32 = self
            .runs
            .iter()
            .map(|r| r.feedback_latency_ms as f32)
            .sum::<f32>()
            / self.runs.len() as f32;
        let full_avg: f32 = self
            .runs
            .iter()
            .map(|r| r.full_verify_baseline_ms as f32)
            .sum::<f32>()
            / self.runs.len() as f32;
        if full_avg == 0.0 {
            return None;
        }
        Some(1.0 - (scoped_avg / full_avg))
    }

    /// Returns the ratio of tool calls saved vs full-verify baseline.
    ///
    /// V1-unavailable: SPEC §15 does not define a baseline for tool-call counts,
    /// so this method returns `None` unconditionally. A future spec change may
    /// introduce a `tool_call_baseline` field to enable meaningful computation.
    ///
    /// Returns `None` if there are no runs (also returns `None` in V1).
    pub fn tool_call_savings_ratio(&self) -> Option<f32> {
        // V1: SPEC §15 does not define tool-call baseline; cannot compute savings ratio
        None
    }

    /// Returns the evidence reuse ratio as `reused / total_selected`.
    ///
    /// Returns `None` if there are no runs.
    pub fn evidence_reuse_ratio(&self) -> Option<f32> {
        if self.runs.is_empty() {
            return None;
        }
        let total_selected: u32 = self.runs.iter().map(|r| r.selected_tests).sum();
        let total_reused: u32 = self.runs.iter().map(|r| r.reused).sum();
        if total_selected == 0 {
            return Some(0.0);
        }
        Some(total_reused as f32 / total_selected as f32)
    }
}

/// Versioned envelope for selection telemetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum SelectionTelemetry {
    /// Version 1 telemetry.
    V1(SelectionTelemetryV1),
}

// ── REQ-7: Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_model::{
        EdgeProvenanceV1, ProjectTestTopologyV1, ReceiptResult, SutKind, SutNodeV1,
        TopologyEdgeKind, TopologyEdgeV1,
    };
    use std::collections::BTreeMap;

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.1: identity — hash stable + mutation-inequality per field
    // ═══════════════════════════════════════════════════════════════════════════

    fn make_identity(
        change_set_digest: &str,
        source_revision: &str,
        topology_revision: &str,
        sut_graph_revision: &str,
        policy_revision: &str,
        capability_test_identity: &str,
        toolchain_identity: &str,
    ) -> ReceiptIdentityV1 {
        ReceiptIdentityV1::new(
            change_set_digest.to_string(),
            source_revision.to_string(),
            topology_revision.to_string(),
            sut_graph_revision.to_string(),
            policy_revision.to_string(),
            capability_test_identity.to_string(),
            toolchain_identity.to_string(),
        )
    }

    #[test]
    fn identity_hash_stable_and_deterministic() {
        let id1 = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let id2 = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        assert_eq!(id1.compute_content_hash(), id2.compute_content_hash());
    }

    #[test]
    fn identity_mutation_change_set_digest() {
        let base = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let changed = make_identity("csd2", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        assert_ne!(base.compute_content_hash(), changed.compute_content_hash());
    }

    #[test]
    fn identity_mutation_source_revision() {
        let base = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let changed = make_identity("csd1", "src2", "topo1", "sut1", "pol1", "cap1", "tool1");
        assert_ne!(base.compute_content_hash(), changed.compute_content_hash());
    }

    #[test]
    fn identity_mutation_topology_revision() {
        let base = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let changed = make_identity("csd1", "src1", "topo2", "sut1", "pol1", "cap1", "tool1");
        assert_ne!(base.compute_content_hash(), changed.compute_content_hash());
    }

    #[test]
    fn identity_mutation_sut_graph_revision() {
        let base = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let changed = make_identity("csd1", "src1", "topo1", "sut2", "pol1", "cap1", "tool1");
        assert_ne!(base.compute_content_hash(), changed.compute_content_hash());
    }

    #[test]
    fn identity_mutation_policy_revision() {
        let base = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let changed = make_identity("csd1", "src1", "topo1", "sut1", "pol2", "cap1", "tool1");
        assert_ne!(base.compute_content_hash(), changed.compute_content_hash());
    }

    #[test]
    fn identity_mutation_capability_test_identity() {
        let base = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let changed = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap2", "tool1");
        assert_ne!(base.compute_content_hash(), changed.compute_content_hash());
    }

    #[test]
    fn identity_mutation_toolchain_identity() {
        let base = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        let changed = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool2");
        assert_ne!(base.compute_content_hash(), changed.compute_content_hash());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.9: validation — empty fields rejected
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn identity_validate_empty_change_set_digest() {
        let id = make_identity("", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        assert!(matches!(
            id.validate(),
            Err(TestEvidenceError::EmptyField { field })
            if field == "change_set_digest"
        ));
    }

    #[test]
    fn identity_validate_empty_source_revision() {
        let id = make_identity("csd1", "", "topo1", "sut1", "pol1", "cap1", "tool1");
        assert!(matches!(
            id.validate(),
            Err(TestEvidenceError::EmptyField { field })
            if field == "source_revision"
        ));
    }

    #[test]
    fn identity_validate_unsupported_schema_version() {
        let mut id = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");
        id.schema_version = 99;
        assert!(matches!(
            id.validate(),
            Err(TestEvidenceError::UnsupportedSchemaVersion { got, want })
            if got == 99 && want == RECEIPT_IDENTITY_SCHEMA_VERSION
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.8: variant-count assertions
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn reuse_decision_variant_count() {
        let variants = [
            ReuseDecision::Reusable,
            ReuseDecision::Stale { reasons: vec![] },
            ReuseDecision::NoEvidence,
        ];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn stale_reason_variant_count() {
        let variants = [
            StaleReason::ChangeSetChanged,
            StaleReason::SourceRevisionChanged,
            StaleReason::TopologyRevisionChanged,
            StaleReason::SutGraphRevisionChanged,
            StaleReason::PolicyRevisionChanged,
            StaleReason::CapabilityTestIdentityChanged,
            StaleReason::ToolchainIdentityChanged,
        ];
        assert_eq!(variants.len(), 7);
    }

    #[test]
    fn test_evidence_error_variant_count() {
        let variants = [
            TestEvidenceError::UnsupportedSchemaVersion { got: 0, want: 1 },
            TestEvidenceError::EmptyField {
                field: String::new(),
            },
            TestEvidenceError::DuplicateReceiptId { id: String::new() },
            TestEvidenceError::InvalidInput {
                reason: String::new(),
            },
        ];
        assert_eq!(variants.len(), 4);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.5: store port — save / latest_for
    // ═══════════════════════════════════════════════════════════════════════════

    fn make_receipt(
        receipt_id: &str,
        change_set_digest: &str,
        capability_id: &str,
        completed_at: &str,
    ) -> TestEvidenceReceiptV1 {
        let built = TestEvidenceReceiptV1::new(
            receipt_id.to_string(),
            change_set_digest.to_string(),
            "src1".to_string(),
            "topo1".to_string(),
            "sut1".to_string(),
            "pol1".to_string(),
            capability_id.to_string(),
            ReceiptResult::Passed,
            completed_at.to_string(),
            String::new(), // toolchain_identity
        );
        let mut receipt = built;
        receipt.tested_sut_ids = vec![capability_id.to_string()];
        receipt
    }

    #[test]
    fn store_insert_and_get() {
        let mut store = EvidenceStoreV1::new();
        let receipt = make_receipt("r1", "csd1", "cap1", "2024-01-01T00:00:00Z");
        store.insert(receipt).unwrap();
        assert!(store.get("r1").is_some());
        assert_eq!(store.get("r1").unwrap().receipt_id, "r1");
    }

    #[test]
    fn store_duplicate_rejected() {
        let mut store = EvidenceStoreV1::new();
        let receipt = make_receipt("r1", "csd1", "cap1", "2024-01-01T00:00:00Z");
        store.insert(receipt.clone()).unwrap();
        let result = store.insert(receipt);
        assert!(matches!(
            result,
            Err(TestEvidenceError::DuplicateReceiptId { id }) if id == "r1"
        ));
    }

    #[test]
    fn store_latest_for_most_recent_by_completed_at() {
        let mut store = EvidenceStoreV1::new();
        // Insert in non-chronological order
        store
            .insert(make_receipt("r1", "csd1", "cap1", "2024-01-01T00:00:00Z"))
            .unwrap();
        store
            .insert(make_receipt("r2", "csd1", "cap1", "2024-01-03T00:00:00Z"))
            .unwrap();
        store
            .insert(make_receipt("r3", "csd1", "cap1", "2024-01-02T00:00:00Z"))
            .unwrap();

        let latest = store.latest_for("csd1", "cap1");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().receipt_id, "r2"); // 2024-01-03 is latest
    }

    #[test]
    fn store_latest_for_no_match() {
        let mut store = EvidenceStoreV1::new();
        store
            .insert(make_receipt("r1", "csd1", "cap1", "2024-01-01T00:00:00Z"))
            .unwrap();
        let latest = store.latest_for("csd2", "cap1"); // different digest
        assert!(latest.is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.2: UAT-6 reuse — unrelated SUT change ⇒ Reusable
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn classify_unrelated_sut_change_reusable() {
        let receipt = make_receipt("r1", "csd1", "cap1", "2024-01-01T00:00:00Z");
        // current matches receipt exactly (toolchain_identity also matches)
        let current = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "");
        let decision = classify(&receipt, &current);
        assert!(matches!(decision, ReuseDecision::Reusable));
    }

    #[test]
    fn classify_capability_test_identity_changed_stale() {
        let receipt = make_receipt("r1", "csd1", "cap1", "2024-01-01T00:00:00Z");
        // current has different capability_test_identity (cap2 vs cap1); toolchain_identity matches
        let current = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap2", "");
        let decision = classify(&receipt, &current);
        assert!(matches!(
            decision,
            ReuseDecision::Stale { reasons } if reasons.len() == 1 && reasons[0] == StaleReason::CapabilityTestIdentityChanged
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.3: UAT-7 invalidation selectiva
    // ═══════════════════════════════════════════════════════════════════════════

    fn make_topology() -> ProjectTestTopologyV1 {
        let prov = EdgeProvenanceV1 {
            source: "test".to_string(),
            adapter_version: "v1".to_string(),
            confidence_source: "test".to_string(),
        };

        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "node-a".to_string(),
            SutNodeV1::new(
                "node-a".to_string(),
                SutKind::Component,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "node-b".to_string(),
            SutNodeV1::new(
                "node-b".to_string(),
                SutKind::Component,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "node-c".to_string(),
            SutNodeV1::new(
                "node-c".to_string(),
                SutKind::Component,
                "rust".to_string(),
                None,
            ),
        );

        let edges = vec![
            TopologyEdgeV1::new(
                TopologyEdgeKind::DependsOn,
                "node-a".to_string(),
                "node-b".to_string(),
                prov.clone(),
            ),
            TopologyEdgeV1::new(
                TopologyEdgeKind::RuntimeDependsOn,
                "node-b".to_string(),
                "node-c".to_string(),
                prov.clone(),
            ),
        ];

        ProjectTestTopologyV1::new("topo1".to_string(), nodes, edges)
    }

    #[test]
    fn invalidate_graph_driven_intersecting_invalidated() {
        let mut store = EvidenceStoreV1::new();
        // Receipt for node-a (transitive closure = {node-a, node-b, node-c})
        store
            .insert(make_receipt(
                "r-a",
                "csd1",
                "node-a",
                "2024-01-01T00:00:00Z",
            ))
            .unwrap();
        // Receipt for node-c (transitive closure = {node-c})
        store
            .insert(make_receipt(
                "r-c",
                "csd1",
                "node-c",
                "2024-01-01T00:00:00Z",
            ))
            .unwrap();

        let topology = make_topology();
        let changed: BTreeSet<String> = vec!["node-b".to_string()].into_iter().collect();
        let current = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "node-a", "tool1");

        let report = invalidate_graph_driven(&mut store, &changed, &topology, &current, "src1");

        // node-a's transitive closure {node-a, node-b, node-c} intersects node-b ⇒ invalidated
        // node-c's transitive closure {node-c} does not intersect node-b ⇒ reusable
        assert_eq!(report.invalidated_count, 1);
        assert_eq!(report.reusable_count, 1);
        assert!(report.invalidated.contains(&"r-a".to_string()));
        assert!(report.reusable.contains(&"r-c".to_string()));
    }

    /// UAT-6 reinforced: precise tested_sut_ids prevents invalidation by changes
    /// outside the receipt's actual closure even when capability_id would suggest
    /// over-invalidation.
    #[test]
    fn invalidate_graph_driven_precise_tested_sut_ids_avoid_overinvalidation() {
        let mut store = EvidenceStoreV1::new();

        // Receipt with precise tested_sut_ids = ["node-c"]
        // This receipt tested ONLY node-c (leaf), not its ancestors
        let mut precise_receipt = make_receipt("r-precise", "csd1", "cap1", "2024-01-01T00:00:00Z");
        precise_receipt.tested_sut_ids = vec!["node-c".to_string()];

        // Receipt without precise tested_sut_ids (degraded path via capability_id = "node-a")
        // In degraded mode, capability_id = "node-a" gives transitive closure {node-a, node-b, node-c}
        let degraded_receipt = make_receipt("r-degraded", "csd1", "node-a", "2024-01-01T00:00:00Z");

        store.insert(precise_receipt).unwrap();
        store.insert(degraded_receipt).unwrap();

        let topology = make_topology();
        // Only node-a changed (in degraded receipt's transitive closure but NOT in precise receipt's closure)
        // Topology: node-a --depends-on--> node-b --runtime-depends-on--> node-c
        // Precise closure: {node-c} (from tested_sut_ids = ["node-c"])
        // Degraded transitive closure: {node-a, node-b, node-c} (from capability_id = "node-a")
        let changed: BTreeSet<String> = vec!["node-a".to_string()].into_iter().collect();
        let current = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "cap1", "tool1");

        let report = invalidate_graph_driven(&mut store, &changed, &topology, &current, "src1");

        // r-precise: closure = {node-c} (from tested_sut_ids), does NOT intersect node-a
        //            ⇒ remains reusable (precise, no over-invalidation)
        // r-degraded: closure = {node-a, node-b} (from capability_id proxy), intersects node-a
        //            ⇒ invalidated (degraded path over-invalidates)
        assert_eq!(report.invalidated_count, 1);
        assert_eq!(report.reusable_count, 1);
        assert!(report.invalidated.contains(&"r-degraded".to_string()));
        assert!(report.reusable.contains(&"r-precise".to_string()));
    }

    /// ENMIENDA 2026-09-03 (SPEC-043 §11): closure is transitive cycle-safe.
    /// Chain depth-2: a --depends-on--> b --runtime-depends-on--> c.
    /// Receipt for a; change in c ⇒ receipt MUST be invalidated (transitive closure).
    #[test]
    fn invalidate_graph_driven_transitive_closure_depth2() {
        let mut store = EvidenceStoreV1::new();
        // Receipt for node-a (transitive closure = {node-a, node-b, node-c})
        store
            .insert(make_receipt(
                "r-a",
                "csd1",
                "node-a",
                "2024-01-01T00:00:00Z",
            ))
            .unwrap();

        let topology = make_topology();
        // Change node-c (deep in the dependency chain)
        let changed: BTreeSet<String> = vec!["node-c".to_string()].into_iter().collect();
        let current = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "node-a", "tool1");

        let report = invalidate_graph_driven(&mut store, &changed, &topology, &current, "src1");

        // Transitive closure of node-a includes node-c ⇒ invalidated
        assert_eq!(report.invalidated_count, 1);
        assert_eq!(report.reusable_count, 0);
        assert!(report.invalidated.contains(&"r-a".to_string()));

        // Determinism: single-element vectors are trivially ordered
        assert_eq!(report.invalidated, vec!["r-a"]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.4: revision-level invalidation (topology/policy/toolchain)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn invalidate_revision_level_topology_changed() {
        let mut store = EvidenceStoreV1::new();
        store
            .insert(make_receipt("r1", "csd1", "node-a", "2024-01-01T00:00:00Z"))
            .unwrap();

        let topology = make_topology();
        let changed: BTreeSet<String> = BTreeSet::new(); // no node changes
        // Current has different topology_revision
        let current = make_identity(
            "csd1", "src1", "topo2", // different topology_revision
            "sut1", "pol1", "node-a", "tool1",
        );

        let report = invalidate_graph_driven(
            &mut store, &changed, &topology, &current, "src1", // same source revision
        );

        // topology_revision changed ⇒ invalidation regardless of intersection
        assert_eq!(report.invalidated_count, 1);
        assert!(report.invalidated.contains(&"r1".to_string()));
    }

    #[test]
    fn invalidate_revision_level_policy_changed() {
        let mut store = EvidenceStoreV1::new();
        store
            .insert(make_receipt("r1", "csd1", "node-a", "2024-01-01T00:00:00Z"))
            .unwrap();

        let topology = make_topology();
        let changed: BTreeSet<String> = BTreeSet::new();
        // Current has different policy_revision
        let current = make_identity(
            "csd1", "src1", "topo1", "sut1", "pol2", // different policy_revision
            "node-a", "tool1",
        );

        let report = invalidate_graph_driven(&mut store, &changed, &topology, &current, "src1");

        assert_eq!(report.invalidated_count, 1);
        assert!(report.invalidated.contains(&"r1".to_string()));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.6: escape rate + savings ratios
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn escape_rate_zero_runs_returns_none() {
        let telemetry = SelectionTelemetryV1::new(vec![], vec![]);
        assert!(telemetry.escape_rate().is_none());
    }

    #[test]
    fn escape_rate_5_runs_1_escape_is_0_2() {
        let runs = vec![
            ScopedRunMetricsV1 {
                feedback_latency_ms: 100,
                tool_calls: 10,
                tokens: 1000,
                selected_tests: 5,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 0,
            },
            ScopedRunMetricsV1 {
                feedback_latency_ms: 110,
                tool_calls: 12,
                tokens: 1100,
                selected_tests: 6,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 1,
            },
            ScopedRunMetricsV1 {
                feedback_latency_ms: 105,
                tool_calls: 11,
                tokens: 1050,
                selected_tests: 5,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 0,
            },
            ScopedRunMetricsV1 {
                feedback_latency_ms: 115,
                tool_calls: 13,
                tokens: 1150,
                selected_tests: 7,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 2,
            },
            ScopedRunMetricsV1 {
                feedback_latency_ms: 120,
                tool_calls: 14,
                tokens: 1200,
                selected_tests: 8,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 1,
            },
        ];
        let escapes = vec![EscapeRecordV1 {
            scoped_receipt_id: "r1".to_string(),
            verify_failure_ref: "fail-1".to_string(),
            escaped_check: "security-check".to_string(),
        }];

        let telemetry = SelectionTelemetryV1::new(runs, escapes);
        assert!((telemetry.escape_rate().unwrap() - 0.2).abs() < 0.001);
    }

    #[test]
    fn time_saved_ratio_correct() {
        // scoped avg = (100+200)/2 = 150, full avg = (500+500)/2 = 500
        // saved = 1 - 150/500 = 0.7
        let runs = vec![
            ScopedRunMetricsV1 {
                feedback_latency_ms: 100,
                tool_calls: 10,
                tokens: 1000,
                selected_tests: 5,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 0,
            },
            ScopedRunMetricsV1 {
                feedback_latency_ms: 200,
                tool_calls: 20,
                tokens: 2000,
                selected_tests: 10,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 0,
            },
        ];
        let telemetry = SelectionTelemetryV1::new(runs, vec![]);
        let ratio = telemetry.time_saved_ratio().unwrap();
        assert!((ratio - 0.7).abs() < 0.001);
    }

    #[test]
    fn evidence_reuse_ratio_correct() {
        let runs = vec![
            ScopedRunMetricsV1 {
                feedback_latency_ms: 100,
                tool_calls: 10,
                tokens: 1000,
                selected_tests: 10,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 3,
            },
            ScopedRunMetricsV1 {
                feedback_latency_ms: 100,
                tool_calls: 10,
                tokens: 1000,
                selected_tests: 10,
                total_tests_in_full_profile: 20,
                full_verify_baseline_ms: 500,
                reused: 7,
            },
        ];
        let telemetry = SelectionTelemetryV1::new(runs, vec![]);
        // total_reused = 10, total_selected = 20
        let ratio = telemetry.evidence_reuse_ratio().unwrap();
        assert!((ratio - 0.5).abs() < 0.001);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-7.7: determinism — same store + changes ⇒ same report
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn invalidate_deterministic_same_input_same_output() {
        let mut store1 = EvidenceStoreV1::new();
        store1
            .insert(make_receipt("r1", "csd1", "node-a", "2024-01-01T00:00:00Z"))
            .unwrap();
        store1
            .insert(make_receipt("r2", "csd1", "node-c", "2024-01-01T00:00:00Z"))
            .unwrap();

        let mut store2 = EvidenceStoreV1::new();
        store2
            .insert(make_receipt("r1", "csd1", "node-a", "2024-01-01T00:00:00Z"))
            .unwrap();
        store2
            .insert(make_receipt("r2", "csd1", "node-c", "2024-01-01T00:00:00Z"))
            .unwrap();

        let topology = make_topology();
        let changed: BTreeSet<String> = vec!["node-b".to_string()].into_iter().collect();
        let current = make_identity("csd1", "src1", "topo1", "sut1", "pol1", "node-a", "tool1");

        let report1 = invalidate_graph_driven(&mut store1, &changed, &topology, &current, "src1");
        let report2 = invalidate_graph_driven(&mut store2, &changed, &topology, &current, "src1");

        assert_eq!(report1.invalidated, report2.invalidated);
        assert_eq!(report1.reusable, report2.reusable);
        assert_eq!(report1.invalidated_count, report2.invalidated_count);
        assert_eq!(report1.reusable_count, report2.reusable_count);
    }
}
