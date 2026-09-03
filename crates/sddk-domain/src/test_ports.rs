//! SPI ports for test/adapter layer (SPEC-043 §4).
//!
//! These ports define the boundary between the SDDK kernel and concrete
//! adapter implementations. Adapters detect manifests/tooling, expose topology,
//! translate semantic batches into tool invocations, and parse results into
//! canonical receipts. Adapters **never select** what should be tested — that
//! is the kernel's responsibility.
//!
//! # Design
//!
//! - All traits are object-safe (`dyn`-compatible, `Send + Sync` supertraits).
//! - No generic methods, no `Self` by value in arguments.
//! - Error type is the closed [`AdapterError`] (3 variants, thiserror).
//! - Versioned envelopes for registry and mapping data.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json;
use serde_saphyr as saphyr;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::test_model::{
    ActiveChangeSetV1, CapabilityKind, ProjectTestTopologyV1, SelectorGranularity, SutKind,
    TestBatchV1, TestEvidenceReceiptV1, TestSelectionPlanV1, TopologyEdgeV1,
    VerificationCapabilityV1,
};

/// Content hash in `sha256:<64-hex-lowercase>` format.
pub type ContentHash = String;

// ── REQ-1: AdapterError ─────────────────────────────────────────────────────────

/// Closed error type for all adapter SPI operations (SPEC-043 §4).
///
/// Exactly 3 variants — adding a variant requires updating every adapter
/// implementation and the SPEC-043 §4 contract.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AdapterError {
    /// The requested data is not available right now.
    #[error("unavailable: {reason}")]
    Unavailable {
        /// Why the data is unavailable.
        reason: String,
    },

    /// Adapter failed to detect or parse the required data.
    #[error("detection failed: {reason}")]
    DetectionFailed {
        /// Why detection failed.
        reason: String,
    },

    /// Caller passed invalid input (e.g. empty sut_node_id).
    #[error("invalid input: {reason}")]
    InvalidInput {
        /// Why the input is invalid.
        reason: String,
    },
}

crate::assert_variant_count_eq!(
    AdapterError,
    3,
    [
        AdapterError::Unavailable { .. },
        AdapterError::DetectionFailed { .. },
        AdapterError::InvalidInput { .. },
    ]
);

// ── REQ-1: SPI ports (9 traits) ────────────────────────────────────────────────

/// Port: retrieves the active change set (SPEC-043 §4.1).
///
/// Adapters implement this to expose the current set of changed artifacts
/// detected by the VCS adapter. The kernel owns selection semantics.
pub trait ActiveChangeSetPort: Send + Sync {
    /// Returns the active change set for this project.
    fn active_change_set(&self) -> Result<ActiveChangeSetV1, AdapterError>;
}

/// Port: retrieves the project topology (SPEC-043 §4.2).
///
/// Adapters implement this to expose the SUT graph derived from workspace
/// structure, build graphs, and dependency analysis.
pub trait ProjectTopologyPort: Send + Sync {
    /// Returns the current project test topology.
    fn topology(&self) -> Result<ProjectTestTopologyV1, AdapterError>;
}

/// Port: retrieves SUT graph metadata (SPEC-043 §4.3).
///
/// Adapters implement this to expose the revision of the SUT dependency
/// graph and its edges.
pub trait SutGraphPort: Send + Sync {
    /// Returns the current SUT graph revision identifier.
    fn sut_graph_revision(&self) -> Result<String, AdapterError>;

    /// Returns all edges in the SUT dependency graph.
    fn edges(&self) -> Result<Vec<TopologyEdgeV1>, AdapterError>;
}

/// Port: exposes available verification capabilities (SPEC-043 §4.4).
///
/// Adapters implement this to declare what test/verification capabilities
/// are available in this project (compilation checks, unit tests, etc.).
pub trait VerificationCapabilityRegistry: Send + Sync {
    /// Returns all verification capabilities available in this project.
    fn capabilities(&self) -> Result<Vec<VerificationCapabilityV1>, AdapterError>;
}

