//! Neutral verification contracts — versioned, migration-safe domain model for test selection.
//!
//! Per ADR-043 and SPEC-043 §3: language-neutral model with closed enums,
//! deterministic digests, and typed fail-closed `MappingOutcome` results.
//!
//! # Design
//!
//! - 8 versioned aggregates with `{ V1(XxxV1) }` envelope + `SCHEMA_VERSION = 1`.
//! - 7 closed enums with `assert_variant_count_eq!` guards (ChangeKind, SutKind,
//!   TopologyEdgeKind, CapabilityKind, SelectorGranularity, ImpactReason, PlanVerdict).
//! - Deterministic serialization: canonical JSON + `sha256:<64-hex-lowercase>` content hash.
//! - Fail-closed `InsufficientMapping` for unknown-impact cases.
//! - NO build-system semantics in types (ecosystem is free String; adapters use TEST-ADAPTER-*).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Schema version constant for all V1 aggregates in this module.
pub const SCHEMA_VERSION: u32 = 1;

/// Content hash in `sha256:<64-hex-lowercase>` format (mirrors WorkflowIR/ExecutionScope).
pub type ContentHash = String;

// ── Closed enums ───────────────────────────────────────────────────────────────

/// Kind of change applied to an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Artifact was added.
    Added,
    /// Artifact was modified.
    Modified,
    /// Artifact was deleted.
    Deleted,
    /// Artifact was renamed.
    Renamed,
}

crate::assert_variant_count_eq!(
    ChangeKind,
    4,
    [
        ChangeKind::Added,
        ChangeKind::Modified,
        ChangeKind::Deleted,
        ChangeKind::Renamed,
    ]
);

/// Kind of system-under-test node in a project topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SutKind {
    /// A source repository.
    Repository,
    /// A workspace or monorepo member.
    Workspace,
    /// A component or module within a workspace.
    Component,
    /// A buildable unit (package, crate, library).
    BuildUnit,
    /// A source artifact (file, source entry).
    SourceArtifact,
    /// A module, namespace, or package scope.
    ModuleOrNamespace,
    /// A named symbol (function, type, constant).
    Symbol,
    /// A runtime service or background process.
    RuntimeService,
    /// A contract boundary (interface, API, protocol).
    ContractBoundary,
    /// A schema definition (type, interface, IDL).
    Schema,
    /// A configuration surface (config files, env vars).
    ConfigurationSurface,
    /// A generated artifact (output, build artifact).
    GeneratedArtifact,
    /// A single test unit.
    TestUnit,
    /// A test suite or test group.
    TestSuite,
    /// A verification capability descriptor.
    VerificationCapability,
    /// An evidence receipt.
    EvidenceReceipt,
}

crate::assert_variant_count_eq!(
    SutKind,
    16,
    [
        SutKind::Repository,
        SutKind::Workspace,
        SutKind::Component,
        SutKind::BuildUnit,
        SutKind::SourceArtifact,
        SutKind::ModuleOrNamespace,
        SutKind::Symbol,
        SutKind::RuntimeService,
        SutKind::ContractBoundary,
        SutKind::Schema,
        SutKind::ConfigurationSurface,
        SutKind::GeneratedArtifact,
        SutKind::TestUnit,
        SutKind::TestSuite,
        SutKind::VerificationCapability,
        SutKind::EvidenceReceipt,
    ]
);

/// Kind of relationship between two SUT nodes in the project topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TopologyEdgeKind {
    /// Node touches (reads/writes) another node.
    Touches,
    /// Node owns or contains another node.
    Owns,
    /// Node builds or compiles another node.
    Builds,
    /// Node depends on another at build time.
    DependsOn,
    /// Node depends on another at runtime.
    RuntimeDependsOn,
    /// Another node depends on this node at build time.
    ReverseDependsOn,
    /// Node generates another artifact.
    Generates,
    /// Node tests another node.
    Tests,
    /// Node covers another node (test coverage).
    Covers,
    /// Node validates a contract boundary.
    ValidatesContract,
    /// Contract dependency between two nodes.
    ContractDependency,
    /// Node uses a capability of another node.
    UsesCapability,
    /// Node produced evidence about another node.
    ProducedEvidence,
    /// Node invalidates another node (cache, output).
    Invalidates,
}

crate::assert_variant_count_eq!(
    TopologyEdgeKind,
    14,
    [
        TopologyEdgeKind::Touches,
        TopologyEdgeKind::Owns,
        TopologyEdgeKind::Builds,
        TopologyEdgeKind::DependsOn,
        TopologyEdgeKind::RuntimeDependsOn,
        TopologyEdgeKind::ReverseDependsOn,
        TopologyEdgeKind::Generates,
        TopologyEdgeKind::Tests,
        TopologyEdgeKind::Covers,
        TopologyEdgeKind::ValidatesContract,
        TopologyEdgeKind::ContractDependency,
        TopologyEdgeKind::UsesCapability,
        TopologyEdgeKind::ProducedEvidence,
        TopologyEdgeKind::Invalidates,
    ]
);

/// Kind of verification or test capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// Compilation check.
    Compile,
    /// Static type checking.
    TypeCheck,
    /// Linting.
    Lint,
    /// Unit test execution.
    Unit,
    /// Integration test execution.
    Integration,
    /// Contract or interface test.
    Contract,
    /// End-to-end test.
    E2e,
    /// Security test or scan.
    Security,
    /// Mutation test.
    Mutation,
    /// Architecture or style check.
    Architecture,
    /// User-acceptance test.
    Uat,
    /// Custom or project-specific capability.
    Custom,
}

crate::assert_variant_count_eq!(
    CapabilityKind,
    12,
    [
        CapabilityKind::Compile,
        CapabilityKind::TypeCheck,
        CapabilityKind::Lint,
        CapabilityKind::Unit,
        CapabilityKind::Integration,
        CapabilityKind::Contract,
        CapabilityKind::E2e,
        CapabilityKind::Security,
        CapabilityKind::Mutation,
        CapabilityKind::Architecture,
        CapabilityKind::Uat,
        CapabilityKind::Custom,
    ]
);

/// Granularity at which test selectors operate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SelectorGranularity {
    /// Select at repository level.
    Repository,
    /// Select at workspace level.
    Workspace,
    /// Select at component level.
    Component,
    /// Select at build-unit level.
    BuildUnit,
    /// Select at file level.
    File,
    /// Select at symbol level.
    Symbol,
    /// Select by test identifier.
    TestId,
    /// Select by tag filter.
    TagFilter,
}

crate::assert_variant_count_eq!(
    SelectorGranularity,
    8,
    [
        SelectorGranularity::Repository,
        SelectorGranularity::Workspace,
        SelectorGranularity::Component,
        SelectorGranularity::BuildUnit,
        SelectorGranularity::File,
        SelectorGranularity::Symbol,
        SelectorGranularity::TestId,
        SelectorGranularity::TagFilter,
    ]
);

/// Reason why a SUT node was impacted by a change (SPEC-043 §3.6, v1 only).
///
/// ObservedCoverage and HistoricalFailureCorrelation are explicitly post-v1;
/// they MUST NOT appear in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ImpactReason {
    /// Direct source-file touch.
    DirectSourceTouch,
    /// Change in a component this node owns.
    ComponentOwnership,
    /// Change in a build unit this node owns.
    BuildUnitOwnership,
    /// Forward build-dependency propagation.
    DependencyPropagation,
    /// Reverse build-dependency propagation.
    ReverseDependencyPropagation,
    /// Runtime dependency propagation.
    RuntimeDependencyPropagation,
    /// Public contract/API change.
    PublicContractChange,
    /// Schema definition change.
    SchemaChange,
    /// Build configuration or workspace change.
    BuildOrWorkspaceChange,
    /// Configuration surface change.
    ConfigurationChange,
    /// Generated surface change.
    GeneratedSurfaceChange,
    /// Explicit test association via annotation or directive.
    ExplicitTestAssociation,
    /// Local unit test coverage.
    LocalUnitTest,
    /// Cross-component integration test coverage.
    ComponentIntegrationTest,
    /// Cross-component contract test coverage.
    CrossComponentContractTest,
}

