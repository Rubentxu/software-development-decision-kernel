//! Declarative ecosystem profiles and ProfileAdapterV1 (SPEC-043 §5/§6/§7).
//!
//! This module provides:
//! - `EcosystemProfileV1`: declarative, data-only ecosystem description (no kernel code).
//! - `ProfileAdapterV1<'a>`: pure adapter over an injected manifest snapshot (no fs/proc).
//! - `compose_polyglot_topology()`: deterministic merge of multi-ecosystem topologies.
//!
//! Design:
//! - Zero filesystem access — all manifest content comes from a `&BTreeMap<String, String>` snapshot.
//! - Closed validation errors: `EmptyField` and `CapabilityAdapterMismatch`.
//! - Adapter NEVER returns selection data — only topology and capabilities (per SPEC-043 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::test_model::{
    ActiveChangeSetV1, ChangedArtifactV1, EdgeProvenanceV1, ProjectTestTopologyV1, SutKind,
    SutNodeV1, TopologyEdgeKind, TopologyEdgeV1, VerificationCapabilityV1,
};

use crate::test_ports::AdapterError;

/// Content hash in `sha256:<64-hex-lowercase>` format.
type ContentHash = String;

// ── REQ-1: EcosystemProfile ────────────────────────────────────────────────────

/// Schema version constant for ecosystem profile.
pub const ECOSYSTEM_PROFILE_SCHEMA_VERSION: u32 = 1;

/// A declarative ecosystem profile — data, not kernel code (SPEC-043 §5).
///
/// Describes how to detect an ecosystem from its manifest file and what
/// verification capabilities its adapter provides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EcosystemProfileV1 {
    /// Schema version — must be `ECOSYSTEM_PROFILE_SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Unique adapter identifier (non-empty).
    pub adapter_id: String,
    /// Ecosystem identifier e.g. "rust", "typescript", "python" (non-empty).
    pub ecosystem: String,
    /// Path to the ecosystem manifest file (non-empty, e.g. "Cargo.toml").
    pub manifest_path: String,
    /// Substrings that MUST appear in the manifest content for detection (non-empty vec).
    pub markers: Vec<String>,
    /// Node id for the component root of this ecosystem (non-empty).
    pub component_node_id: String,
    /// Verification capabilities provided by this ecosystem's adapter (non-empty).
    /// Each capability's `adapter_id` MUST match this profile's `adapter_id`.
    pub capabilities: Vec<VerificationCapabilityV1>,
}

/// Versioned envelope for ecosystem profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum EcosystemProfile {
    /// Version 1 profile.
    V1(EcosystemProfileV1),
}

/// Closed validation errors for ecosystem profile.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum EcosystemProfileError {
    /// A required field is empty.
    #[error("profile field is empty: {field}")]
    EmptyField {
        /// The name of the empty field.
        field: String,
    },

    /// A capability's adapter_id does not match the profile's adapter_id.
    #[error(
        "capability adapter mismatch: expected '{}', got '{}' in capability '{}'",
        expected,
        got,
        capability_id
    )]
    CapabilityAdapterMismatch {
        /// The adapter_id expected (from the profile).
        expected: String,
        /// The adapter_id found in the capability.
        got: String,
        /// The capability_id whose adapter_id mismatched.
        capability_id: String,
    },
}

crate::assert_variant_count_eq!(
    EcosystemProfileError,
    2,
    [
        EcosystemProfileError::EmptyField { .. },
        EcosystemProfileError::CapabilityAdapterMismatch { .. },
    ]
);