/// Port: resolves stable test identities for a SUT node (SPEC-043 §4.5).
///
/// Adapters implement this to provide stable test identifiers that persist
/// across topology revisions. The kernel uses these as stable keys.
pub trait TestCatalogPort: Send + Sync {
    /// Returns stable test identifiers for the given SUT node.
    ///
    /// The returned identifiers are ecosystem-specific strings (e.g. "tests/unit/foo.rs"
    /// for Rust, "test/Foo.test.ts" for TypeScript) that remain stable across
    /// topology changes.
    fn tests_for(&self, sut_node_id: &str) -> Result<Vec<String>, AdapterError>;
}

/// Port: computes a test selection plan for a change set (SPEC-043 §4.6).
///
/// Adapters implement this to translate a change-set digest and topology revision
/// into a [`TestSelectionPlanV1`] — a structured selection of which tests to run.
pub trait TestImpactPlannerPort: Send + Sync {
    /// Returns a test selection plan for the given change set and topology revision.
    fn plan(
        &self,
        change_set_digest: &str,
        topology_revision: &str,
    ) -> Result<TestSelectionPlanV1, AdapterError>;
}

/// Port: executes a batch of tests and returns evidence (SPEC-043 §4.7).
///
/// Adapters implement this to run a semantic batch (produced by the kernel's
/// planner) against the concrete tooling and return a canonical [`TestEvidenceReceiptV1`].
pub trait VerificationExecutorPort: Send + Sync {
    /// Executes the given test batch and returns an evidence receipt.
    fn execute(&self, batch: &TestBatchV1) -> Result<TestEvidenceReceiptV1, AdapterError>;
}

/// Port: persists and retrieves test evidence receipts (SPEC-043 §4.8).
///
/// Adapters implement this to store receipts for later retrieval and comparison.
pub trait TestEvidenceRepository: Send + Sync {
    /// Persists an evidence receipt.
    fn save(&self, receipt: &TestEvidenceReceiptV1) -> Result<(), AdapterError>;

    /// Returns the latest evidence receipt for a given change-set digest and capability.
    fn latest_for(
        &self,
        change_set_digest: &str,
        capability_id: &str,
    ) -> Result<Option<TestEvidenceReceiptV1>, AdapterError>;
}

/// Port: exposes the current verification policy revision (SPEC-043 §4.9).
///
/// Adapters implement this to declare which policy revision governs test
/// selection and execution.
pub trait VerificationPolicyPort: Send + Sync {
    /// Returns the current verification policy revision string.
    fn policy_revision(&self) -> Result<String, AdapterError>;
}

// ── REQ-2: CapabilityRegistryV1 ───────────────────────────────────────────────

/// Schema version constant for capability registry.
pub const CAPABILITY_REGISTRY_SCHEMA_VERSION: u32 = 1;

/// The versioned capability registry (SPEC-043 §5).
///
/// Holds a deterministic BTreeMap of verification capabilities keyed by
/// capability_id. Iteration order is guaranteed by BTreeMap's sort order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CapabilityRegistryV1 {
    /// Schema version — must be `CAPABILITY_REGISTRY_SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Capabilities keyed by capability_id. BTreeMap ensures deterministic iteration.
    capabilities: BTreeMap<String, VerificationCapabilityV1>,
}

impl CapabilityRegistryV1 {
    /// Creates a new empty capability registry.
    pub fn new() -> Self {
        Self {
            schema_version: CAPABILITY_REGISTRY_SCHEMA_VERSION,
            capabilities: BTreeMap::new(),
        }
    }

    /// Registers a verification capability.
    ///
    /// Returns `Err(AdapterError::InvalidInput)` if a capability with the same
    /// `capability_id` is already registered.
    pub fn register(&mut self, cap: VerificationCapabilityV1) -> Result<(), AdapterError> {
        if self.capabilities.contains_key(&cap.capability_id) {
            return Err(AdapterError::InvalidInput {
                reason: format!("capability already registered: {}", cap.capability_id),
            });
        }
        self.capabilities.insert(cap.capability_id.clone(), cap);
        Ok(())
    }