crate::assert_variant_count_eq!(
    ImpactReason,
    15,
    [
        ImpactReason::DirectSourceTouch,
        ImpactReason::ComponentOwnership,
        ImpactReason::BuildUnitOwnership,
        ImpactReason::DependencyPropagation,
        ImpactReason::ReverseDependencyPropagation,
        ImpactReason::RuntimeDependencyPropagation,
        ImpactReason::PublicContractChange,
        ImpactReason::SchemaChange,
        ImpactReason::BuildOrWorkspaceChange,
        ImpactReason::ConfigurationChange,
        ImpactReason::GeneratedSurfaceChange,
        ImpactReason::ExplicitTestAssociation,
        ImpactReason::LocalUnitTest,
        ImpactReason::ComponentIntegrationTest,
        ImpactReason::CrossComponentContractTest,
    ]
);

/// Overall verdict of a test selection plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PlanVerdict {
    /// Plan is executable — test selection produced actionable batches.
    Executable,
    /// Plan is blocked — prerequisites or capabilities are missing.
    Blocked,
    /// Human verification is required before execution.
    VerifyRequired,
}

crate::assert_variant_count_eq!(
    PlanVerdict,
    3,
    [
        PlanVerdict::Executable,
        PlanVerdict::Blocked,
        PlanVerdict::VerifyRequired,
    ]
);

/// Result of a single test execution receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptResult {
    /// All tests passed.
    Passed,
    /// One or more tests failed.
    Failed,
}

crate::assert_variant_count_eq!(
    ReceiptResult,
    2,
    [ReceiptResult::Passed, ReceiptResult::Failed,]
);

// ── Versioned aggregates ───────────────────────────────────────────────────────

/// Versioned envelope for all aggregates in this module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum TestModel {
    /// Version 1 aggregates.
    V1(TestModelV1),
}

/// Container for all V1 aggregates — mirrors the canonical TestModel projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TestModelV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Active change set tracked by the kernel.
    pub active_change_set: ActiveChangeSetV1,
    /// Project test topology (SUT graph).
    pub project_test_topology: ProjectTestTopologyV1,
    /// Verification capabilities available in this project.
    pub verification_capabilities: Vec<VerificationCapabilityV1>,
    /// Selected test plan (if any) for the current change set.
    pub test_selection_plan: Option<TestSelectionPlanV1>,
    /// Evidence receipts collected for this change set.
    pub evidence_receipts: Vec<TestEvidenceReceiptV1>,
}

impl TestModelV1 {
    /// Validates this model instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        self.active_change_set.validate()?;
        self.project_test_topology.validate()?;
        for cap in &self.verification_capabilities {
            cap.validate()?;
        }
        if let Some(ref plan) = self.test_selection_plan {
            plan.validate()?;
        }
        for receipt in &self.evidence_receipts {
            receipt.validate()?;
        }
        Ok(())
    }
}

// ── ActiveChangeSet ──────────────────────────────────────────────────────────

/// A changed artifact within an active change set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChangedArtifactV1 {
    /// Path to the changed artifact (non-empty).
    pub path: String,
    /// Kind of change applied.
    pub change_kind: ChangeKind,
    /// Whether the change is staged (vs. unstaged working-tree change).
    pub staged: bool,
}

/// Active change set — the kernel's record of what is changing (ADR-043).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActiveChangeSetV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Project identifier (non-empty).
    pub project_id: String,
    /// Work item this change set is associated with (optional).
    pub work_item_id: Option<String>,
    /// Run identifier for this execution (optional).
    pub run_id: Option<String>,
    /// Base revision (commit, tag, or ref) this change set is diffed against (non-empty).
    pub base_revision: String,
    /// Head revision this change set produces (non-empty).
    pub head_revision: String,
    /// Digest of the working tree state (non-empty).
    pub working_tree_digest: String,
    /// Artifacts that changed in this set.
    pub changed_artifacts: Vec<ChangedArtifactV1>,
    /// Digest of this change set (computed, excluded from its own digest input).
    pub change_set_digest: Option<ContentHash>,
}

impl ActiveChangeSetV1 {
    /// Creates a new V1 change set (caller must validate non-empty required fields).
    pub fn new(
        project_id: String,
        base_revision: String,
        head_revision: String,
        working_tree_digest: String,
        changed_artifacts: Vec<ChangedArtifactV1>,
    ) -> Self {
        let mut s = Self {
            schema_version: SCHEMA_VERSION,
            project_id,
            work_item_id: None,
            run_id: None,
            base_revision,
            head_revision,
            working_tree_digest,
            changed_artifacts,
            change_set_digest: None,
        };
        let digest = s.compute_change_set_digest();
        s.change_set_digest = Some(digest);
        s
    }

    /// Validates this change set instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.project_id.is_empty() {
            return Err(TestModelError::EmptyProjectId);
        }
        if self.base_revision.is_empty() {
            return Err(TestModelError::EmptyBaseRevision);
        }
        if self.head_revision.is_empty() {
            return Err(TestModelError::EmptyHeadRevision);
        }
        if self.working_tree_digest.is_empty() {
            return Err(TestModelError::EmptyWorkingTreeDigest);
        }
        Ok(())
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    ///
    /// Uses `serde_json::Map` with explicitly ordered insertions to guarantee
    /// consistent output regardless of construction context.
    pub fn to_canonical_json(&self) -> String {
        use serde_json::Map;

        // Canonical form excludes change_set_digest to avoid circularity.
        // Keys inserted in alphabetical order for determinism.
        let mut map = Map::new();
        map.insert(
            "base_revision".into(),
            serde_json::to_value(&self.base_revision).unwrap(),
        );
        map.insert(
            "changed_artifacts".into(),
            serde_json::to_value(&self.changed_artifacts).unwrap(),
        );
        map.insert(
            "head_revision".into(),
            serde_json::to_value(&self.head_revision).unwrap(),
        );
        map.insert(
            "project_id".into(),
            serde_json::to_value(&self.project_id).unwrap(),
        );
        map.insert("run_id".into(), serde_json::to_value(&self.run_id).unwrap());
        map.insert(
            "schema_version".into(),
            serde_json::to_value(self.schema_version).unwrap(),
        );
        map.insert(
            "work_item_id".into(),
            serde_json::to_value(&self.work_item_id).unwrap(),
        );
        map.insert(
            "working_tree_digest".into(),
            serde_json::to_value(&self.working_tree_digest).unwrap(),
        );
        serde_json::to_string(&map).expect("ActiveChangeSetV1 is always serializable")
    }

    /// Computes the change-set digest over the canonical JSON form (excludes itself).
    ///
    /// Format: `sha256:<64-hex-lowercase>`.
    pub fn compute_change_set_digest(&self) -> ContentHash {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }
}

// ── SutNode ───────────────────────────────────────────────────────────────────

/// A node in the project test topology representing a system under test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SutNodeV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Unique node identifier within the topology (non-empty).
    pub node_id: String,
    /// Kind of SUT node (closed set).
    pub kind: SutKind,
    /// Ecosystem identifier (e.g. "rust", "typescript", "" for neutral).
    /// Adapters map this to TEST-ADAPTER-* tooling; kernel stays build-neutral.
    pub ecosystem: String,
    /// Optional human-readable label.
    pub label: Option<String>,
}