impl EcosystemProfileV1 {
    /// Validates this profile instance — closed errors per REQ-1.
    pub fn validate(&self) -> Result<(), EcosystemProfileError> {
        if self.schema_version != ECOSYSTEM_PROFILE_SCHEMA_VERSION {
            return Err(EcosystemProfileError::EmptyField {
                field: "schema_version".to_string(),
            });
        }
        if self.adapter_id.is_empty() {
            return Err(EcosystemProfileError::EmptyField {
                field: "adapter_id".to_string(),
            });
        }
        if self.ecosystem.is_empty() {
            return Err(EcosystemProfileError::EmptyField {
                field: "ecosystem".to_string(),
            });
        }
        if self.manifest_path.is_empty() {
            return Err(EcosystemProfileError::EmptyField {
                field: "manifest_path".to_string(),
            });
        }
        if self.markers.is_empty() {
            return Err(EcosystemProfileError::EmptyField {
                field: "markers".to_string(),
            });
        }
        if self.component_node_id.is_empty() {
            return Err(EcosystemProfileError::EmptyField {
                field: "component_node_id".to_string(),
            });
        }
        if self.capabilities.is_empty() {
            return Err(EcosystemProfileError::EmptyField {
                field: "capabilities".to_string(),
            });
        }
        for cap in &self.capabilities {
            if cap.adapter_id != self.adapter_id {
                return Err(EcosystemProfileError::CapabilityAdapterMismatch {
                    expected: self.adapter_id.clone(),
                    got: cap.adapter_id.clone(),
                    capability_id: cap.capability_id.clone(),
                });
            }
        }
        Ok(())
    }
}

// ── REQ-2: ProfileAdapterV1 ────────────────────────────────────────────────────

/// Profile adapter over a pure manifest snapshot (SPEC-043 §5).
///
/// Constructed with `&EcosystemProfileV1` + `&BTreeMap<String, String>` (path → content).
/// Implements `ActiveChangeSetPort` and `ProjectTopologyPort` without any filesystem
/// or process access.
///
/// The adapter version is pinned to the profile schema version for provenance clarity.
#[derive(Debug, Clone)]
pub struct ProfileAdapterV1<'a> {
    /// The ecosystem profile this adapter applies.
    profile: &'a EcosystemProfileV1,
    /// Injected manifest snapshot: path → content. No fs access.
    snapshot: &'a BTreeMap<String, String>,
}

impl<'a> ProfileAdapterV1<'a> {
    /// Creates a new profile adapter from a profile and manifest snapshot.
    pub fn new(profile: &'a EcosystemProfileV1, snapshot: &'a BTreeMap<String, String>) -> Self {
        Self { profile, snapshot }
    }

    /// Returns the adapter version string for provenance.
    fn adapter_version(&self) -> String {
        format!("profile-adapter-v{}", self.profile.schema_version)
    }