    /// Returns the capability with the given id, or `None` if not found.
    pub fn get(&self, id: &str) -> Option<&VerificationCapabilityV1> {
        self.capabilities.get(id)
    }

    /// Returns all capabilities of the given kind, in deterministic BTreeMap order.
    pub fn by_kind(&self, kind: CapabilityKind) -> Vec<&VerificationCapabilityV1> {
        self.capabilities
            .values()
            .filter(|cap| cap.kind == kind)
            .collect()
    }

    /// Returns all capabilities that support the given SUT kind and granularity,
    /// in deterministic BTreeMap order.
    pub fn supporting(
        &self,
        sut_kind: SutKind,
        granularity: SelectorGranularity,
    ) -> Vec<&VerificationCapabilityV1> {
        self.capabilities
            .values()
            .filter(|cap| {
                cap.supported_sut_kinds.contains(&sut_kind)
                    && cap.selector_granularity == granularity
            })
            .collect()
    }

    /// Returns the number of registered capabilities.
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Returns `true` if the registry contains no capabilities.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("CapabilityRegistryV1 is always serializable")
    }

    /// Computes the content hash over the canonical JSON form.
    pub fn compute_content_hash(&self) -> ContentHash {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }
}

impl Default for CapabilityRegistryV1 {
    fn default() -> Self {
        Self::new()
    }
}

/// Versioned envelope for capability registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum CapabilityRegistry {
    /// Version 1 registry.
    V1(CapabilityRegistryV1),
}

// ── REQ-3: ProjectTestMapV1 (explicit YAML mapping) ───────────────────────────

/// Schema version constant for project test map.
pub const PROJECT_TEST_MAP_SCHEMA_VERSION: u32 = 1;

/// A mapped test within an explicit project mapping entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MappedTestV1 {
    /// Stable test identifier (non-empty).
    pub id: String,
    /// Kind of this test capability.
    pub kind: CapabilityKind,
}

/// An explicit mapping entry from a SUT node to its associated tests (SPEC-043 §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MappingEntryV1 {
    /// SUT node identifier this entry applies to (non-empty).
    pub sut: String,
    /// Explicit list of tests for this SUT (non-empty OR `affects` must be non-empty).
    pub tests: Vec<MappedTestV1>,
    /// SUT node IDs affected by changes to this SUT (used for propagation).
    pub affects: Vec<String>,
    /// Human-readable justification for this mapping (non-empty).
    pub reason: String,
}

/// Intermediate for two-pass YAML parsing.
///
/// `schema_version` uses `#[serde(default)]` so that a wrong/missing version
/// does not cause a deserialize error — we validate the value explicitly after
/// the first parse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RawProjectTestMapV1 {
    #[serde(default)]
    schema_version: u32,
    mappings: Vec<MappingEntryV1>,
}

/// The explicit project test mapping loaded from `.sddk/test-map.yaml` v1 (SPEC-043 §6).
///
/// This is the fallback surface for ecosystems that do not have a native adapter.
/// The kernel consults this after the registry to resolve test selections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectTestMapV1 {
    /// Schema version — must be `PROJECT_TEST_MAP_SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Individual mapping entries.
    pub mappings: Vec<MappingEntryV1>,
}

impl ProjectTestMapV1 {
    /// Schema version constant for v1 maps.
    pub const SCHEMA_VERSION: u32 = PROJECT_TEST_MAP_SCHEMA_VERSION;