impl SutNodeV1 {
    /// Creates a new V1 SUT node.
    pub fn new(node_id: String, kind: SutKind, ecosystem: String, label: Option<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            node_id,
            kind,
            ecosystem,
            label,
        }
    }

    /// Validates this SUT node instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.node_id.is_empty() {
            return Err(TestModelError::EmptyNodeId);
        }
        Ok(())
    }
}

// ── TopologyEdge ─────────────────────────────────────────────────────────────

/// Provenance record for every inferred topology edge (ADR-043 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EdgeProvenanceV1 {
    /// Source of the inference (tool, adapter, or heuristic name — non-empty).
    pub source: String,
    /// Version of the adapter or tool that produced this edge (non-empty).
    pub adapter_version: String,
    /// Source of confidence scoring for this edge (non-empty).
    pub confidence_source: String,
}

/// An edge in the project test topology connecting two SUT nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TopologyEdgeV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Kind of relationship (closed set).
    pub edge_kind: TopologyEdgeKind,
    /// Source node identifier.
    pub from_node: String,
    /// Target node identifier.
    pub to_node: String,
    /// Provenance of this inferred relationship (ADR-043: all inferred relations carry provenance).
    pub provenance: EdgeProvenanceV1,
}

impl TopologyEdgeV1 {
    /// Creates a new V1 topology edge.
    pub fn new(
        edge_kind: TopologyEdgeKind,
        from_node: String,
        to_node: String,
        provenance: EdgeProvenanceV1,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            edge_kind,
            from_node,
            to_node,
            provenance,
        }
    }

    /// Validates this topology edge instance (does NOT check node existence — use ProjectTestTopologyV1::validate for that).
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.from_node.is_empty() {
            return Err(TestModelError::EmptyEdgeFromNode);
        }
        if self.to_node.is_empty() {
            return Err(TestModelError::EmptyEdgeToNode);
        }
        if self.provenance.source.is_empty() {
            return Err(TestModelError::EmptyProvenanceSource);
        }
        if self.provenance.adapter_version.is_empty() {
            return Err(TestModelError::EmptyProvenanceAdapterVersion);
        }
        if self.provenance.confidence_source.is_empty() {
            return Err(TestModelError::EmptyProvenanceConfidenceSource);
        }
        Ok(())
    }
}

// ── ProjectTestTopology ────────────────────────────────────────────────────────

/// The project test topology — a typed, polyglot SUT graph (ADR-043).
///
/// Polyglot support: nodes carry independent `ecosystem` values; no single-ecosystem
/// assumption is made anywhere in the kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectTestTopologyV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Revision identifier for this topology snapshot (non-empty).
    pub topology_revision: String,
    /// SUT nodes keyed by node_id (BTreeMap ensures deterministic iteration).
    pub nodes: BTreeMap<String, SutNodeV1>,
    /// Edges connecting SUT nodes.
    pub edges: Vec<TopologyEdgeV1>,
}

impl ProjectTestTopologyV1 {
    /// Creates a new V1 project test topology.
    pub fn new(
        topology_revision: String,
        nodes: BTreeMap<String, SutNodeV1>,
        edges: Vec<TopologyEdgeV1>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            topology_revision,
            nodes,
            edges,
        }
    }

    /// Validates this topology instance.
    ///
    /// Checks: non-empty revision, non-empty nodes map, and all edge endpoints
    /// exist in the nodes map (no dangling references).
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.topology_revision.is_empty() {
            return Err(TestModelError::EmptyTopologyRevision);
        }
        if self.nodes.is_empty() {
            return Err(TestModelError::EmptyNodes);
        }
        // Check for dangling edges
        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from_node) {
                return Err(TestModelError::DanglingEdge {
                    edge_from: edge.from_node.clone(),
                    edge_to: edge.to_node.clone(),
                    missing_node: edge.from_node.clone(),
                });
            }
            if !self.nodes.contains_key(&edge.to_node) {
                return Err(TestModelError::DanglingEdge {
                    edge_from: edge.from_node.clone(),
                    edge_to: edge.to_node.clone(),
                    missing_node: edge.to_node.clone(),
                });
            }
        }
        Ok(())
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("ProjectTestTopologyV1 is always serializable")
    }

    /// Computes the content hash over the canonical JSON form.
    pub fn compute_content_hash(&self) -> ContentHash {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }
}

// ── VerificationCapability ────────────────────────────────────────────────────

/// A verification or test capability available in the project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VerificationCapabilityV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Unique capability identifier (non-empty).
    pub capability_id: String,
    /// Kind of capability (closed set).
    pub kind: CapabilityKind,
    /// SUT node kinds this capability can target (non-empty set).
    pub supported_sut_kinds: BTreeSet<SutKind>,
    /// Granularity at which this capability selects tests.
    pub selector_granularity: SelectorGranularity,
    /// Adapter or tool providing this capability (non-empty).
    pub adapter_id: String,
    /// Toolchain identity required (e.g. "rustc 1.75", "node 20" — non-empty).
    pub toolchain_identity: String,
    /// Estimated relative cost (None = unknown).
    pub estimated_cost: Option<f32>,
    /// Additional constraints (e.g. required env vars, tags).
    pub constraints: Vec<String>,
}

impl VerificationCapabilityV1 {
    /// Creates a new V1 verification capability.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        capability_id: String,
        kind: CapabilityKind,
        supported_sut_kinds: BTreeSet<SutKind>,
        selector_granularity: SelectorGranularity,
        adapter_id: String,
        toolchain_identity: String,
        estimated_cost: Option<f32>,
        constraints: Vec<String>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            capability_id,
            kind,
            supported_sut_kinds,
            selector_granularity,
            adapter_id,
            toolchain_identity,
            estimated_cost,
            constraints,
        }
    }

    /// Validates this capability instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.capability_id.is_empty() {
            return Err(TestModelError::EmptyCapabilityId);
        }
        if self.supported_sut_kinds.is_empty() {
            return Err(TestModelError::EmptySupportedSutKinds);
        }
        if self.adapter_id.is_empty() {
            return Err(TestModelError::EmptyAdapterId);
        }
        if self.toolchain_identity.is_empty() {
            return Err(TestModelError::EmptyToolchainIdentity);
        }
        if let Some(cost) = self.estimated_cost
            && (cost.is_nan() || cost < 0.0)
        {
            return Err(TestModelError::InvalidEstimatedCost { value: cost });
        }
        Ok(())
    }
}

// ── TestBatch ─────────────────────────────────────────────────────────────────

/// A batch of tests selected for a specific capability and scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TestBatchV1 {
    /// Execution stage (lower = earlier).
    pub stage: u32,
    /// Capability used to select these tests.
    pub capability_id: String,
    /// Semantic scope descriptors (paths, modules, labels).
    pub semantic_scope: Vec<String>,
    /// Selected test identifiers.
    pub test_ids: Vec<String>,
    /// Impact reasons for each selected test (non-empty per SPEC-043 §3.7).
    pub reasons: Vec<ImpactReason>,
    /// Expected relative cost for this batch (None = unknown).
    pub expected_cost: Option<f32>,
    /// Whether this batch should be escalated (run before others).
    pub escalation: bool,
}

impl TestBatchV1 {
    /// Validates this batch instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.capability_id.is_empty() {
            return Err(TestModelError::EmptyBatchCapabilityId);
        }
        if self.reasons.is_empty() {
            return Err(TestModelError::EmptyBatchReasons);
        }
        if let Some(cost) = self.expected_cost
            && (cost.is_nan() || cost < 0.0)
        {
            return Err(TestModelError::InvalidBatchCost { value: cost });
        }
        Ok(())
    }
}

// ── TestSelectionPlan ─────────────────────────────────────────────────────────