    /// Computes a deterministic digest for a manifest's content.
    ///
    /// Uses canonical JSON serialization of a minimal artifact record to produce
    /// a stable `sha256:<hex>` digest that changes iff the manifest content changes.
    fn compute_manifest_digest(content: &str) -> ContentHash {
        // Canonical form: { "path": <path>, "content": <content> } in sorted key order
        let json = serde_json::json!({
            "content": content,
        });
        let digest = Sha256::digest(json.to_string().as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }

    /// Returns the manifest content for this profile's manifest path, if present.
    fn manifest_content(&self) -> Option<&String> {
        self.snapshot.get(&self.profile.manifest_path)
    }

    /// Returns `true` if ALL markers appear in the manifest content.
    fn all_markers_present(&self, content: &str) -> bool {
        self.profile.markers.iter().all(|m| content.contains(m))
    }
}

impl<'a> crate::test_ports::ActiveChangeSetPort for ProfileAdapterV1<'a> {
    /// Returns an `ActiveChangeSetV1` whose `changed_artifacts` covers the profile's
    /// manifest path with `ChangeKind::Modified`, driven entirely by the snapshot.
    fn active_change_set(&self) -> Result<ActiveChangeSetV1, AdapterError> {
        let manifest_path = &self.profile.manifest_path;
        let content = self
            .manifest_content()
            .ok_or_else(|| AdapterError::Unavailable {
                reason: format!("manifest not found in snapshot: {}", manifest_path),
            })?;

        let digest = Self::compute_manifest_digest(content);

        let changed = ChangedArtifactV1 {
            path: manifest_path.clone(),
            change_kind: crate::test_model::ChangeKind::Modified,
            staged: false,
        };

        let change_set = ActiveChangeSetV1::new(
            // project_id is derived from the ecosystem profile identifier
            format!("profile:{}", self.profile.adapter_id),
            // base/head revisions are synthetic but stable
            format!("base:{}", digest),
            format!("head:{}", digest),
            digest,
            vec![changed],
        );

        Ok(change_set)
    }
}

impl<'a> crate::test_ports::ProjectTopologyPort for ProfileAdapterV1<'a> {
    /// Returns a `ProjectTestTopologyV1` if the manifest is present and all markers
    /// are detected in its content.
    ///
    /// Topology structure:
    /// - One `Component` node (the ecosystem component root).
    /// - One `SourceArtifact` node (the manifest itself).
    /// - One `VerificationCapability` node per declared capability.
    /// - Edges: `Owns` (component → manifest), `UsesCapability` (component → each capability).
    fn topology(&self) -> Result<ProjectTestTopologyV1, AdapterError> {
        let manifest_path = &self.profile.manifest_path;

        let content = self
            .manifest_content()
            .ok_or_else(|| AdapterError::Unavailable {
                reason: format!("manifest not found in snapshot: {}", manifest_path),
            })?;

        if !self.all_markers_present(content) {
            return Err(AdapterError::DetectionFailed {
                reason: format!(
                    "not all markers found in {}: {:?}",
                    manifest_path, self.profile.markers
                ),
            });
        }

        let prov = EdgeProvenanceV1 {
            source: "profile-adapter".to_string(),
            adapter_version: self.adapter_version(),
            confidence_source: "manifest-marker".to_string(),
        };

        // Node ids (deterministic BTreeMap key order via insertion below)
        let component_id = &self.profile.component_node_id;
        let manifest_node_id = format!("{}:manifest", component_id);

        // Build nodes BTreeMap — insertion order below determines key-order.
        // BTreeMap requires unique keys; we build in stages.
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();

        // Component node
        nodes.insert(
            component_id.clone(),
            SutNodeV1::new(
                component_id.clone(),
                SutKind::Component,
                self.profile.ecosystem.clone(),
                Some(manifest_path.clone()),
            ),
        );

        // Manifest-as-source-artifact node
        nodes.insert(
            manifest_node_id.clone(),
            SutNodeV1::new(
                manifest_node_id.clone(),
                SutKind::SourceArtifact,
                String::new(), // source artifact is ecosystem-neutral
                Some(manifest_path.clone()),
            ),
        );

        // Capability nodes + edges
        let mut edges: Vec<TopologyEdgeV1> =
            Vec::with_capacity(self.profile.capabilities.len() + 1);

        // Edge: component Owns manifest
        edges.push(TopologyEdgeV1::new(
            TopologyEdgeKind::Owns,
            component_id.clone(),
            manifest_node_id.clone(),
            prov.clone(),
        ));

        for cap in &self.profile.capabilities {
            let cap_node_id = format!("{}:cap:{}", component_id, cap.capability_id);
            nodes.insert(
                cap_node_id.clone(),
                SutNodeV1::new(
                    cap_node_id.clone(),
                    SutKind::VerificationCapability,
                    String::new(), // capability node is ecosystem-neutral
                    Some(cap.capability_id.clone()),
                ),
            );

            edges.push(TopologyEdgeV1::new(
                TopologyEdgeKind::UsesCapability,
                component_id.clone(),
                cap_node_id,
                prov.clone(),
            ));
        }

        let topology_revision = format!(
            "profile:{}:v{}",
            self.profile.adapter_id, self.profile.schema_version
        );

        let topology = ProjectTestTopologyV1::new(topology_revision, nodes, edges);
        topology.validate().map_err(|e| {
            // Validate should not fail for adapter-produced topologies, but be safe
            AdapterError::Unavailable {
                reason: format!("topology validation failed: {}", e),
            }
        })?;

        Ok(topology)
    }
}

// ── REQ-3: compose_polyglot_topology ─────────────────────────────────────────

/// Merges multiple ecosystem topologies into a single polyglot topology (SPEC-043 §6).
///
/// Rules:
/// - Duplicate `node_id` → `AdapterError::InvalidInput`.
/// - Deterministic ordering via BTreeMap (sorted by node_id).
/// - Resulting `topology_revision` = `polyglot:<n>-ecosystems`.
/// - Validates the merged topology before returning.
pub fn compose_polyglot_topology(
    topologies: Vec<ProjectTestTopologyV1>,
) -> Result<ProjectTestTopologyV1, AdapterError> {
    if topologies.is_empty() {
        return Err(AdapterError::InvalidInput {
            reason: "at least one topology required".to_string(),
        });
    }

    let n = topologies.len();
    let mut merged_nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
    let mut merged_edges: Vec<TopologyEdgeV1> = Vec::new();

    for topo in &topologies {
        for (node_id, node) in &topo.nodes {
            if merged_nodes.contains_key(node_id) {
                return Err(AdapterError::InvalidInput {
                    reason: format!("duplicate node_id across ecosystems: {}", node_id),
                });
            }
            merged_nodes.insert(node_id.clone(), node.clone());
        }
        merged_edges.extend_from_slice(&topo.edges);
    }

    let topology_revision = format!("polyglot:{}-ecosystems", n);

    let merged = ProjectTestTopologyV1::new(topology_revision, merged_nodes, merged_edges);
    merged.validate().map_err(|e| AdapterError::InvalidInput {
        reason: format!("merged topology validation failed: {}", e),
    })?;

    Ok(merged)
}

// ── REQ-7: Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_model::{CapabilityKind, SelectorGranularity};
    use crate::test_ports::{
        ActiveChangeSetPort, CapabilityRegistryV1, FallbackResolverV1, ProjectTestMapV1,
        ProjectTopologyPort, ResolutionOutcome,
    };