    /// Parses a YAML string into a `ProjectTestMapV1`.
    ///
    /// Loader is pure: string in, validated struct out. No filesystem access.
    ///
    /// Strategy: two-pass with serde_saphyr.
    /// 1. Deserialize into `RawProjectTestMapV1` (schema_version has no validate, uses default).
    /// 2. Validate schema_version == 1.
    /// 3. Re-serialize to JSON and deserialize into `ProjectTestMapV1` (final validation in impl).
    /// 4. Run structural entry validation.
    pub fn from_yaml_str(yaml: &str) -> Result<ProjectTestMapV1, MapParseError> {
        // Pass 1: deserialize into intermediate that doesn't enforce schema_version validation
        let raw: RawProjectTestMapV1 =
            saphyr::from_str(yaml).map_err(|e| MapParseError::Yaml(e.to_string()))?;

        // Validate schema_version == 1 (SPEC: reject if != 1)
        if raw.schema_version != PROJECT_TEST_MAP_SCHEMA_VERSION {
            return Err(MapParseError::UnsupportedSchemaVersion {
                got: raw.schema_version,
            });
        }

        // Pass 2: re-serialize via serde_json (LosslessYaml compatible) → final parse
        let json_str = serde_json::to_string(&raw)
            .map_err(|e| MapParseError::Yaml(format!("serialization failed: {}", e)))?;

        let result: ProjectTestMapV1 = serde_json::from_str(&json_str)
            .map_err(|e| MapParseError::Yaml(format!("final parse failed: {}", e)))?;

        // Structural validation (empty reason, duplicate sut, etc.)
        result.validate_entries()?;

        Ok(result)
    }

    /// Validates structural constraints on mapping entries.
    fn validate_entries(&self) -> Result<(), MapParseError> {
        let mut seen_suts: BTreeSet<&str> = BTreeSet::new();

        for entry in &self.mappings {
            // Non-empty sut
            if entry.sut.is_empty() {
                return Err(MapParseError::EmptyEntry);
            }

            // Duplicate sut
            if !seen_suts.insert(entry.sut.as_str()) {
                return Err(MapParseError::DuplicateSut {
                    sut: entry.sut.clone(),
                });
            }

            // Non-empty reason
            if entry.reason.is_empty() {
                return Err(MapParseError::EmptyReason);
            }

            // At least one of tests or affects must be non-empty
            if entry.tests.is_empty() && entry.affects.is_empty() {
                return Err(MapParseError::EmptyEntry);
            }

            // Non-empty test ids
            for test in &entry.tests {
                if test.id.is_empty() {
                    return Err(MapParseError::Yaml("test id cannot be empty".into()));
                }
            }
        }

        Ok(())
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("ProjectTestMapV1 is always serializable")
    }

    /// Computes the content hash over the canonical JSON form.
    pub fn compute_content_hash(&self) -> ContentHash {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }
}

/// Versioned envelope for project test map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum ProjectTestMap {
    /// Version 1 project test map.
    V1(ProjectTestMapV1),
}

/// Closed errors for parsing project test map YAML.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum MapParseError {
    /// The YAML file has a schema_version that is not 1.
    #[error("unsupported schema version: got {got}, want 1")]
    UnsupportedSchemaVersion {
        /// The schema version found in the YAML.
        got: u32,
    },

    /// A mapping entry has an empty reason field.
    #[error("mapping entry has empty reason")]
    EmptyReason,

    /// A mapping entry has an empty tests list AND empty affects list.
    #[error("mapping entry must have at least one test or affects entry")]
    EmptyEntry,

    /// Two mapping entries target the same sut node id.
    #[error("duplicate sut in mappings: {sut}")]
    DuplicateSut {
        /// The duplicate SUT identifier.
        sut: String,
    },

    /// The YAML could not be parsed.
    #[error("YAML parse error: {0}")]
    Yaml(String),
}

crate::assert_variant_count_eq!(
    MapParseError,
    5,
    [
        MapParseError::UnsupportedSchemaVersion { .. },
        MapParseError::EmptyReason,
        MapParseError::EmptyEntry,
        MapParseError::DuplicateSut { .. },
        MapParseError::Yaml(..),
    ]
);