/// A test selection plan produced by the test selection algorithm (TEST-SELECT-001).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TestSelectionPlanV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Plan identifier (non-empty).
    pub plan_id: String,
    /// Change set digest this plan is for (non-empty).
    pub change_set_digest: String,
    /// Topology revision this plan was computed against (non-empty).
    pub topology_revision: String,
    /// SUT graph revision this plan was computed against (non-empty).
    pub sut_graph_revision: String,
    /// Policy revision in effect when computed (non-empty).
    pub policy_revision: String,
    /// SUT node IDs impacted by the change.
    pub impacted_sut: Vec<String>,
    /// Test batches to execute.
    pub batches: Vec<TestBatchV1>,
    /// Receipt IDs reused from previous runs (unchanged artifacts).
    pub reused_receipts: Vec<String>,
    /// SUT node IDs that could not be mapped to any capability.
    pub unmapped_nodes: Vec<String>,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f32,
    /// Overall plan verdict.
    pub verdict: PlanVerdict,
}

impl TestSelectionPlanV1 {
    /// Creates a new V1 test selection plan.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plan_id: String,
        change_set_digest: String,
        topology_revision: String,
        sut_graph_revision: String,
        policy_revision: String,
        impacted_sut: Vec<String>,
        batches: Vec<TestBatchV1>,
        reused_receipts: Vec<String>,
        unmapped_nodes: Vec<String>,
        confidence: f32,
        verdict: PlanVerdict,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            plan_id,
            change_set_digest,
            topology_revision,
            sut_graph_revision,
            policy_revision,
            impacted_sut,
            batches,
            reused_receipts,
            unmapped_nodes,
            confidence,
            verdict,
        }
    }

    /// Validates this plan instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.plan_id.is_empty() {
            return Err(TestModelError::EmptyPlanId);
        }
        if self.change_set_digest.is_empty() {
            return Err(TestModelError::EmptyChangeSetDigest);
        }
        if self.topology_revision.is_empty() {
            return Err(TestModelError::EmptyTopologyRevision);
        }
        if self.sut_graph_revision.is_empty() {
            return Err(TestModelError::EmptySutGraphRevision);
        }
        if self.policy_revision.is_empty() {
            return Err(TestModelError::EmptyPolicyRevision);
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(TestModelError::InvalidConfidence {
                value: self.confidence,
            });
        }
        for batch in &self.batches {
            batch.validate()?;
        }
        Ok(())
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("TestSelectionPlanV1 is always serializable")
    }

    /// Computes the content hash over the canonical JSON form.
    pub fn compute_content_hash(&self) -> ContentHash {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }
}

// ── TestEvidenceReceipt ───────────────────────────────────────────────────────

/// An evidence receipt recording the outcome of a test execution (TEST-EVIDENCE-001 lifecycle).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TestEvidenceReceiptV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Receipt identifier (non-empty).
    pub receipt_id: String,
    /// Change set digest this receipt is for (non-empty).
    pub change_set_digest: String,
    /// Source revision the tests were run against (non-empty).
    pub source_revision: String,
    /// Topology revision in effect (non-empty).
    pub topology_revision: String,
    /// SUT graph revision in effect (non-empty).
    pub sut_graph_revision: String,
    /// Policy revision in effect (non-empty).
    pub policy_revision: String,
    /// Capability that produced this receipt (non-empty).
    pub capability_id: String,
    /// Test outcome (closed set).
    pub result: ReceiptResult,
    /// RFC 3339 timestamp when execution completed (non-empty).
    pub completed_at: String,
}

impl TestEvidenceReceiptV1 {
    /// Creates a new V1 evidence receipt.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        receipt_id: String,
        change_set_digest: String,
        source_revision: String,
        topology_revision: String,
        sut_graph_revision: String,
        policy_revision: String,
        capability_id: String,
        result: ReceiptResult,
        completed_at: String,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            receipt_id,
            change_set_digest,
            source_revision,
            topology_revision,
            sut_graph_revision,
            policy_revision,
            capability_id,
            result,
            completed_at,
        }
    }

    /// Validates this receipt instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.receipt_id.is_empty() {
            return Err(TestModelError::EmptyReceiptId);
        }
        if self.change_set_digest.is_empty() {
            return Err(TestModelError::EmptyReceiptChangeSetDigest);
        }
        if self.source_revision.is_empty() {
            return Err(TestModelError::EmptySourceRevision);
        }
        if self.topology_revision.is_empty() {
            return Err(TestModelError::EmptyReceiptTopologyRevision);
        }
        if self.sut_graph_revision.is_empty() {
            return Err(TestModelError::EmptySutGraphRevision);
        }
        if self.policy_revision.is_empty() {
            return Err(TestModelError::EmptyReceiptPolicyRevision);
        }
        if self.capability_id.is_empty() {
            return Err(TestModelError::EmptyReceiptCapabilityId);
        }
        if self.completed_at.is_empty() {
            return Err(TestModelError::EmptyCompletedAt);
        }
        Ok(())
    }
}

// ── InsufficientMapping & MappingOutcome ──────────────────────────────────────

/// Fail-closed typed result when the test selection algorithm cannot produce
/// an actionable plan (ADR-043: typed insufficient-mapping result).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InsufficientMappingV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Artifact paths that could not be mapped to any SUT node.
    pub unmapped_artifacts: Vec<String>,
    /// SUT node IDs that have no associated capability.
    pub unmapped_suts: Vec<String>,
    /// Edge kinds that were required but could not be inferred.
    pub missing_relations: Vec<TopologyEdgeKind>,
    /// Capability IDs that were requested but are not available.
    pub unavailable_capabilities: Vec<String>,
    /// Human-readable justification for the failure (non-empty).
    pub justification: String,
    /// Steps to remediate the mapping gap (non-empty).
    pub remediation: String,
    /// Whether human verification is required before proceeding.
    pub verify_required: bool,
}

impl InsufficientMappingV1 {
    /// Creates a new V1 insufficient mapping result.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        unmapped_artifacts: Vec<String>,
        unmapped_suts: Vec<String>,
        missing_relations: Vec<TopologyEdgeKind>,
        unavailable_capabilities: Vec<String>,
        justification: String,
        remediation: String,
        verify_required: bool,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            unmapped_artifacts,
            unmapped_suts,
            missing_relations,
            unavailable_capabilities,
            justification,
            remediation,
            verify_required,
        }
    }

    /// Validates this insufficient mapping instance.
    pub fn validate(&self) -> Result<(), TestModelError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TestModelError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.justification.is_empty() {
            return Err(TestModelError::EmptyJustification);
        }
        if self.remediation.is_empty() {
            return Err(TestModelError::EmptyRemediation);
        }
        Ok(())
    }
}