    // ═══════════════════════════════════════════════════════════════════════════
    // UAT Fixtures — minimal realistic manifest contents
    // ═══════════════════════════════════════════════════════════════════════════

    const FIXTURE_CARGO_TOML: &str = r#"[package]
name = "mycrate"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"

[[test]]
name = "it_works"
path = "tests/it_works.rs"
"#;

    const FIXTURE_PACKAGE_JSON: &str = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "test": "vitest run",
    "build": "tsc"
  },
  "devDependencies": {
    "vitest": "^1.0.0"
  }
}
"#;

    const FIXTURE_PYPROJECT_TOML: &str = r#"[project]
name = "mypackage"
version = "0.1.0"
requires-python = ">=3.10"

[project.optional-dependencies]
dev = ["pytest", "pytest-cov"]

[tool.pytest.ini_options]
testpaths = ["tests"]
"#;

    const FIXTURE_GO_MOD: &str = r#"module mymodule

go 1.22

require (
	github.com/foo/bar v1.2.3
)
"#;

    // ═══════════════════════════════════════════════════════════════════════════
    // Helper: build a minimal EcosystemProfileV1 for testing
    // ═══════════════════════════════════════════════════════════════════════════

    fn make_profile(
        adapter_id: &str,
        ecosystem: &str,
        manifest_path: &str,
        markers: Vec<&str>,
        component_node_id: &str,
    ) -> EcosystemProfileV1 {
        let marker_strs: Vec<String> = markers.iter().map(|s| (*s).to_string()).collect();
        EcosystemProfileV1 {
            schema_version: ECOSYSTEM_PROFILE_SCHEMA_VERSION,
            adapter_id: adapter_id.to_string(),
            ecosystem: ecosystem.to_string(),
            manifest_path: manifest_path.to_string(),
            markers: marker_strs,
            component_node_id: component_node_id.to_string(),
            capabilities: vec![],
        }
    }

    fn make_cap(cap_id: &str, adapter_id: &str, kind: CapabilityKind) -> VerificationCapabilityV1 {
        VerificationCapabilityV1::new(
            cap_id.to_string(),
            kind,
            [SutKind::Component, SutKind::BuildUnit]
                .iter()
                .cloned()
                .collect(),
            SelectorGranularity::BuildUnit,
            adapter_id.to_string(),
            format!("{} toolchain", adapter_id),
            None,
            vec![],
        )
    }

    fn snapshot_from<'a>(entries: Vec<(&'a str, &'a str)>) -> BTreeMap<String, String> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-4.1 — Rust alone
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn uat_rust_alone_detects_cargo_toml() {
        let mut profile = make_profile(
            "rust-adapter",
            "rust",
            "Cargo.toml",
            vec!["[package]", "name = "],
            "component:mycrate",
        );
        profile.capabilities = vec![
            make_cap("rust:compile", "rust-adapter", CapabilityKind::Compile),
            make_cap("rust:unit", "rust-adapter", CapabilityKind::Unit),
        ];

        let snap = snapshot_from(vec![("Cargo.toml", FIXTURE_CARGO_TOML)]);
        let adapter = ProfileAdapterV1::new(&profile, &snap);

        // ActiveChangeSet
        let cs = adapter
            .active_change_set()
            .expect("active_change_set must succeed");
        assert_eq!(cs.project_id, "profile:rust-adapter");
        assert_eq!(cs.changed_artifacts.len(), 1);
        assert_eq!(cs.changed_artifacts[0].path, "Cargo.toml");
        assert_eq!(
            cs.changed_artifacts[0].change_kind,
            crate::test_model::ChangeKind::Modified
        );

        // Topology
        let topo = adapter.topology().expect("topology must succeed");
        assert_eq!(topo.nodes.len(), 4); // component + manifest + 2 caps
        assert!(topo.nodes.contains_key("component:mycrate"));
        assert!(
            topo.nodes
                .contains_key("component:mycrate:cap:rust:compile")
        );
        assert!(topo.nodes.contains_key("component:mycrate:cap:rust:unit"));

        // Edges: component Owns manifest, component UsesCapability for each cap
        let cap_edges: Vec<_> = topo
            .edges
            .iter()
            .filter(|e| e.edge_kind == TopologyEdgeKind::UsesCapability)
            .collect();
        assert_eq!(cap_edges.len(), 2);

        // Provenance
        for edge in &topo.edges {
            assert_eq!(edge.provenance.source, "profile-adapter");
            assert_eq!(edge.provenance.confidence_source, "manifest-marker");
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-4.2 — Two contrasting non-Rust (TypeScript + Python)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn uat_typescript_detects_package_json() {
        let mut profile = make_profile(
            "ts-adapter",
            "typescript",
            "package.json",
            vec!["\"name\":", "\"version\":"],
            "component:my-package",
        );
        profile.capabilities = vec![
            make_cap("ts:typecheck", "ts-adapter", CapabilityKind::TypeCheck),
            make_cap("ts:unit", "ts-adapter", CapabilityKind::Unit),
        ];

        let snap = snapshot_from(vec![("package.json", FIXTURE_PACKAGE_JSON)]);
        let adapter = ProfileAdapterV1::new(&profile, &snap);

        let topo = adapter
            .topology()
            .expect("typescript topology must succeed");
        assert_eq!(topo.nodes.len(), 4); // component + manifest + 2 caps
        assert!(topo.nodes.contains_key("component:my-package"));
        let comp_node = topo.nodes.get("component:my-package").unwrap();
        assert_eq!(comp_node.ecosystem, "typescript");
    }

    #[test]
    fn uat_python_detects_pyproject_toml() {
        let mut profile = make_profile(
            "python-adapter",
            "python",
            "pyproject.toml",
            vec!["[project]", "name = "],
            "component:mypackage",
        );
        profile.capabilities = vec![
            make_cap("py:compile", "python-adapter", CapabilityKind::Compile),
            make_cap("py:unit", "python-adapter", CapabilityKind::Unit),
        ];

        let snap = snapshot_from(vec![("pyproject.toml", FIXTURE_PYPROJECT_TOML)]);
        let adapter = ProfileAdapterV1::new(&profile, &snap);

        let topo = adapter.topology().expect("python topology must succeed");
        assert_eq!(topo.nodes.len(), 4);
        assert!(topo.nodes.contains_key("component:mypackage"));
        let comp_node = topo.nodes.get("component:mypackage").unwrap();
        assert_eq!(comp_node.ecosystem, "python");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-4.3 — Polyglot + cross-language contract edge
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn uat_polyglot_merges_and_cross_language_contract_edge() {
        // Build individual topologies
        let mut rust_profile = make_profile(
            "rust-adapter",
            "rust",
            "Cargo.toml",
            vec!["[package]", "name = "],
            "component:mycrate",
        );
        rust_profile.capabilities =
            vec![make_cap("rust:unit", "rust-adapter", CapabilityKind::Unit)];

        let mut ts_profile = make_profile(
            "ts-adapter",
            "typescript",
            "package.json",
            vec!["\"name\":", "\"version\":"],
            "component:web-client",
        );
        ts_profile.capabilities = vec![make_cap(
            "ts:contract",
            "ts-adapter",
            CapabilityKind::Contract,
        )];

        let rust_snap = snapshot_from(vec![("Cargo.toml", FIXTURE_CARGO_TOML)]);
        let ts_snap = snapshot_from(vec![("package.json", FIXTURE_PACKAGE_JSON)]);

        let rust_adapter = ProfileAdapterV1::new(&rust_profile, &rust_snap);
        let ts_adapter = ProfileAdapterV1::new(&ts_profile, &ts_snap);

        let rust_topo = rust_adapter.topology().expect("rust topology");
        let ts_topo = ts_adapter.topology().expect("ts topology");

        // Compose polyglot
        let merged = compose_polyglot_topology(vec![rust_topo.clone(), ts_topo.clone()])
            .expect("merge must succeed");
        assert_eq!(merged.topology_revision, "polyglot:2-ecosystems");
        assert_eq!(merged.nodes.len(), 6); // 3 nodes each
        assert!(merged.nodes.contains_key("component:mycrate"));
        assert!(merged.nodes.contains_key("component:web-client"));

        // Cross-language contract edge via explicit mapping
        let yaml_map = r#"
schema_version: 1
mappings:
  - sut: "schema:openapi"
    tests:
      - id: "contract_test_backend"
        kind: contract
    affects: ["component:web-client"]
    reason: "OpenAPI schema change affects TypeScript client"
"#;
        let map = ProjectTestMapV1::from_yaml_str(yaml_map).expect("map must parse");

        // Add the schema node + edge to the topology manually (simulating what a
        // schema-detection adapter would produce)
        let mut enriched = merged.clone();
        let schema_node = SutNodeV1::new(
            "schema:openapi".to_string(),
            SutKind::Schema,
            String::new(),
            Some("openapi.yaml".to_string()),
        );
        enriched
            .nodes
            .insert("schema:openapi".to_string(), schema_node);

        let edge_prov = EdgeProvenanceV1 {
            source: "schema-adapter".to_string(),
            adapter_version: "schema-adapter-v1".to_string(),
            confidence_source: "file-marker".to_string(),
        };
        enriched.edges.push(TopologyEdgeV1::new(
            TopologyEdgeKind::ContractDependency,
            "schema:openapi".to_string(),
            "component:web-client".to_string(),
            edge_prov,
        ));

        enriched
            .validate()
            .expect("enriched topology must be valid");

        // FallbackResolver resolves schema:openapi → ExplicitMapping
        let registry = CapabilityRegistryV1::new();
        let resolver = FallbackResolverV1::new();
        let outcome = resolver.resolve(
            &registry,
            &map,
            "schema:openapi",
            SutKind::Schema,
            SelectorGranularity::Component,
        );

        match outcome {
            ResolutionOutcome::ExplicitMapping { entry } => {
                assert_eq!(entry.sut, "schema:openapi");
                assert!(entry.affects.contains(&"component:web-client".to_string()));
            }
            other => panic!("expected ExplicitMapping, got {:?}", other),
        }

        // Cross-language edge exists in the enriched topology
        let cross_edges: Vec<_> = enriched
            .edges
            .iter()
            .filter(|e| e.edge_kind == TopologyEdgeKind::ContractDependency)
            .collect();
        assert_eq!(cross_edges.len(), 1);
        assert_eq!(cross_edges[0].from_node, "schema:openapi");
        assert_eq!(cross_edges[0].to_node, "component:web-client");

        // Zero kernel branching: same resolver used as single-language
        let single_outcome = resolver.resolve(
            &registry,
            &map,
            "component:web-client",
            SutKind::Component,
            SelectorGranularity::Component,
        );
        // component:web-client has no explicit entry, so it should be Unresolved
        // (no registry capability either) — proving the resolver path is identical
        assert!(matches!(
            single_outcome,
            ResolutionOutcome::Unresolved { .. }
        ));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-4.4 — Add another ecosystem (Go) — profile, not kernel branching
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn uat_add_go_ecosystem_no_kernel_change() {
        // Start with rust + ts
        let mut rust_profile = make_profile(
            "rust-adapter",
            "rust",
            "Cargo.toml",
            vec!["[package]", "name = "],
            "component:mycrate",
        );
        rust_profile.capabilities =
            vec![make_cap("rust:unit", "rust-adapter", CapabilityKind::Unit)];

        let mut ts_profile = make_profile(
            "ts-adapter",
            "typescript",
            "package.json",
            vec!["\"name\":", "\"version\":"],
            "component:web-client",
        );
        ts_profile.capabilities = vec![make_cap("ts:unit", "ts-adapter", CapabilityKind::Unit)];

        let mut go_profile = make_profile(
            "go-adapter",
            "go",
            "go.mod",
            vec!["module ", "go 1."],
            "component:mymodule",
        );
        go_profile.capabilities = vec![make_cap("go:unit", "go-adapter", CapabilityKind::Unit)];

        let rust_snap = snapshot_from(vec![("Cargo.toml", FIXTURE_CARGO_TOML)]);
        let ts_snap = snapshot_from(vec![("package.json", FIXTURE_PACKAGE_JSON)]);
        let go_snap = snapshot_from(vec![("go.mod", FIXTURE_GO_MOD)]);

        let rust_adapter = ProfileAdapterV1::new(&rust_profile, &rust_snap);
        let ts_adapter = ProfileAdapterV1::new(&ts_profile, &ts_snap);
        let go_adapter = ProfileAdapterV1::new(&go_profile, &go_snap);

        let rust_topo = rust_adapter.topology().expect("rust");
        let ts_topo = ts_adapter.topology().expect("ts");
        let go_topo = go_adapter.topology().expect("go");

        // 3-ecosystem merge
        let merged = compose_polyglot_topology(vec![rust_topo, ts_topo, go_topo])
            .expect("3-ecosystem merge must succeed");
        assert_eq!(merged.topology_revision, "polyglot:3-ecosystems");
        assert_eq!(merged.nodes.len(), 9); // 3 × (component + manifest + cap)

        // Resolver still works the same way — no kernel branching
        let yaml_map = r#"
schema_version: 1
mappings:
  - sut: "schema:openapi"
    tests:
      - id: "contract_test"
        kind: contract
    affects: ["component:web-client"]
    reason: "OpenAPI affects TS client"
"#;
        let map = ProjectTestMapV1::from_yaml_str(yaml_map).unwrap();
        let registry = CapabilityRegistryV1::new();
        let resolver = FallbackResolverV1::new();

        let outcome = resolver.resolve(
            &registry,
            &map,
            "schema:openapi",
            SutKind::Schema,
            SelectorGranularity::Component,
        );
        assert!(matches!(outcome, ResolutionOutcome::ExplicitMapping { .. }));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-4.5 — Fail-closed
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn uat_fail_closed_missing_manifest() {
        let profile = make_profile(
            "rust-adapter",
            "rust",
            "Cargo.toml",
            vec!["[package]"],
            "component:mycrate",
        );
        let snap = snapshot_from(vec![]); // empty — no Cargo.toml
        let adapter = ProfileAdapterV1::new(&profile, &snap);

        let result = adapter.topology();
        assert!(matches!(result, Err(AdapterError::Unavailable { .. })));
    }

    #[test]
    fn uat_fail_closed_markers_missing() {
        let profile = make_profile(
            "rust-adapter",
            "rust",
            "Cargo.toml",
            vec!["[package]", "name = ", "THIS_MARKER_DOES_NOT_EXIST"],
            "component:mycrate",
        );
        let snap = snapshot_from(vec![("Cargo.toml", FIXTURE_CARGO_TOML)]);
        let adapter = ProfileAdapterV1::new(&profile, &snap);

        let result = adapter.topology();
        assert!(matches!(result, Err(AdapterError::DetectionFailed { .. })));
    }

    #[test]
    fn compose_rejects_duplicate_node_id() {
        let mut profile = make_profile(
            "ts-adapter",
            "typescript",
            "package.json",
            vec!["\"name\":", "\"version\":"],
            "component:same-id",
        );
        profile.capabilities = vec![make_cap("cap", "ts-adapter", CapabilityKind::Unit)];

        let snap = snapshot_from(vec![("package.json", FIXTURE_PACKAGE_JSON)]);
        let adapter = ProfileAdapterV1::new(&profile, &snap);
        let topo = adapter.topology().expect("topology");

        let result = compose_polyglot_topology(vec![topo.clone(), topo.clone()]);
        assert!(matches!(result, Err(AdapterError::InvalidInput { .. })));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REQ-1: profile validation
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn profile_rejects_empty_adapter_id() {
        let mut profile = make_profile("", "rust", "Cargo.toml", vec!["[package]"], "c");
        profile.capabilities = vec![make_cap("cap", "non-empty", CapabilityKind::Unit)];
        assert!(matches!(
            profile.validate(),
            Err(EcosystemProfileError::EmptyField { field }) if field == "adapter_id"
        ));
    }

    #[test]
    fn profile_rejects_capability_adapter_mismatch() {
        let mut profile = make_profile("adapter-a", "rust", "Cargo.toml", vec!["[package]"], "c");
        profile.capabilities = vec![make_cap("cap", "adapter-b", CapabilityKind::Unit)];
        assert!(matches!(
            profile.validate(),
            Err(EcosystemProfileError::CapabilityAdapterMismatch { expected, got, .. })
            if expected == "adapter-a" && got == "adapter-b"
        ));
    }

    #[test]
    fn profile_validates_ok() {
        let mut profile = make_profile(
            "rust-adapter",
            "rust",
            "Cargo.toml",
            vec!["[package]"],
            "component:mycrate",
        );
        profile.capabilities = vec![make_cap("cap", "rust-adapter", CapabilityKind::Unit)];
        assert!(profile.validate().is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Variant-count assertions
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn ecosystem_profile_error_variant_count() {
        let variants = [
            EcosystemProfileError::EmptyField {
                field: String::new(),
            },
            EcosystemProfileError::CapabilityAdapterMismatch {
                expected: String::new(),
                got: String::new(),
                capability_id: String::new(),
            },
        ];
        assert_eq!(variants.len(), 2);
    }
}