// ── REQ-4: FallbackResolverV1 ─────────────────────────────────────────────────

/// Outcome of the fallback resolver — typed absence, never fabricated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionOutcome {
    /// Resolution succeeded via the capability registry.
    RegistryCapability {
        /// The capability id from the registry.
        capability_id: String,
    },
    /// Resolution succeeded via explicit mapping.
    ExplicitMapping {
        /// The mapping entry that resolved the SUT.
        entry: MappingEntryV1,
    },
    /// Neither registry nor explicit map could resolve this SUT.
    Unresolved {
        /// The SUT node id that could not be resolved.
        sut: String,
    },
}

crate::assert_variant_count_eq!(
    ResolutionOutcome,
    3,
    [
        ResolutionOutcome::RegistryCapability { .. },
        ResolutionOutcome::ExplicitMapping { .. },
        ResolutionOutcome::Unresolved { .. },
    ]
);

/// The fallback resolver — fail-closed resolution of test capabilities (SPEC-043 §6).
///
/// Precedence:
/// 1. Registry capability supporting (kind, granularity) for the given SUT node.
/// 2. Explicit mapping entry for the given sut node id.
/// 3. `Unresolved` — typed absence, never invented.
#[derive(Debug, Clone, Default)]
pub struct FallbackResolverV1;

impl FallbackResolverV1 {
    /// Creates a new fallback resolver.
    pub fn new() -> Self {
        Self
    }

    /// Resolves the best capability for a given SUT node.
    ///
    /// Returns `RegistryCapability` if the registry has a supporting capability;
    /// otherwise returns `ExplicitMapping` if the explicit map has an entry;
    /// otherwise returns `Unresolved`. Never fabricates a result.
    pub fn resolve(
        &self,
        registry: &CapabilityRegistryV1,
        map: &ProjectTestMapV1,
        sut_node_id: &str,
        kind: SutKind,
        granularity: SelectorGranularity,
    ) -> ResolutionOutcome {
        // Precedence 1: registry
        let caps = registry.supporting(kind, granularity);
        if let Some(cap) = caps.first() {
            return ResolutionOutcome::RegistryCapability {
                capability_id: cap.capability_id.clone(),
            };
        }

        // Precedence 2: explicit mapping
        for entry in &map.mappings {
            if entry.sut == sut_node_id {
                return ResolutionOutcome::ExplicitMapping {
                    entry: entry.clone(),
                };
            }
        }

        // Precedence 3: typed absence
        ResolutionOutcome::Unresolved {
            sut: sut_node_id.to_string(),
        }
    }
}