/// The typed outcome of the test selection algorithm — fail-closed result shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingOutcome {
    /// Test selection produced an actionable plan.
    Mapped(TestSelectionPlanV1),
    /// Test selection could not produce a complete plan — fail-closed typed result.
    Insufficient(InsufficientMappingV1),
    /// Test selection requires human verification before proceeding.
    VerifyRequired,
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// Validation errors for test model aggregates.
///
/// All variants are closed (no catch-all) per REQ-4.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TestModelError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version found.
        got: u32,
        /// The version expected.
        want: u32,
    },

    // ── ActiveChangeSet ──────────────────────────────────────────────────────
    /// Project identifier is empty.
    #[error("project identifier is empty")]
    EmptyProjectId,

    /// Base revision is empty.
    #[error("base revision is empty")]
    EmptyBaseRevision,

    /// Head revision is empty.
    #[error("head revision is empty")]
    EmptyHeadRevision,

    /// Working tree digest is empty.
    #[error("working tree digest is empty")]
    EmptyWorkingTreeDigest,

    // ── SutNode ──────────────────────────────────────────────────────────────
    /// Node identifier is empty.
    #[error("SUT node identifier is empty")]
    EmptyNodeId,

    // ── TopologyEdge ─────────────────────────────────────────────────────────
    /// Edge source node identifier is empty.
    #[error("topology edge from_node is empty")]
    EmptyEdgeFromNode,

    /// Edge target node identifier is empty.
    #[error("topology edge to_node is empty")]
    EmptyEdgeToNode,

    /// Provenance source is empty.
    #[error("edge provenance source is empty")]
    EmptyProvenanceSource,

    /// Provenance adapter version is empty.
    #[error("edge provenance adapter_version is empty")]
    EmptyProvenanceAdapterVersion,

    /// Provenance confidence source is empty.
    #[error("edge provenance confidence_source is empty")]
    EmptyProvenanceConfidenceSource,

    // ── ProjectTestTopology ─────────────────────────────────────────────────
    /// Topology revision is empty.
    #[error("topology revision is empty")]
    EmptyTopologyRevision,

    /// Nodes map is empty.
    #[error("project test topology has no nodes")]
    EmptyNodes,

    /// Edge references a node that does not exist in the topology.
    #[error(
        "topology edge references missing node '{missing_node}' (from={edge_from}, to={edge_to})"
    )]
    DanglingEdge {
        /// Source node of the dangling edge.
        edge_from: String,
        /// Target node of the dangling edge.
        edge_to: String,
        /// Node ID that does not exist.
        missing_node: String,
    },

    // ── VerificationCapability ───────────────────────────────────────────────
    /// Capability identifier is empty.
    #[error("capability identifier is empty")]
    EmptyCapabilityId,

    /// Supported SUT kinds set is empty.
    #[error("verification capability has no supported SUT kinds")]
    EmptySupportedSutKinds,

    /// Adapter identifier is empty.
    #[error("adapter identifier is empty")]
    EmptyAdapterId,

    /// Toolchain identity is empty.
    #[error("toolchain identity is empty")]
    EmptyToolchainIdentity,

    /// Estimated cost is invalid (NaN or negative).
    #[error("estimated cost is invalid: {value}")]
    InvalidEstimatedCost {
        /// The invalid cost value.
        value: f32,
    },

    // ── TestBatch ───────────────────────────────────────────────────────────
    /// Batch capability identifier is empty.
    #[error("test batch capability_id is empty")]
    EmptyBatchCapabilityId,

    /// Batch reasons are empty (non-empty required per SPEC-043 §3.7).
    #[error("test batch has no impact reasons")]
    EmptyBatchReasons,

    /// Batch expected cost is invalid.
    #[error("test batch expected_cost is invalid: {value}")]
    InvalidBatchCost {
        /// The invalid cost value.
        value: f32,
    },

    // ── TestSelectionPlan ───────────────────────────────────────────────────
    /// Plan identifier is empty.
    #[error("plan identifier is empty")]
    EmptyPlanId,

    /// Change set digest is empty.
    #[error("change set digest is empty")]
    EmptyChangeSetDigest,

    /// SUT graph revision is empty.
    #[error("SUT graph revision is empty")]
    EmptySutGraphRevision,

    /// Policy revision is empty.
    #[error("policy revision is empty")]
    EmptyPolicyRevision,

    /// Confidence score is outside [0.0, 1.0].
    #[error("confidence {value} is outside valid range [0.0, 1.0]")]
    InvalidConfidence {
        /// The invalid confidence value.
        value: f32,
    },

    // ── TestEvidenceReceipt ─────────────────────────────────────────────────
    /// Receipt identifier is empty.
    #[error("receipt identifier is empty")]
    EmptyReceiptId,

    /// Receipt change set digest is empty.
    #[error("receipt change_set_digest is empty")]
    EmptyReceiptChangeSetDigest,

    /// Source revision is empty.
    #[error("receipt source_revision is empty")]
    EmptySourceRevision,

    /// Receipt topology revision is empty.
    #[error("receipt topology_revision is empty")]
    EmptyReceiptTopologyRevision,

    /// Receipt policy revision is empty.
    #[error("receipt policy_revision is empty")]
    EmptyReceiptPolicyRevision,

    /// Receipt capability_id is empty.
    #[error("receipt capability_id is empty")]
    EmptyReceiptCapabilityId,

    /// completed_at timestamp is empty.
    #[error("receipt completed_at is empty")]
    EmptyCompletedAt,

    // ── InsufficientMapping ─────────────────────────────────────────────────
    /// Justification is empty.
    #[error("insufficient mapping justification is empty")]
    EmptyJustification,

    /// Remediation is empty.
    #[error("insufficient mapping remediation is empty")]
    EmptyRemediation,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-2: variant-count assertions
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn change_kind_variant_count() {
        let variants = [
            ChangeKind::Added,
            ChangeKind::Modified,
            ChangeKind::Deleted,
            ChangeKind::Renamed,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn sut_kind_variant_count() {
        let variants = [
            SutKind::Repository,
            SutKind::Workspace,
            SutKind::Component,
            SutKind::BuildUnit,
            SutKind::SourceArtifact,
            SutKind::ModuleOrNamespace,
            SutKind::Symbol,
            SutKind::RuntimeService,
            SutKind::ContractBoundary,
            SutKind::Schema,
            SutKind::ConfigurationSurface,
            SutKind::GeneratedArtifact,
            SutKind::TestUnit,
            SutKind::TestSuite,
            SutKind::VerificationCapability,
            SutKind::EvidenceReceipt,
        ];
        assert_eq!(variants.len(), 16);
    }

    #[test]
    fn topology_edge_kind_variant_count() {
        let variants = [
            TopologyEdgeKind::Touches,
            TopologyEdgeKind::Owns,
            TopologyEdgeKind::Builds,
            TopologyEdgeKind::DependsOn,
            TopologyEdgeKind::RuntimeDependsOn,
            TopologyEdgeKind::ReverseDependsOn,
            TopologyEdgeKind::Generates,
            TopologyEdgeKind::Tests,
            TopologyEdgeKind::Covers,
            TopologyEdgeKind::ValidatesContract,
            TopologyEdgeKind::ContractDependency,
            TopologyEdgeKind::UsesCapability,
            TopologyEdgeKind::ProducedEvidence,
            TopologyEdgeKind::Invalidates,
        ];
        assert_eq!(variants.len(), 14);
    }

    #[test]
    fn capability_kind_variant_count() {
        let variants = [
            CapabilityKind::Compile,
            CapabilityKind::TypeCheck,
            CapabilityKind::Lint,
            CapabilityKind::Unit,
            CapabilityKind::Integration,
            CapabilityKind::Contract,
            CapabilityKind::E2e,
            CapabilityKind::Security,
            CapabilityKind::Mutation,
            CapabilityKind::Architecture,
            CapabilityKind::Uat,
            CapabilityKind::Custom,
        ];
        assert_eq!(variants.len(), 12);
    }

    #[test]
    fn selector_granularity_variant_count() {
        let variants = [
            SelectorGranularity::Repository,
            SelectorGranularity::Workspace,
            SelectorGranularity::Component,
            SelectorGranularity::BuildUnit,
            SelectorGranularity::File,
            SelectorGranularity::Symbol,
            SelectorGranularity::TestId,
            SelectorGranularity::TagFilter,
        ];
        assert_eq!(variants.len(), 8);
    }

    #[test]
    fn impact_reason_variant_count() {
        let variants = [
            ImpactReason::DirectSourceTouch,
            ImpactReason::ComponentOwnership,
            ImpactReason::BuildUnitOwnership,
            ImpactReason::DependencyPropagation,
            ImpactReason::ReverseDependencyPropagation,
            ImpactReason::RuntimeDependencyPropagation,
            ImpactReason::PublicContractChange,
            ImpactReason::SchemaChange,
            ImpactReason::BuildOrWorkspaceChange,
            ImpactReason::ConfigurationChange,
            ImpactReason::GeneratedSurfaceChange,
            ImpactReason::ExplicitTestAssociation,
            ImpactReason::LocalUnitTest,
            ImpactReason::ComponentIntegrationTest,
            ImpactReason::CrossComponentContractTest,
        ];
        assert_eq!(variants.len(), 15);
    }

    #[test]
    fn plan_verdict_variant_count() {
        let variants = [
            PlanVerdict::Executable,
            PlanVerdict::Blocked,
            PlanVerdict::VerifyRequired,
        ];
        assert_eq!(variants.len(), 3);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-3: aggregate field-level (byte-stability + hash mutation-inequality)
    // ═══════════════════════════════════════════════════════════════════════════

    // ── ActiveChangeSet ───────────────────────────────────────────────────────

    #[test]
    fn active_change_set_digest_stability() {
        let cs1 = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![ChangedArtifactV1 {
                path: "src/main.rs".into(),
                change_kind: ChangeKind::Modified,
                staged: true,
            }],
        );
        let cs2 = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![ChangedArtifactV1 {
                path: "src/main.rs".into(),
                change_kind: ChangeKind::Modified,
                staged: true,
            }],
        );
        assert_eq!(
            cs1.change_set_digest, cs2.change_set_digest,
            "identical change sets must produce identical digests"
        );
    }

    #[test]
    fn active_change_set_digest_mutation_inequality() {
        let cs = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![ChangedArtifactV1 {
                path: "src/main.rs".into(),
                change_kind: ChangeKind::Modified,
                staged: true,
            }],
        );
        let digest_before = cs.change_set_digest.clone();

        // Mutate base_revision
        let mut cs2 = cs.clone();
        cs2.base_revision = "other".into();

        // Recompute digest from the mutated canonical JSON (this is what would be
        // stored if new() were called afresh with the mutated data)
        let new_digest = cs2.compute_change_set_digest();
        let prev = digest_before.as_ref().map(String::as_str);
        assert_ne!(
            prev,
            Some(new_digest.as_str()),
            "mutated base_revision must change digest. before={:?}, new={:?}",
            digest_before,
            new_digest
        );
    }

    #[test]
    fn active_change_set_digest_artifact_mutation() {
        let cs = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![ChangedArtifactV1 {
                path: "src/main.rs".into(),
                change_kind: ChangeKind::Modified,
                staged: true,
            }],
        );
        let digest_before = cs.change_set_digest.clone();

        // Change artifact path
        let cs2 = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![ChangedArtifactV1 {
                path: "src/lib.rs".into(),
                change_kind: ChangeKind::Modified,
                staged: true,
            }],
        );
        assert_ne!(
            digest_before, cs2.change_set_digest,
            "changed artifact path must change digest"
        );
    }

    // ── ProjectTestTopology ────────────────────────────────────────────────────

    #[test]
    fn topology_canonical_byte_stable() {
        let nodes1: BTreeMap<String, SutNodeV1> = [
            (
                "node-1".into(),
                SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
            ),
            (
                "node-2".into(),
                SutNodeV1::new("node-2".into(), SutKind::Workspace, "rust".into(), None),
            ),
        ]
        .into_iter()
        .collect();

        let nodes2: BTreeMap<String, SutNodeV1> = [
            (
                "node-2".into(),
                SutNodeV1::new("node-2".into(), SutKind::Workspace, "rust".into(), None),
            ),
            (
                "node-1".into(),
                SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
            ),
        ]
        .into_iter()
        .collect();

        let topo1 = ProjectTestTopologyV1::new("rev-1".into(), nodes1, vec![]);
        let topo2 = ProjectTestTopologyV1::new("rev-1".into(), nodes2, vec![]);

        let json1 = topo1.to_canonical_json();
        let json2 = topo2.to_canonical_json();
        assert_eq!(
            json1, json2,
            "BTreeMap iteration order must produce identical canonical JSON"
        );
    }

    #[test]
    fn topology_hash_mutation_inequality() {
        let nodes: BTreeMap<String, SutNodeV1> = [(
            "node-1".into(),
            SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
        )]
        .into_iter()
        .collect();
        let topo = ProjectTestTopologyV1::new("rev-1".into(), nodes, vec![]);

        let mut topo2 = topo.clone();
        topo2.topology_revision = "rev-2".into();
        assert_ne!(
            topo.compute_content_hash(),
            topo2.compute_content_hash(),
            "mutated topology_revision must change hash"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-3: serde round-trip per aggregate
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn serde_roundtrip_active_change_set() {
        let cs = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![
                ChangedArtifactV1 {
                    path: "src/main.rs".into(),
                    change_kind: ChangeKind::Modified,
                    staged: true,
                },
                ChangedArtifactV1 {
                    path: "tests/integration.rs".into(),
                    change_kind: ChangeKind::Added,
                    staged: false,
                },
            ],
        );
        let json = serde_json::to_string(&cs).unwrap();
        let roundtrip: ActiveChangeSetV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(cs, roundtrip, "round-trip must preserve value equality");
    }

    #[test]
    fn serde_roundtrip_sut_node() {
        let node = SutNodeV1::new(
            "my-crate".into(),
            SutKind::BuildUnit,
            "rust".into(),
            Some("My Crate".into()),
        );
        let json = serde_json::to_string(&node).unwrap();
        let roundtrip: SutNodeV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(node, roundtrip);
    }

    #[test]
    fn serde_roundtrip_topology_edge() {
        let edge = TopologyEdgeV1::new(
            TopologyEdgeKind::DependsOn,
            "crate-a".into(),
            "crate-b".into(),
            EdgeProvenanceV1 {
                source: "cargo-metadata".into(),
                adapter_version: "0.1.0".into(),
                confidence_source: "static-analysis".into(),
            },
        );
        let json = serde_json::to_string(&edge).unwrap();
        let roundtrip: TopologyEdgeV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, roundtrip);
    }

    #[test]
    fn serde_roundtrip_project_test_topology() {
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "node-1".into(),
            SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
        );
        nodes.insert(
            "node-2".into(),
            SutNodeV1::new(
                "node-2".into(),
                SutKind::Workspace,
                "typescript".into(),
                None,
            ),
        );

        let edges = vec![TopologyEdgeV1::new(
            TopologyEdgeKind::Owns,
            "node-1".into(),
            "node-2".into(),
            EdgeProvenanceV1 {
                source: "tsconfig-analyzer".into(),
                adapter_version: "1.2.3".into(),
                confidence_source: "declaration-graph".into(),
            },
        )];

        let topo = ProjectTestTopologyV1::new("topo-rev-1".into(), nodes, edges);
        let json = serde_json::to_string(&topo).unwrap();
        let roundtrip: ProjectTestTopologyV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(topo, roundtrip);
    }

    #[test]
    fn serde_roundtrip_verification_capability() {
        let mut sut_kinds = BTreeSet::new();
        sut_kinds.insert(SutKind::BuildUnit);
        sut_kinds.insert(SutKind::SourceArtifact);

        let cap = VerificationCapabilityV1::new(
            "cargo-test".into(),
            CapabilityKind::Unit,
            sut_kinds,
            SelectorGranularity::File,
            "TEST-ADAPTER-RUST".into(),
            "cargo 1.75".into(),
            Some(1.0),
            vec!["--lib".into(), "--bins".into()],
        );
        let json = serde_json::to_string(&cap).unwrap();
        let roundtrip: VerificationCapabilityV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, roundtrip);
    }

    #[test]
    fn serde_roundtrip_test_selection_plan() {
        let plan = TestSelectionPlanV1::new(
            "plan-001".into(),
            "sha256:digest".into(),
            "topo-rev-1".into(),
            "graph-rev-1".into(),
            "policy-rev-1".into(),
            vec!["node-1".into(), "node-2".into()],
            vec![TestBatchV1 {
                stage: 1,
                capability_id: "cargo-test".into(),
                semantic_scope: vec!["src/".into()],
                test_ids: vec!["test_a".into(), "test_b".into()],
                reasons: vec![ImpactReason::DirectSourceTouch],
                expected_cost: Some(0.5),
                escalation: false,
            }],
            vec![],
            vec![],
            0.95,
            PlanVerdict::Executable,
        );
        let json = serde_json::to_string(&plan).unwrap();
        let roundtrip: TestSelectionPlanV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(plan, roundtrip);
    }

    #[test]
    fn serde_roundtrip_test_evidence_receipt() {
        let receipt = TestEvidenceReceiptV1::new(
            "receipt-001".into(),
            "sha256:digest".into(),
            "abc123".into(),
            "topo-rev-1".into(),
            "graph-rev-1".into(),
            "policy-rev-1".into(),
            "cargo-test".into(),
            ReceiptResult::Passed,
            "2026-09-03T12:00:00Z".into(),
        );
        let json = serde_json::to_string(&receipt).unwrap();
        let roundtrip: TestEvidenceReceiptV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt, roundtrip);
    }

    #[test]
    fn serde_roundtrip_insufficient_mapping() {
        let insufficient = InsufficientMappingV1::new(
            vec!["unknown-artifact".into()],
            vec!["unmapped-node".into()],
            vec![TopologyEdgeKind::DependsOn],
            vec!["missing-cap".into()],
            "Could not resolve all SUT nodes".into(),
            "Add BUILD.bazel mapping for unknown-artifact".into(),
            true,
        );
        let json = serde_json::to_string(&insufficient).unwrap();
        let roundtrip: InsufficientMappingV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(insufficient, roundtrip);
    }

    #[test]
    fn serde_roundtrip_mapping_outcome_mapped() {
        let plan = TestSelectionPlanV1::new(
            "plan-001".into(),
            "sha256:digest".into(),
            "topo-rev-1".into(),
            "graph-rev-1".into(),
            "policy-rev-1".into(),
            vec!["node-1".into()],
            vec![],
            vec![],
            vec![],
            1.0,
            PlanVerdict::Executable,
        );
        let outcome = MappingOutcome::Mapped(plan);
        let json = serde_json::to_string(&outcome).unwrap();
        let roundtrip: MappingOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(outcome, roundtrip);
    }

    #[test]
    fn serde_roundtrip_mapping_outcome_insufficient() {
        let insufficient = InsufficientMappingV1::new(
            vec![],
            vec![],
            vec![],
            vec![],
            "No capabilities available".into(),
            "Register a test adapter".into(),
            false,
        );
        let outcome = MappingOutcome::Insufficient(insufficient);
        let json = serde_json::to_string(&outcome).unwrap();
        let roundtrip: MappingOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(outcome, roundtrip);
    }

    #[test]
    fn serde_roundtrip_mapping_outcome_verify_required() {
        let outcome = MappingOutcome::VerifyRequired;
        let json = serde_json::to_string(&outcome).unwrap();
        let roundtrip: MappingOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(outcome, roundtrip);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-4: validation — rejection + accept tests
    // ═══════════════════════════════════════════════════════════════════════════

    // ── ActiveChangeSet ───────────────────────────────────────────────────────

    #[test]
    fn validate_rejects_empty_project_id() {
        let cs = ActiveChangeSetV1 {
            schema_version: SCHEMA_VERSION,
            project_id: "".into(),
            work_item_id: None,
            run_id: None,
            base_revision: "abc".into(),
            head_revision: "def".into(),
            working_tree_digest: "sha256:abc".into(),
            changed_artifacts: vec![],
            change_set_digest: None,
        };
        assert!(matches!(cs.validate(), Err(TestModelError::EmptyProjectId)));
    }

    #[test]
    fn validate_accepts_valid_active_change_set() {
        let cs = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![],
        );
        assert!(cs.validate().is_ok());
    }

    #[test]
    fn validate_rejects_wrong_schema_version_change_set() {
        let mut cs = ActiveChangeSetV1::new(
            "proj-1".into(),
            "abc123".into(),
            "def456".into(),
            "sha256:working".into(),
            vec![],
        );
        cs.schema_version = 99;
        assert!(matches!(
            cs.validate(),
            Err(TestModelError::UnsupportedSchemaVersion { got: 99, want: 1 })
        ));
    }

    // ── SutNode ───────────────────────────────────────────────────────────────

    #[test]
    fn validate_rejects_empty_node_id() {
        let node = SutNodeV1::new("".into(), SutKind::Repository, "rust".into(), None);
        assert!(matches!(node.validate(), Err(TestModelError::EmptyNodeId)));
    }

    #[test]
    fn validate_accepts_valid_sut_node() {
        let node = SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None);
        assert!(node.validate().is_ok());
    }

    // ── ProjectTestTopology ──────────────────────────────────────────────────

    #[test]
    fn validate_rejects_empty_topology_revision() {
        let nodes: BTreeMap<String, SutNodeV1> = [(
            "node-1".into(),
            SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
        )]
        .into_iter()
        .collect();
        let topo = ProjectTestTopologyV1::new("".into(), nodes, vec![]);
        assert!(matches!(
            topo.validate(),
            Err(TestModelError::EmptyTopologyRevision)
        ));
    }

    #[test]
    fn validate_rejects_empty_nodes() {
        let topo = ProjectTestTopologyV1::new("rev-1".into(), BTreeMap::new(), vec![]);
        assert!(matches!(topo.validate(), Err(TestModelError::EmptyNodes)));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-4: dangling edge rejected
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn validate_rejects_dangling_edge() {
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "node-1".into(),
            SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
        );

        // Edge references "node-2" which doesn't exist
        let edge = TopologyEdgeV1::new(
            TopologyEdgeKind::DependsOn,
            "node-1".into(),
            "node-2".into(),
            EdgeProvenanceV1 {
                source: "test-adapter".into(),
                adapter_version: "0.0.1".into(),
                confidence_source: "static".into(),
            },
        );
        let topo = ProjectTestTopologyV1::new("rev-1".into(), nodes, vec![edge]);
        let err = topo.validate().unwrap_err();
        assert!(matches!(
            err,
            TestModelError::DanglingEdge { missing_node, .. } if missing_node == "node-2"
        ));
    }

    #[test]
    fn validate_accepts_valid_topology_with_edges() {
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "node-1".into(),
            SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
        );
        nodes.insert(
            "node-2".into(),
            SutNodeV1::new("node-2".into(), SutKind::Workspace, "rust".into(), None),
        );

        let edge = TopologyEdgeV1::new(
            TopologyEdgeKind::Owns,
            "node-1".into(),
            "node-2".into(),
            EdgeProvenanceV1 {
                source: "test-adapter".into(),
                adapter_version: "0.0.1".into(),
                confidence_source: "static".into(),
            },
        );
        let topo = ProjectTestTopologyV1::new("rev-1".into(), nodes, vec![edge]);
        assert!(topo.validate().is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6: MappingOutcome fail-closed construction
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn insufficient_mapping_full_construction() {
        let insufficient = InsufficientMappingV1::new(
            vec!["art-a".into(), "art-b".into()],
            vec!["node-x".into()],
            vec![TopologyEdgeKind::DependsOn, TopologyEdgeKind::Builds],
            vec!["cap-missing".into()],
            "Several nodes could not be mapped to any test capability".into(),
            "Register TEST-ADAPTER-* capabilities for rust/typescript ecosystems".into(),
            true,
        );
        assert!(insufficient.validate().is_ok());
        assert!(insufficient.verify_required);
    }

    #[test]
    fn insufficient_mapping_validate_rejects_empty_justification() {
        let insufficient = InsufficientMappingV1::new(
            vec![],
            vec![],
            vec![],
            vec![],
            "".into(),
            "Do something".into(),
            false,
        );
        assert!(matches!(
            insufficient.validate(),
            Err(TestModelError::EmptyJustification)
        ));
    }

    #[test]
    fn insufficient_mapping_validate_rejects_empty_remediation() {
        let insufficient = InsufficientMappingV1::new(
            vec![],
            vec![],
            vec![],
            vec![],
            "Missing mapping".into(),
            "".into(),
            false,
        );
        assert!(matches!(
            insufficient.validate(),
            Err(TestModelError::EmptyRemediation)
        ));
    }

    #[test]
    fn mapping_outcome_verify_required() {
        let outcome = MappingOutcome::VerifyRequired;
        // VerifyRequired carries no data — human must assess
        assert!(matches!(outcome, MappingOutcome::VerifyRequired));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6: change_set_digest stability
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn change_set_digest_same_inputs_same_digest() {
        let cs1 = ActiveChangeSetV1::new(
            "proj".into(),
            "base".into(),
            "head".into(),
            "tree".into(),
            vec![ChangedArtifactV1 {
                path: "a.rs".into(),
                change_kind: ChangeKind::Added,
                staged: true,
            }],
        );
        let cs2 = ActiveChangeSetV1::new(
            "proj".into(),
            "base".into(),
            "head".into(),
            "tree".into(),
            vec![ChangedArtifactV1 {
                path: "a.rs".into(),
                change_kind: ChangeKind::Added,
                staged: true,
            }],
        );
        assert_eq!(cs1.change_set_digest, cs2.change_set_digest);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6: polyglot topology fixture (≥3 ecosystems)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn polyglot_topology_three_ecosystems() {
        // Polyglot fixture: rust + typescript + python coexisting
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "rust-repo".into(),
            SutNodeV1::new(
                "rust-repo".into(),
                SutKind::Repository,
                "rust".into(),
                Some("Rust monorepo".into()),
            ),
        );
        nodes.insert(
            "ts-workspace".into(),
            SutNodeV1::new(
                "ts-workspace".into(),
                SutKind::Workspace,
                "typescript".into(),
                Some("TypeScript app".into()),
            ),
        );
        nodes.insert(
            "py-repo".into(),
            SutNodeV1::new(
                "py-repo".into(),
                SutKind::Repository,
                "python".into(),
                Some("Python library".into()),
            ),
        );
        nodes.insert(
            "rust-crate-a".into(),
            SutNodeV1::new(
                "rust-crate-a".into(),
                SutKind::BuildUnit,
                "rust".into(),
                None,
            ),
        );
        nodes.insert(
            "ts-component".into(),
            SutNodeV1::new(
                "ts-component".into(),
                SutKind::Component,
                "typescript".into(),
                None,
            ),
        );

        // Cross-ecosystem edge
        let edge = TopologyEdgeV1::new(
            TopologyEdgeKind::UsesCapability,
            "rust-crate-a".into(),
            "ts-component".into(),
            EdgeProvenanceV1 {
                source: "polyglot-analyzer".into(),
                adapter_version: "0.2.0".into(),
                confidence_source: "cargo-and-tsconfig".into(),
            },
        );

        let topo = ProjectTestTopologyV1::new("polyglot-rev-1".into(), nodes, vec![edge]);
        assert!(topo.validate().is_ok());

        // Verify 3 distinct ecosystems
        let ecosystems: std::collections::BTreeSet<_> =
            topo.nodes.values().map(|n| n.ecosystem.clone()).collect();
        assert!(
            ecosystems.contains("rust")
                && ecosystems.contains("typescript")
                && ecosystems.contains("python"),
            "polyglot topology must contain rust, typescript, and python ecosystems"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional: content hash format
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn content_hash_format() {
        let nodes: BTreeMap<String, SutNodeV1> = [(
            "node-1".into(),
            SutNodeV1::new("node-1".into(), SutKind::Repository, "rust".into(), None),
        )]
        .into_iter()
        .collect();
        let topo = ProjectTestTopologyV1::new("rev-1".into(), nodes, vec![]);
        let hash = topo.compute_content_hash();
        assert!(
            hash.starts_with("sha256:"),
            "hash must start with 'sha256:': {}",
            hash
        );
        let hex = &hash[7..];
        assert_eq!(hex.len(), 64, "hash hex must be 64 chars: {}", hex);
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "hash hex must be all hex digits: {}",
            hex
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional: verification capability validation
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn validate_rejects_empty_capability_id() {
        let cap = VerificationCapabilityV1::new(
            "".into(),
            CapabilityKind::Unit,
            [SutKind::BuildUnit].into_iter().collect(),
            SelectorGranularity::File,
            "adapter".into(),
            "toolchain".into(),
            None,
            vec![],
        );
        assert!(matches!(
            cap.validate(),
            Err(TestModelError::EmptyCapabilityId)
        ));
    }

    #[test]
    fn validate_rejects_empty_supported_sut_kinds() {
        let cap = VerificationCapabilityV1::new(
            "cap-1".into(),
            CapabilityKind::Unit,
            BTreeSet::new(),
            SelectorGranularity::File,
            "adapter".into(),
            "toolchain".into(),
            None,
            vec![],
        );
        assert!(matches!(
            cap.validate(),
            Err(TestModelError::EmptySupportedSutKinds)
        ));
    }

    #[test]
    fn validate_rejects_invalid_estimated_cost() {
        let cap = VerificationCapabilityV1::new(
            "cap-1".into(),
            CapabilityKind::Unit,
            [SutKind::BuildUnit].into_iter().collect(),
            SelectorGranularity::File,
            "adapter".into(),
            "toolchain".into(),
            Some(f32::NAN),
            vec![],
        );
        assert!(matches!(
            cap.validate(),
            Err(TestModelError::InvalidEstimatedCost { .. })
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional: test selection plan validation
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn validate_rejects_invalid_confidence() {
        let plan = TestSelectionPlanV1::new(
            "plan-1".into(),
            "sha256:digest".into(),
            "topo".into(),
            "graph".into(),
            "policy".into(),
            vec![],
            vec![],
            vec![],
            vec![],
            1.5, // out of range
            PlanVerdict::Executable,
        );
        let err = plan.validate().unwrap_err();
        assert!(matches!(err, TestModelError::InvalidConfidence { .. }));
        let msg = err.to_string();
        assert!(
            msg.contains("1.5") || msg.contains("confidence"),
            "error message should mention confidence and value: {}",
            msg
        );
    }

    #[test]
    fn validate_rejects_empty_batch_reasons() {
        let plan = TestSelectionPlanV1::new(
            "plan-1".into(),
            "sha256:digest".into(),
            "topo".into(),
            "graph".into(),
            "policy".into(),
            vec![],
            vec![TestBatchV1 {
                stage: 1,
                capability_id: "cap".into(),
                semantic_scope: vec![],
                test_ids: vec![],
                reasons: vec![], // EMPTY — must be non-empty
                expected_cost: None,
                escalation: false,
            }],
            vec![],
            vec![],
            0.5,
            PlanVerdict::Executable,
        );
        assert!(matches!(
            plan.validate(),
            Err(TestModelError::EmptyBatchReasons)
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional: serde rename values
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn serde_change_kind_rename() {
        let json = serde_json::to_string(&ChangeKind::Added).unwrap();
        assert_eq!(json, "\"added\"");
        let roundtrip: ChangeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, ChangeKind::Added);
    }

    #[test]
    fn serde_sut_kind_rename() {
        let json = serde_json::to_string(&SutKind::BuildUnit).unwrap();
        assert_eq!(json, "\"build_unit\"");
        let roundtrip: SutKind = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, SutKind::BuildUnit);
    }

    #[test]
    fn serde_plan_verdict_rename() {
        let json = serde_json::to_string(&PlanVerdict::VerifyRequired).unwrap();
        assert_eq!(json, "\"verify_required\"");
        let roundtrip: PlanVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, PlanVerdict::VerifyRequired);
    }
}