// ── REQ-6: Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6.1: registry tests
    // ═══════════════════════════════════════════════════════════════════════════

    fn make_cap(
        id: &str,
        kind: CapabilityKind,
        sut_kinds: &[SutKind],
        granularity: SelectorGranularity,
    ) -> VerificationCapabilityV1 {
        VerificationCapabilityV1::new(
            id.to_string(),
            kind,
            sut_kinds.iter().cloned().collect(),
            granularity,
            "test-adapter".to_string(),
            "test-toolchain".to_string(),
            None,
            vec![],
        )
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = CapabilityRegistryV1::new();
        let cap = make_cap(
            "cap-1",
            CapabilityKind::Unit,
            &[SutKind::BuildUnit],
            SelectorGranularity::BuildUnit,
        );
        assert!(reg.register(cap.clone()).is_ok());
        assert_eq!(
            reg.get("cap-1").map(|c| c.capability_id.as_str()),
            Some("cap-1")
        );
        assert!(reg.get("cap-nonexistent").is_none());
    }

    #[test]
    fn registry_duplicate_rejects() {
        let mut reg = CapabilityRegistryV1::new();
        let cap = make_cap(
            "cap-dup",
            CapabilityKind::Unit,
            &[SutKind::BuildUnit],
            SelectorGranularity::BuildUnit,
        );
        assert!(reg.register(cap.clone()).is_ok());
        let result = reg.register(cap);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AdapterError::InvalidInput { .. }
        ));
    }

    #[test]
    fn registry_deterministic_ordering() {
        let mut reg = CapabilityRegistryV1::new();
        let kinds = [
            CapabilityKind::Unit,
            CapabilityKind::Compile,
            CapabilityKind::Lint,
        ];
        for (i, kind) in kinds.iter().enumerate() {
            let cap = make_cap(
                &format!("cap-{:03}", i),
                *kind,
                &[SutKind::BuildUnit],
                SelectorGranularity::BuildUnit,
            );
            reg.register(cap).unwrap();
        }
        let caps = reg.by_kind(CapabilityKind::Unit);
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].capability_id.as_str(), "cap-000"); // index 0 = Unit

        let supporting = reg.supporting(SutKind::BuildUnit, SelectorGranularity::BuildUnit);
        assert_eq!(supporting.len(), 3);
    }

    #[test]
    fn registry_len_and_is_empty() {
        let mut reg = CapabilityRegistryV1::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        let cap = make_cap(
            "cap-1",
            CapabilityKind::Unit,
            &[SutKind::BuildUnit],
            SelectorGranularity::BuildUnit,
        );
        reg.register(cap).unwrap();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6.2: YAML v1 parsing
    // ═══════════════════════════════════════════════════════════════════════════

    const VALID_YAML_V1: &str = r#"
schema_version: 1
mappings:
  - sut: "src/lib.rs"
    tests:
      - id: "test_unit_a"
        kind: unit
      - id: "test_unit_b"
        kind: unit
    affects: ["src/main.rs"]
    reason: "Unit tests for lib.rs"
  - sut: "src/main.rs"
    tests:
      - id: "test_integration_x"
        kind: integration
    affects: []
    reason: "Integration tests for main"
"#;

    #[test]
    fn yaml_v1_parses_valid() {
        let result = ProjectTestMapV1::from_yaml_str(VALID_YAML_V1);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let map = result.unwrap();
        assert_eq!(map.schema_version, 1);
        assert_eq!(map.mappings.len(), 2);
        assert_eq!(map.mappings[0].sut.as_str(), "src/lib.rs");
        assert_eq!(map.mappings[0].tests.len(), 2);
        assert_eq!(map.mappings[0].reason.as_str(), "Unit tests for lib.rs");
    }

    #[test]
    fn yaml_v1_rejects_wrong_schema_version() {
        let yaml = r#"
schema_version: 2
mappings: []
"#;
        let result = ProjectTestMapV1::from_yaml_str(yaml);
        assert!(matches!(
            result,
            Err(MapParseError::UnsupportedSchemaVersion { got: 2 })
        ));
    }

    #[test]
    fn yaml_v1_rejects_empty_reason() {
        let yaml = r#"
schema_version: 1
mappings:
  - sut: "src/lib.rs"
    tests:
      - id: "test_a"
        kind: unit
    affects: []
    reason: ""
"#;
        let result = ProjectTestMapV1::from_yaml_str(yaml);
        assert!(matches!(result, Err(MapParseError::EmptyReason)));
    }

    #[test]
    fn yaml_v1_rejects_empty_entry() {
        let yaml = r#"
schema_version: 1
mappings:
  - sut: "src/lib.rs"
    tests: []
    affects: []
    reason: "some reason"
"#;
        let result = ProjectTestMapV1::from_yaml_str(yaml);
        assert!(matches!(result, Err(MapParseError::EmptyEntry)));
    }

    #[test]
    fn yaml_v1_rejects_duplicate_sut() {
        let yaml = r#"
schema_version: 1
mappings:
  - sut: "src/lib.rs"
    tests:
      - id: "test_a"
        kind: unit
    affects: []
    reason: "first"
  - sut: "src/lib.rs"
    tests:
      - id: "test_b"
        kind: unit
    affects: []
    reason: "second"
"#;
        let result = ProjectTestMapV1::from_yaml_str(yaml);
        assert!(matches!(result, Err(MapParseError::DuplicateSut { sut }) if sut == "src/lib.rs"));
    }

    #[test]
    fn yaml_v1_rejects_malformed_yaml() {
        let yaml = "not: valid: yaml: : :";
        let result = ProjectTestMapV1::from_yaml_str(yaml);
        assert!(matches!(result, Err(MapParseError::Yaml(_))));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6.3: resolution precedence
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn resolution_registry_precedence_over_map() {
        let mut registry = CapabilityRegistryV1::new();
        let cap = make_cap(
            "reg-cap",
            CapabilityKind::Unit,
            &[SutKind::BuildUnit],
            SelectorGranularity::BuildUnit,
        );
        registry.register(cap).unwrap();

        let map = ProjectTestMapV1::from_yaml_str(VALID_YAML_V1).unwrap();
        let resolver = FallbackResolverV1::new();

        let outcome = resolver.resolve(
            &registry,
            &map,
            "src/lib.rs",
            SutKind::BuildUnit,
            SelectorGranularity::BuildUnit,
        );

        match outcome {
            ResolutionOutcome::RegistryCapability { capability_id } => {
                assert_eq!(capability_id.as_str(), "reg-cap");
            }
            other => panic!("expected RegistryCapability, got {:?}", other),
        }
    }

    #[test]
    fn resolution_map_fallback_for_unsupported_ecosystem() {
        let registry = CapabilityRegistryV1::new(); // empty
        let map = ProjectTestMapV1::from_yaml_str(VALID_YAML_V1).unwrap();
        let resolver = FallbackResolverV1::new();

        let outcome = resolver.resolve(
            &registry,
            &map,
            "src/lib.rs",
            SutKind::BuildUnit,
            SelectorGranularity::BuildUnit,
        );

        match outcome {
            ResolutionOutcome::ExplicitMapping { entry } => {
                assert_eq!(entry.sut.as_str(), "src/lib.rs");
            }
            other => panic!("expected ExplicitMapping, got {:?}", other),
        }
    }

    #[test]
    fn resolution_unresolved_when_neither() {
        let registry = CapabilityRegistryV1::new();
        let map = ProjectTestMapV1::from_yaml_str(VALID_YAML_V1).unwrap();
        let resolver = FallbackResolverV1::new();

        let outcome = resolver.resolve(
            &registry,
            &map,
            "unknown/sut.rs",
            SutKind::BuildUnit,
            SelectorGranularity::BuildUnit,
        );

        match outcome {
            ResolutionOutcome::Unresolved { sut } => {
                assert_eq!(sut.as_str(), "unknown/sut.rs");
            }
            other => panic!("expected Unresolved, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6.4: object-safety smoke (dyn trait usage)
    // ═══════════════════════════════════════════════════════════════════════════

    /// A smoke test that uses &dyn trait object to verify object safety.
    #[derive(Debug)]
    struct FakeChangeSetAdapter {
        change_set: ActiveChangeSetV1,
    }

    impl FakeChangeSetAdapter {
        fn new() -> Self {
            Self {
                change_set: ActiveChangeSetV1::new(
                    "test-project".to_string(),
                    "base".to_string(),
                    "head".to_string(),
                    "tree".to_string(),
                    vec![],
                ),
            }
        }
    }

    impl ActiveChangeSetPort for FakeChangeSetAdapter {
        fn active_change_set(&self) -> Result<ActiveChangeSetV1, AdapterError> {
            Ok(self.change_set.clone())
        }
    }

    /// A smoke test that uses &dyn VerificationExecutorPort to verify object safety.
    #[derive(Debug)]
    struct FakeExecutor;

    impl VerificationExecutorPort for FakeExecutor {
        fn execute(&self, _batch: &TestBatchV1) -> Result<TestEvidenceReceiptV1, AdapterError> {
            Ok(TestEvidenceReceiptV1::new(
                "receipt-1".to_string(),
                "cs-digest".to_string(),
                "rev".to_string(),
                "topo-rev".to_string(),
                "sut-rev".to_string(),
                "policy-rev".to_string(),
                "cap-1".to_string(),
                crate::test_model::ReceiptResult::Passed,
                "2024-01-01T00:00:00Z".to_string(),
            ))
        }
    }

    #[test]
    fn object_safety_active_change_set_port() {
        let adapter = FakeChangeSetAdapter::new();
        let port: &dyn ActiveChangeSetPort = &adapter;
        let result = port.active_change_set();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().project_id.as_str(), "test-project");
    }

    #[test]
    fn object_safety_verification_executor_port() {
        let executor = FakeExecutor;
        let port: &dyn VerificationExecutorPort = &executor;
        let batch = TestBatchV1 {
            stage: 1,
            capability_id: "cap-1".to_string(),
            semantic_scope: vec![],
            test_ids: vec!["test-1".to_string()],
            reasons: vec![crate::test_model::ImpactReason::DirectSourceTouch],
            expected_cost: None,
            escalation: false,
        };
        let result = port.execute(&batch);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().receipt_id.as_str(), "receipt-1");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6.5: canonical byte-stability + hash
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn registry_canonical_hash_deterministic() {
        let mut reg1 = CapabilityRegistryV1::new();
        let cap1 = make_cap(
            "cap-a",
            CapabilityKind::Unit,
            &[SutKind::BuildUnit, SutKind::Component],
            SelectorGranularity::BuildUnit,
        );
        reg1.register(cap1).unwrap();

        let mut reg2 = CapabilityRegistryV1::new();
        let cap2 = make_cap(
            "cap-a",
            CapabilityKind::Unit,
            &[SutKind::BuildUnit, SutKind::Component],
            SelectorGranularity::BuildUnit,
        );
        reg2.register(cap2).unwrap();

        // Same contents → same hash
        assert_eq!(reg1.compute_content_hash(), reg2.compute_content_hash());

        // Adding another capability changes the hash
        let cap3 = make_cap(
            "cap-b",
            CapabilityKind::Lint,
            &[SutKind::BuildUnit],
            SelectorGranularity::BuildUnit,
        );
        reg2.register(cap3).unwrap();
        assert_ne!(reg1.compute_content_hash(), reg2.compute_content_hash());
    }

    #[test]
    fn map_canonical_hash_deterministic() {
        let map1 = ProjectTestMapV1::from_yaml_str(VALID_YAML_V1).unwrap();
        let map2 = ProjectTestMapV1::from_yaml_str(VALID_YAML_V1).unwrap();
        assert_eq!(map1.compute_content_hash(), map2.compute_content_hash());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-6.6: variant-count assertions for closed enums
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn adapter_error_variant_count() {
        let variants = [
            AdapterError::Unavailable {
                reason: String::new(),
            },
            AdapterError::DetectionFailed {
                reason: String::new(),
            },
            AdapterError::InvalidInput {
                reason: String::new(),
            },
        ];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn map_parse_error_variant_count() {
        use MapParseError::*;
        let variants = [
            UnsupportedSchemaVersion { got: 0 },
            EmptyReason,
            EmptyEntry,
            DuplicateSut { sut: String::new() },
            Yaml(String::new()),
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn resolution_outcome_variant_count() {
        let variants = [
            ResolutionOutcome::RegistryCapability {
                capability_id: String::new(),
            },
            ResolutionOutcome::ExplicitMapping {
                entry: MappingEntryV1 {
                    sut: String::new(),
                    tests: vec![],
                    affects: vec![],
                    reason: String::new(),
                },
            },
            ResolutionOutcome::Unresolved { sut: String::new() },
        ];
        assert_eq!(variants.len(), 3);
    }
}
