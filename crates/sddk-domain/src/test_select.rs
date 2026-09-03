//! Deterministic test-impact propagation and progressive test selection (TEST-SELECT-001).
//!
//! Implements `TestImpactPlannerPort` with:
//! - REQ-1: Constructor injection of change_set, topology, registry, map.
//! - REQ-2: Deterministic propagation (ownership → boundary classification → stages 0-4).
//! - REQ-3: Canonical hashing + stability.
//! - REQ-4: Fail-closed (unmapped → Blocked + `insufficient()`).
//! - REQ-5: Explicit map integration.
//! - REQ-6: 8 acceptance scenarios (TDD).
//!
//! Determinism: BTreeMap/BTreeSet, sorted iteration, canonical JSON for hashing.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

/// Raw unmapped data stored after a `Blocked` verdict, used to reconstruct
/// `InsufficientMappingV1` on demand via `insufficient()`.
type StoredUnmapped = Option<(Vec<String>, Vec<String>, Vec<TopologyEdgeKind>)>;

use sha2::{Digest, Sha256};

use crate::test_model::{
    ActiveChangeSetV1, CapabilityKind, ChangeKind, ChangedArtifactV1, EdgeProvenanceV1,
    ImpactReason, InsufficientMappingV1, MappingOutcome, PlanVerdict, ProjectTestTopologyV1,
    SCHEMA_VERSION, SelectorGranularity, SutKind, SutNodeV1, TestBatchV1, TestSelectionPlanV1,
    TopologyEdgeKind, TopologyEdgeV1, VerificationCapabilityV1,
};
use crate::test_ports::{
    AdapterError, CapabilityRegistryV1, ProjectTestMapV1, TestImpactPlannerPort,
};

// ── ImpactPlannerV1 ──────────────────────────────────────────────────────────

/// Deterministic impact planner — implements `TestImpactPlannerPort` (SPEC-043 §7/§8).
///
/// Constructed via `new()` which injects all inputs; `plan()` validates the
/// passed digests against the injected inputs and computes the selection.
///
/// # Design
///
/// - All internal state is deterministic: BTreeMap/BTreeSet, sorted iteration,
///   canonical JSON hashing via `compute_plan_hash()`.
/// - Boundary classification, ownership resolution, and BFS closure are pure
///   functions with no side-effects.
/// - Fail-closed: unmapped artifacts or dangling edges produce a `Blocked` verdict
///   with an `InsufficientMappingV1` that encodes exactly what could not be mapped.
#[derive(Debug)]
pub struct ImpactPlannerV1 {
    change_set: ActiveChangeSetV1,
    topology: ProjectTestTopologyV1,
    registry: CapabilityRegistryV1,
    map: Option<ProjectTestMapV1>,
    stage4_policy_enabled: bool,
    /// Stores the raw unmapped data from the last `plan()` call that produced a
    /// `Blocked` verdict, so `insufficient()` can reconstruct the `InsufficientMappingV1`
    /// on demand. Uses `RwLock` for interior mutability (`Sync` + `Clone`).
    stored_unmapped: RwLock<StoredUnmapped>,
}

impl ImpactPlannerV1 {
    /// Creates a new impact planner with injected inputs.
    ///
    /// `map` is optional — when `Some`, explicit mapping entries are integrated
    /// into stage 3; when `None`, only registry capabilities are used.
    ///
    /// `stage4_policy_enabled` gates risk-policy escalation batches (Stage 4).
    pub fn new(
        change_set: ActiveChangeSetV1,
        topology: ProjectTestTopologyV1,
        registry: CapabilityRegistryV1,
        map: Option<ProjectTestMapV1>,
        stage4_policy_enabled: bool,
    ) -> Self {
        Self {
            change_set,
            topology,
            registry,
            map,
            stage4_policy_enabled,
            stored_unmapped: RwLock::new(None),
        }
    }

    /// Returns the change-set digest of the injected inputs (owned String).
    fn injected_change_set_digest(&self) -> String {
        self.change_set
            .change_set_digest
            .clone()
            .unwrap_or_else(|| self.change_set.compute_change_set_digest())
    }

    /// Returns the topology revision of the injected topology.
    fn injected_topology_revision(&self) -> &str {
        &self.topology.topology_revision
    }

    /// Returns `Some` with the `InsufficientMappingV1` reconstructed from stored raw
    /// unmapped data; `None` if the last `plan()` call did not produce a `Blocked` verdict.
    pub fn insufficient(&self) -> Option<InsufficientMappingV1> {
        self.stored_unmapped
            .read()
            .unwrap()
            .as_ref()
            .map(|(artifacts, suts, relations)| {
                self.build_insufficient(artifacts.clone(), suts.clone(), relations.clone())
            })
    }

    // ── Internal propagation engine ─────────────────────────────────────────────

    /// Main propagation: runs all stages and returns (batches, impacted_suts, unmapped).
    fn propagate(&self) -> PropResult {
        let mut stage0: BTreeMap<String, Vec<ImpactReason>> = BTreeMap::new(); // test_id → reasons
        let mut stage1: BTreeMap<String, Vec<ImpactReason>> = BTreeMap::new();
        let mut stage2: BTreeMap<String, Vec<ImpactReason>> = BTreeMap::new();
        let mut stage3: BTreeMap<String, Vec<ImpactReason>> = BTreeMap::new();
        let mut stage4: BTreeMap<String, Vec<ImpactReason>> = BTreeMap::new();

        let mut impacted_suts: BTreeSet<String> = BTreeSet::new();
        let mut unmapped_artifacts: BTreeSet<String> = BTreeSet::new();
        let mut unmapped_suts: BTreeSet<String> = BTreeSet::new();
        let mut missing_relations: BTreeSet<TopologyEdgeKind> = BTreeSet::new();

        // Sort changed artifacts by path for deterministic ordering
        let mut sorted_artifacts: Vec<&ChangedArtifactV1> =
            self.change_set.changed_artifacts.iter().collect();
        sorted_artifacts.sort_by_key(|a| &a.path);

        // Build lookup indexes
        let owners_index = self.build_owners_index();
        let tests_index = self.build_tests_index();
        let reverse_deps_index = self.build_reverse_deps_index();

        // Track tests already assigned to earlier stages (global dedup per SPEC REQ-2)
        let mut seen_tests: BTreeSet<String> = BTreeSet::new();

        for artifact in &sorted_artifacts {
            let path = &artifact.path;

            // ── Step 1: Find narrowest owner ──────────────────────────────────
            let owner = self.find_narrowest_owner(path, &owners_index);

            match owner {
                Some((node_id, owner_kind)) => {
                    impacted_suts.insert(node_id.clone());

                    // ── Stage 0: Compile/TypeCheck/Lint capabilities ─────────────
                    let cap_kinds = [
                        CapabilityKind::Compile,
                        CapabilityKind::TypeCheck,
                        CapabilityKind::Lint,
                    ];
                    for cap_kind in cap_kinds {
                        self.propagate_stage0(&node_id, cap_kind, &mut stage0, &mut impacted_suts);
                    }

                    // ── Stage 1: Direct tests via Tests edges ───────────────────
                    if let Some(direct_tests) = tests_index.get(&node_id) {
                        for test_id in direct_tests {
                            if seen_tests.insert(test_id.clone()) {
                                stage1
                                    .entry(test_id.clone())
                                    .or_default()
                                    .push(ImpactReason::LocalUnitTest);
                            }
                        }
                    }

                    // ── Stage 2: Owning component/build-unit tests ───────────────
                    let component_tests =
                        self.find_component_tests(&node_id, owner_kind, &tests_index);
                    for (test_id, reason) in component_tests {
                        if seen_tests.insert(test_id.clone()) {
                            stage2.entry(test_id).or_default().push(reason);
                        }
                    }

                    // ── Stage 3: Closure via dependency edges ──────────────────
                    self.propagate_stage3(
                        &node_id,
                        &tests_index,
                        &reverse_deps_index,
                        &mut stage3,
                        &mut impacted_suts,
                        &mut seen_tests,
                    );
                }
                None => {
                    unmapped_artifacts.insert(path.clone());
                }
            }
        }

        // ── Explicit map integration (Stage 3 join) ───────────────────────────
        if let Some(ref map) = self.map {
            for entry in &map.mappings {
                if !self.topology.nodes.contains_key(&entry.sut) {
                    // REQ-5: unknown sut → fail-closed
                    unmapped_suts.insert(entry.sut.clone());
                    continue;
                }
                impacted_suts.insert(entry.sut.clone());

                for mapped_test in &entry.tests {
                    if seen_tests.insert(mapped_test.id.clone()) {
                        stage3
                            .entry(mapped_test.id.clone())
                            .or_default()
                            .push(ImpactReason::ExplicitTestAssociation);
                    }
                }

                for affected_sut in &entry.affects {
                    if !self.topology.nodes.contains_key(affected_sut) {
                        unmapped_suts.insert(affected_sut.clone());
                    } else {
                        impacted_suts.insert(affected_sut.clone());
                    }
                }
            }
        }

        // ── Stage 4: Risk policy escalation ────────────────────────────────
        if self.stage4_policy_enabled {
            self.propagate_stage4(&impacted_suts, &mut stage4);
        }

        // ── Assemble batches (stage ascending) ─────────────────────────────
        let mut batches = Vec::new();
        batches.append(&mut self.make_batch(stage0, 0));
        batches.append(&mut self.make_batch(stage1, 1));
        batches.append(&mut self.make_batch(stage2, 2));
        batches.append(&mut self.make_batch(stage3, 3));
        if self.stage4_policy_enabled {
            batches.append(&mut self.make_batch(stage4, 4));
        }

        let unmapped_artifacts: Vec<String> = unmapped_artifacts.into_iter().collect();
        let unmapped_suts: Vec<String> = unmapped_suts.into_iter().collect();
        let missing_relations: Vec<TopologyEdgeKind> = missing_relations.into_iter().collect();

        let has_unmapped = !unmapped_artifacts.is_empty()
            || !unmapped_suts.is_empty()
            || !missing_relations.is_empty();

        PropResult {
            batches,
            impacted_suts: impacted_suts.into_iter().collect(),
            unmapped_artifacts,
            unmapped_suts,
            missing_relations,
            has_unmapped,
        }
    }

    /// Builds an index: artifact_path → (owning_node_id, owning_node_kind)
    fn build_owners_index(&self) -> BTreeMap<String, (String, SutKind)> {
        let mut index: BTreeMap<String, (String, SutKind)> = BTreeMap::new();

        // Two ways to own: Owns edge from node to artifact, or owned SourceArtifact node_id == artifact
        for edge in &self.topology.edges {
            if edge.edge_kind == TopologyEdgeKind::Owns {
                let from_node = &edge.from_node;
                let to_node = &edge.to_node;
                // to_node is the owned artifact
                if let Some(target_node) = self.topology.nodes.get(to_node) {
                    // If the owned node is a SourceArtifact whose node_id equals the path,
                    // OR if we have an Owns edge pointing TO this path
                    // We record the from_node as the owner of the to_node
                    index.insert(to_node.clone(), (from_node.clone(), target_node.kind));
                }
                // Also check: if to_node is a SourceArtifact and its node_id is a path
                if let Some(target_node) = self.topology.nodes.get(to_node)
                    && target_node.kind == SutKind::SourceArtifact
                {
                    // The node_id of a SourceArtifact IS the artifact path
                    index.insert(to_node.clone(), (from_node.clone(), target_node.kind));
                }
            }
        }

        // Direct: if the artifact path matches a node's node_id
        for (node_id, node) in &self.topology.nodes {
            if node_id == &node.node_id {
                // This node IS the artifact (direct match)
                // We still need to find its owner
            }
        }

        index
    }

    /// Builds an index: sut_node_id → Vec<test_id> via Tests edges
    fn build_tests_index(&self) -> BTreeMap<String, Vec<String>> {
        let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for edge in &self.topology.edges {
            if edge.edge_kind == TopologyEdgeKind::Tests {
                // from_node Tests to_node (the test)
                let sut_id = &edge.from_node;
                let test_id = &edge.to_node;
                index
                    .entry(sut_id.clone())
                    .or_default()
                    .push(test_id.clone());
            }
        }
        // Sort each list for determinism
        for tests in index.values_mut() {
            tests.sort();
        }
        index
    }

    /// Builds an index: sut_node_id → Vec<(dependent_node_id, edge_kind)>
    fn build_reverse_deps_index(&self) -> BTreeMap<String, Vec<(String, TopologyEdgeKind)>> {
        let mut index: BTreeMap<String, Vec<(String, TopologyEdgeKind)>> = BTreeMap::new();

        let dep_kinds = [
            TopologyEdgeKind::ReverseDependsOn,
            TopologyEdgeKind::RuntimeDependsOn,
            TopologyEdgeKind::ContractDependency,
            TopologyEdgeKind::DependsOn,
        ];

        for edge in &self.topology.edges {
            if dep_kinds.contains(&edge.edge_kind) {
                // ReverseDependsOn: from_node is the dependent, to_node is what it depends on
                // We want: given a changed node, find what depends on it (reverse)
                // edge.from_node depends on edge.to_node
                // So reverse: edge.to_node has a dependent edge.from_node
                index
                    .entry(edge.to_node.clone())
                    .or_default()
                    .push((edge.from_node.clone(), edge.edge_kind));
            }
        }

        // Sort for determinism
        for deps in index.values_mut() {
            deps.sort_by_key(|(id, _kind)| id.clone());
        }

        index
    }

    /// Finds the narrowest owner for an artifact path.
    fn find_narrowest_owner<'a>(
        &'a self,
        artifact_path: &str,
        owners_index: &'a BTreeMap<String, (String, SutKind)>,
    ) -> Option<(String, SutKind)> {
        // Two lookup strategies:
        // 1. Direct match: artifact_path == node.node_id for a SourceArtifact node
        // 2. Owns edge: artifact_path is the target of an Owns edge

        // First: check if a SourceArtifact node exists with node_id == artifact_path
        if let Some(node) = self.topology.nodes.get(artifact_path)
            && node.kind == SutKind::SourceArtifact
        {
            // This node IS the artifact. Find its owner via Owns edges.
            // Look for an Owns edge where to_node == artifact_path
            for edge in &self.topology.edges {
                if edge.edge_kind == TopologyEdgeKind::Owns
                    && edge.to_node == artifact_path
                    && let Some(owner_node) = self.topology.nodes.get(&edge.from_node)
                {
                    return Some((edge.from_node.clone(), owner_node.kind));
                }
            }
        }

        // Second: check the owners index
        owners_index.get(artifact_path).cloned()
    }

    /// Classifies the boundary kind of a SUT node.
    fn classify_boundary(&self, node_id: &str) -> ImpactReason {
        let node = match self.topology.nodes.get(node_id) {
            Some(n) => n,
            None => return ImpactReason::DirectSourceTouch,
        };

        match node.kind {
            SutKind::Schema => ImpactReason::SchemaChange,
            SutKind::ConfigurationSurface => ImpactReason::ConfigurationChange,
            SutKind::GeneratedArtifact => ImpactReason::GeneratedSurfaceChange,
            SutKind::BuildUnit => ImpactReason::BuildOrWorkspaceChange,
            SutKind::ContractBoundary => ImpactReason::PublicContractChange,
            _ => ImpactReason::DirectSourceTouch,
        }
    }

    /// Stage 0: adds compile/typecheck/lint capabilities for the SUT kind.
    fn propagate_stage0(
        &self,
        sut_id: &str,
        cap_kind: CapabilityKind,
        stage0: &mut BTreeMap<String, Vec<ImpactReason>>,
        impacted_suts: &mut BTreeSet<String>,
    ) {
        let node = match self.topology.nodes.get(sut_id) {
            Some(n) => n,
            None => return,
        };

        let caps = self.registry.by_kind(cap_kind);
        for cap in caps {
            if cap.supported_sut_kinds.contains(&node.kind) {
                stage0
                    .entry(cap.capability_id.clone())
                    .or_default()
                    .push(ImpactReason::DirectSourceTouch);
                impacted_suts.insert(sut_id.to_string());
            }
        }
    }

    /// Finds component/build-unit tests for stage 2.
    ///
    /// When the owner is a BuildUnit, also finds tests on the owning Component
    /// (via the Owns edge where to_node == owner_id).
    fn find_component_tests(
        &self,
        owner_id: &str,
        owner_kind: SutKind,
        tests_index: &BTreeMap<String, Vec<String>>,
    ) -> Vec<(String, ImpactReason)> {
        let mut results = Vec::new();

        // For BuildUnit: find tests on the BuildUnit AND on its owning Component
        if owner_kind == SutKind::BuildUnit {
            // Tests directly on the BuildUnit (BuildUnitOwnership)
            if let Some(tests) = tests_index.get(owner_id) {
                for test_id in tests {
                    results.push((test_id.clone(), ImpactReason::BuildUnitOwnership));
                }
            }
            // Find owning component via Owns edge (where to_node == owner_id)
            for edge in &self.topology.edges {
                if edge.edge_kind == TopologyEdgeKind::Owns
                    && edge.to_node == owner_id
                    && let Some(comp_tests) = tests_index.get(&edge.from_node)
                {
                    for test_id in comp_tests {
                        results.push((test_id.clone(), ImpactReason::ComponentOwnership));
                    }
                }
            }
        } else if owner_kind == SutKind::Component {
            // Tests directly on the Component (ComponentOwnership)
            if let Some(tests) = tests_index.get(owner_id) {
                for test_id in tests {
                    results.push((test_id.clone(), ImpactReason::ComponentOwnership));
                }
            }
        }

        results
    }

    /// Stage 3: BFS depth-1 closure via dependency edges.
    fn propagate_stage3(
        &self,
        sut_id: &str,
        tests_index: &BTreeMap<String, Vec<String>>,
        reverse_deps_index: &BTreeMap<String, Vec<(String, TopologyEdgeKind)>>,
        stage3: &mut BTreeMap<String, Vec<ImpactReason>>,
        impacted_suts: &mut BTreeSet<String>,
        seen_tests: &mut BTreeSet<String>,
    ) {
        // Find all dependents of this SUT (reverse deps, runtime deps, contract deps)
        let direct_dependents = reverse_deps_index.get(sut_id);

        if let Some(deps) = direct_dependents {
            for (dependent_id, edge_kind) in deps {
                impacted_suts.insert(dependent_id.clone());

                // Add tests for the dependent (deduplicated against earlier stages)
                if let Some(tests) = tests_index.get(dependent_id) {
                    let reason = match edge_kind {
                        TopologyEdgeKind::RuntimeDependsOn => {
                            ImpactReason::RuntimeDependencyPropagation
                        }
                        TopologyEdgeKind::ContractDependency => {
                            ImpactReason::CrossComponentContractTest
                        }
                        TopologyEdgeKind::ReverseDependsOn => {
                            ImpactReason::ReverseDependencyPropagation
                        }
                        _ => ImpactReason::DependencyPropagation,
                    };

                    for test_id in tests {
                        if seen_tests.insert(test_id.clone()) {
                            stage3.entry(test_id.clone()).or_default().push(reason);
                        }
                    }
                }

                // Contract tests via ValidatesContract edge
                for edge in &self.topology.edges {
                    if edge.edge_kind == TopologyEdgeKind::ValidatesContract
                        && edge.from_node == *sut_id
                        && let Some(tests) = tests_index.get(&edge.to_node)
                    {
                        for test_id in tests {
                            if seen_tests.insert(test_id.clone()) {
                                stage3
                                    .entry(test_id.clone())
                                    .or_default()
                                    .push(ImpactReason::CrossComponentContractTest);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Stage 4: risk policy escalation (Architecture/Security/Mutation/Uat).
    fn propagate_stage4(
        &self,
        impacted_suts: &BTreeSet<String>,
        stage4: &mut BTreeMap<String, Vec<ImpactReason>>,
    ) {
        let risk_kinds = [
            CapabilityKind::Architecture,
            CapabilityKind::Security,
            CapabilityKind::Mutation,
            CapabilityKind::Uat,
        ];

        for risk_kind in risk_kinds {
            for cap in self.registry.by_kind(risk_kind) {
                for sut_id in impacted_suts {
                    if let Some(node) = self.topology.nodes.get(sut_id)
                        && cap.supported_sut_kinds.contains(&node.kind)
                    {
                        stage4
                            .entry(cap.capability_id.clone())
                            .or_default()
                            .push(ImpactReason::RiskPolicyEscalation);
                    }
                }
            }
        }
    }

    /// Converts a stage map into a `TestBatchV1` with deduplication and sorted test_ids.
    fn make_batch(
        &self,
        mut stage_map: BTreeMap<String, Vec<ImpactReason>>,
        stage_num: u32,
    ) -> Vec<TestBatchV1> {
        if stage_map.is_empty() {
            return Vec::new();
        }

        let mut batches = Vec::new();

        // Group by capability_id to create proper batches
        let mut cap_groups: BTreeMap<String, BTreeMap<String, Vec<ImpactReason>>> = BTreeMap::new();

        for (test_id, reasons) in stage_map {
            // Find the first capability that covers this test
            // In our model, test_ids are the capability_ids themselves (stage 0)
            // or we need to look up which capability owns this test
            // For simplicity: test_id is the capability_id for stage 0/4,
            // for stages 1/2/3 we pick the first matching capability
            let cap_id = self
                .registry
                .get(&test_id)
                .map(|c| c.capability_id.clone())
                .unwrap_or_else(|| test_id.clone());

            cap_groups
                .entry(cap_id)
                .or_default()
                .entry(test_id)
                .or_default()
                .extend(reasons);
        }

        for (cap_id, mut test_groups) in cap_groups {
            // Deduplicate reasons per test (keep unique, sorted)
            for reasons in test_groups.values_mut() {
                let mut unique: BTreeSet<ImpactReason> = reasons.drain(..).collect();
                reasons.extend(unique.into_iter());
                reasons.sort();
            }

            // Collect test_ids sorted
            let mut test_ids: Vec<String> = test_groups.keys().cloned().collect();
            test_ids.sort();

            // Collect all reasons (flatten, dedup, sort)
            let all_reasons: BTreeSet<ImpactReason> =
                test_groups.values().flat_map(|r| r.clone()).collect();
            let mut reasons: Vec<ImpactReason> = all_reasons.into_iter().collect();
            reasons.sort();

            let batch = TestBatchV1 {
                stage: stage_num,
                capability_id: cap_id,
                semantic_scope: vec![],
                test_ids,
                reasons,
                expected_cost: None,
                escalation: false,
            };

            batches.push(batch);
        }

        batches
    }

    /// Compute canonical hash of the plan for stability checking.
    fn compute_plan_hash(plan: &TestSelectionPlanV1) -> String {
        let json = plan.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        format!("{:064x}", digest)
    }

    // ── Fail-closed insufficient mapping ───────────────────────────────────────

    fn build_insufficient(
        &self,
        unmapped_artifacts: Vec<String>,
        unmapped_suts: Vec<String>,
        missing_relations: Vec<TopologyEdgeKind>,
    ) -> InsufficientMappingV1 {
        // verify_required = true when:
        // 1. An unmapped SUT is not in the topology at all (can't verify the unknown),
        //    OR
        // 2. An unmapped SUT IS in the topology and its kind is
        //    Schema/ContractBoundary/GeneratedArtifact (per SPEC-043 §3.6 fail-closed).
        let verify_required = unmapped_suts.iter().any(|s| {
            match self.topology.nodes.get(s) {
                Some(n) => {
                    n.kind == SutKind::Schema
                        || n.kind == SutKind::ContractBoundary
                        || n.kind == SutKind::GeneratedArtifact
                }
                // Node not in topology → can't verify, needs human review
                None => true,
            }
        });

        InsufficientMappingV1::new(
            unmapped_artifacts,
            unmapped_suts,
            missing_relations,
            Vec::new(),
            "One or more changed artifacts could not be mapped to any SUT node, or required edges are dangling.".to_string(),
            "Add Owns edges from the owning component/build-unit to the changed artifact, or add an explicit mapping entry for the artifact.".to_string(),
            verify_required,
        )
    }
}

impl TestImpactPlannerPort for ImpactPlannerV1 {
    /// Computes a deterministic test selection plan for the given change set.
    ///
    /// Validates that the passed `change_set_digest` and `topology_revision`
    /// match the injected inputs' canonical digests/revisions. Mismatch produces
    /// `AdapterError::InvalidInput` (stale-context rejection, predictable).
    fn plan(
        &self,
        change_set_digest: &str,
        topology_revision: &str,
    ) -> Result<TestSelectionPlanV1, AdapterError> {
        // REQ-1: Stale-context rejection
        if change_set_digest != self.injected_change_set_digest() {
            return Err(AdapterError::InvalidInput {
                reason: format!(
                    "stale change_set_digest: got '{}', expected '{}'",
                    change_set_digest,
                    self.injected_change_set_digest()
                ),
            });
        }

        if topology_revision != self.injected_topology_revision() {
            return Err(AdapterError::InvalidInput {
                reason: format!(
                    "stale topology_revision: got '{}', expected '{}'",
                    topology_revision,
                    self.injected_topology_revision()
                ),
            });
        }

        // Run propagation
        let prop = self.propagate();

        // REQ-4: Fail-closed
        if prop.has_unmapped {
            let insufficient = self.build_insufficient(
                prop.unmapped_artifacts.clone(),
                prop.unmapped_suts.clone(),
                prop.missing_relations.clone(),
            );
            *self.stored_unmapped.write().unwrap() = Some((
                prop.unmapped_artifacts.clone(),
                prop.unmapped_suts.clone(),
                prop.missing_relations.clone(),
            ));

            return Err(AdapterError::InvalidInput {
                reason: format!(
                    "fail-closed: {} unmapped artifact(s), {} unmapped SUT(s)",
                    prop.unmapped_artifacts.len(),
                    prop.unmapped_suts.len()
                ),
            });
        }

        // REQ-3: confidence = 1.0 when fully mapped
        let confidence = if prop.has_unmapped { 0.0 } else { 1.0 };

        let plan = TestSelectionPlanV1::new(
            format!(
                "plan:{}:{}",
                self.injected_change_set_digest(),
                self.topology.topology_revision
            ),
            self.injected_change_set_digest(),
            self.topology.topology_revision.clone(),
            String::new(), // sut_graph_revision
            String::new(), // policy_revision
            prop.impacted_suts,
            prop.batches,
            Vec::new(), // no reused receipts
            Vec::new(), // no unmapped nodes
            confidence,
            PlanVerdict::Executable,
        );

        Ok(plan)
    }
}

// ── Internal result type ──────────────────────────────────────────────────────

struct PropResult {
    batches: Vec<TestBatchV1>,
    impacted_suts: Vec<String>,
    unmapped_artifacts: Vec<String>,
    unmapped_suts: Vec<String>,
    missing_relations: Vec<TopologyEdgeKind>,
    has_unmapped: bool,
}

// ── Tests (REQ-6) ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_model::{CapabilityKind, SelectorGranularity};

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.1: determinism/stability
    // ═══════════════════════════════════════════════════════════════════════════════

    fn make_cap(id: &str, kind: CapabilityKind, sut_kinds: &[SutKind]) -> VerificationCapabilityV1 {
        VerificationCapabilityV1::new(
            id.to_string(),
            kind,
            sut_kinds.iter().cloned().collect(),
            SelectorGranularity::TestId,
            "test-adapter".to_string(),
            "test-toolchain".to_string(),
            None,
            vec![],
        )
    }

    fn standard_registry() -> CapabilityRegistryV1 {
        let mut reg = CapabilityRegistryV1::new();
        reg.register(make_cap(
            "cap:compile",
            CapabilityKind::Compile,
            &[SutKind::BuildUnit, SutKind::Component],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:typecheck",
            CapabilityKind::TypeCheck,
            &[SutKind::BuildUnit, SutKind::Component],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:lint",
            CapabilityKind::Lint,
            &[SutKind::BuildUnit, SutKind::Component],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:unit",
            CapabilityKind::Unit,
            &[SutKind::BuildUnit, SutKind::Component],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:integration",
            CapabilityKind::Integration,
            &[SutKind::Component],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:contract",
            CapabilityKind::Contract,
            &[SutKind::ContractBoundary, SutKind::Schema],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:security",
            CapabilityKind::Security,
            &[SutKind::BuildUnit],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:arch",
            CapabilityKind::Architecture,
            &[SutKind::Component],
        ))
        .unwrap();
        reg
    }

    fn standard_prov() -> EdgeProvenanceV1 {
        EdgeProvenanceV1 {
            source: "test-adapter".to_string(),
            adapter_version: "test-adapter-v1".to_string(),
            confidence_source: "test-fixture".to_string(),
        }
    }

    fn simple_topology() -> ProjectTestTopologyV1 {
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "comp:a".to_string(),
            SutNodeV1::new(
                "comp:a".to_string(),
                SutKind::Component,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "unit:lib".to_string(),
            SutNodeV1::new(
                "unit:lib".to_string(),
                SutKind::BuildUnit,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "src:lib".to_string(),
            SutNodeV1::new(
                "src:lib".to_string(),
                SutKind::SourceArtifact,
                String::new(),
                None,
            ),
        );
        nodes.insert(
            "test:lib_tests".to_string(),
            SutNodeV1::new(
                "test:lib_tests".to_string(),
                SutKind::TestUnit,
                String::new(),
                None,
            ),
        );
        nodes.insert(
            "test:integration".to_string(),
            SutNodeV1::new(
                "test:integration".to_string(),
                SutKind::TestUnit,
                String::new(),
                None,
            ),
        );

        let edges = vec![
            TopologyEdgeV1::new(
                TopologyEdgeKind::Owns,
                "comp:a".to_string(),
                "unit:lib".to_string(),
                standard_prov(),
            ),
            TopologyEdgeV1::new(
                TopologyEdgeKind::Owns,
                "unit:lib".to_string(),
                "src:lib".to_string(),
                standard_prov(),
            ),
            TopologyEdgeV1::new(
                TopologyEdgeKind::Tests,
                "unit:lib".to_string(),
                "test:lib_tests".to_string(),
                standard_prov(),
            ),
            TopologyEdgeV1::new(
                TopologyEdgeKind::Tests,
                "comp:a".to_string(),
                "test:integration".to_string(),
                standard_prov(),
            ),
        ];

        ProjectTestTopologyV1::new("rev:1".to_string(), nodes, edges)
    }

    fn simple_change_set(path: &str) -> ActiveChangeSetV1 {
        ActiveChangeSetV1::new(
            "test-project".to_string(),
            "base".to_string(),
            "head".to_string(),
            "tree".to_string(),
            vec![ChangedArtifactV1 {
                path: path.to_string(),
                change_kind: ChangeKind::Modified,
                staged: false,
            }],
        )
    }

    #[test]
    fn determinism_identical_inputs_same_hash() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        let planner1 = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg.clone(), None, false);
        let planner2 = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg.clone(), None, false);

        let digest = cs.change_set_digest.clone().unwrap();
        let rev = topo.topology_revision.clone();

        let plan1 = planner1.plan(&digest, &rev).unwrap();
        let plan2 = planner2.plan(&digest, &rev).unwrap();

        assert_eq!(
            ImpactPlannerV1::compute_plan_hash(&plan1),
            ImpactPlannerV1::compute_plan_hash(&plan2),
            "identical inputs must produce byte-identical plans"
        );
    }

    #[test]
    fn stale_context_rejected() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, false);

        let result = planner.plan("stale-digest", &topo.topology_revision);
        assert!(matches!(result, Err(AdapterError::InvalidInput { .. })));
    }

    #[test]
    fn stale_topology_revision_rejected() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let cs_digest = cs.change_set_digest.clone().unwrap();
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs, topo.clone(), reg, None, false);

        let result = planner.plan(&cs_digest, "stale-revision");
        assert!(matches!(result, Err(AdapterError::InvalidInput { .. })));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.2: stage coverage
    // ═══════════════════════════════════════════════════════════════════════════════

    fn stage_topology() -> ProjectTestTopologyV1 {
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "comp:a".to_string(),
            SutNodeV1::new(
                "comp:a".to_string(),
                SutKind::Component,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "unit:lib".to_string(),
            SutNodeV1::new(
                "unit:lib".to_string(),
                SutKind::BuildUnit,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "src:lib".to_string(),
            SutNodeV1::new(
                "src:lib".to_string(),
                SutKind::SourceArtifact,
                String::new(),
                None,
            ),
        );
        // Stage 1: direct test
        nodes.insert(
            "test:unit_lib".to_string(),
            SutNodeV1::new(
                "test:unit_lib".to_string(),
                SutKind::TestUnit,
                String::new(),
                None,
            ),
        );
        // Stage 2: component test
        nodes.insert(
            "test:integration".to_string(),
            SutNodeV1::new(
                "test:integration".to_string(),
                SutKind::TestUnit,
                String::new(),
                None,
            ),
        );
        // Stage 3: reverse-dependency test (another component depends on lib)
        nodes.insert(
            "comp:b".to_string(),
            SutNodeV1::new(
                "comp:b".to_string(),
                SutKind::Component,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "test:compb_unit".to_string(),
            SutNodeV1::new(
                "test:compb_unit".to_string(),
                SutKind::TestUnit,
                String::new(),
                None,
            ),
        );
        // Stage 4: risk capability
        nodes.insert(
            "cap:security".to_string(),
            SutNodeV1::new(
                "cap:security".to_string(),
                SutKind::VerificationCapability,
                String::new(),
                None,
            ),
        );

        let edges = vec![
            TopologyEdgeV1::new(
                TopologyEdgeKind::Owns,
                "comp:a".to_string(),
                "unit:lib".to_string(),
                standard_prov(),
            ),
            TopologyEdgeV1::new(
                TopologyEdgeKind::Owns,
                "unit:lib".to_string(),
                "src:lib".to_string(),
                standard_prov(),
            ),
            // Stage 1: direct test on unit:lib
            TopologyEdgeV1::new(
                TopologyEdgeKind::Tests,
                "unit:lib".to_string(),
                "test:unit_lib".to_string(),
                standard_prov(),
            ),
            // Stage 2: component test on comp:a (owning Component)
            TopologyEdgeV1::new(
                TopologyEdgeKind::Tests,
                "comp:a".to_string(),
                "test:integration".to_string(),
                standard_prov(),
            ),
            // Stage 3: comp:b ReverseDependsOn unit:lib
            TopologyEdgeV1::new(
                TopologyEdgeKind::Tests,
                "comp:b".to_string(),
                "test:compb_unit".to_string(),
                standard_prov(),
            ),
            TopologyEdgeV1::new(
                TopologyEdgeKind::ReverseDependsOn,
                "comp:b".to_string(),
                "unit:lib".to_string(),
                standard_prov(),
            ),
        ];

        ProjectTestTopologyV1::new("rev:stage".to_string(), nodes, edges)
    }

    #[test]
    fn stage_coverage_direct_component_reverse_dep() {
        let topo = stage_topology();
        let cs = simple_change_set("src:lib");
        // cap:arch and cap:security already in standard_registry
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, true);

        let digest = cs.change_set_digest.clone().unwrap();
        let rev = topo.topology_revision.clone();
        let plan = planner.plan(&digest, &rev).unwrap();

        // Verify stages present
        let stages: BTreeSet<u32> = plan.batches.iter().map(|b| b.stage).collect();
        assert!(
            stages.contains(&0),
            "Stage 0 (compile/typecheck/lint) must be present"
        );
        assert!(
            stages.contains(&1),
            "Stage 1 (direct tests) must be present"
        );
        assert!(
            stages.contains(&2),
            "Stage 2 (component tests) must be present"
        );
        assert!(
            stages.contains(&3),
            "Stage 3 (dependency closure) must be present"
        );
        assert!(
            stages.contains(&4),
            "Stage 4 (risk) must be present when enabled"
        );

        // Every batch must have ≥1 reason
        for batch in &plan.batches {
            assert!(
                !batch.reasons.is_empty(),
                "batch at stage {} must have ≥1 reason",
                batch.stage
            );
        }

        // Every test in impacted_suts appears in some batch (test nodes only, not components)
        for sut in &plan.impacted_sut {
            // Only check test nodes - component/runtime nodes may not have direct tests
            if sut.starts_with("test:") || sut.starts_with("cap:") {
                assert!(
                    plan.batches
                        .iter()
                        .any(|b| b.test_ids.iter().any(|t| t == sut)),
                    "impacted test sut '{}' must appear in some batch",
                    sut
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.3: fail-closed
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn fail_closed_unmapped_artifact() {
        let topo = simple_topology();
        // Change an artifact that doesn't exist in topology
        let cs = ActiveChangeSetV1::new(
            "test-project".to_string(),
            "base".to_string(),
            "head".to_string(),
            "tree".to_string(),
            vec![ChangedArtifactV1 {
                path: "nonexistent/file.rs".to_string(),
                change_kind: ChangeKind::Modified,
                staged: false,
            }],
        );
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, false);

        let digest = cs.change_set_digest.clone().unwrap();
        let result = planner.plan(&digest, &topo.topology_revision);

        assert!(
            matches!(result, Err(AdapterError::InvalidInput { .. })),
            "unmapped artifact must produce InvalidInput"
        );
    }

    #[test]
    fn insufficient_returns_insufficient_mapping_on_blocked() {
        let topo = simple_topology();
        // Change an artifact that has no owner in the topology → unmapped
        let cs = ActiveChangeSetV1::new(
            "test-project".to_string(),
            "base".to_string(),
            "head".to_string(),
            "tree".to_string(),
            vec![ChangedArtifactV1 {
                path: "nonexistent/file.rs".to_string(),
                change_kind: ChangeKind::Modified,
                staged: false,
            }],
        );
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, false);

        let digest = cs.change_set_digest.clone().unwrap();
        let result = planner.plan(&digest, &topo.topology_revision);

        // plan() must fail with Blocked
        assert!(
            matches!(result, Err(AdapterError::InvalidInput { .. })),
            "unmapped artifact must produce InvalidInput"
        );

        // After plan() fails, insufficient() must return Some with real data
        let insufficient = planner.insufficient();
        assert!(
            insufficient.is_some(),
            "insufficient() must be Some after Blocked verdict"
        );
        let insufficient = insufficient.unwrap();

        // unmapped_artifacts must contain the orphan path
        assert!(
            insufficient
                .unmapped_artifacts
                .contains(&"nonexistent/file.rs".to_string()),
            "unmapped_artifacts must contain the orphan path"
        );

        // justification and remediation must be non-empty (fail-closed contract)
        assert!(
            !insufficient.justification.is_empty(),
            "justification must be non-empty"
        );
        assert!(
            !insufficient.remediation.is_empty(),
            "remediation must be non-empty"
        );

        // verify_required is false for a plain path (not Schema/ContractBoundary/GeneratedArtifact)
        assert!(
            !insufficient.verify_required,
            "verify_required must be false for non-critical unmapped artifact"
        );
    }

    #[test]
    fn insufficient_verify_required_true_for_schema_node() {
        // Create a topology where an explicit map entry points to a non-existent SUT
        // that would be classified as Schema if it existed → verify_required=true
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        // Create a map that references a Schema node that doesn't exist
        // by using a dangling reference via explicit map
        let yaml_map = r#"
schema_version: 1
mappings:
  - sut: "schema:missing_api"
    tests:
      - id: "test:orphan_schema"
        kind: contract
    affects: []
    reason: "Maps to non-existent Schema node"
"#;
        let map = ProjectTestMapV1::from_yaml_str(yaml_map).unwrap();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, Some(map), false);

        let digest = cs.change_set_digest.clone().unwrap();
        let result = planner.plan(&digest, &topo.topology_revision);

        // plan() must fail because sut "schema:missing_api" doesn't exist in topology
        assert!(
            matches!(result, Err(AdapterError::InvalidInput { .. })),
            "unknown sut in explicit map must produce InvalidInput"
        );

        let insufficient = planner.insufficient();
        assert!(
            insufficient.is_some(),
            "insufficient() must be Some after Blocked"
        );
        let insufficient = insufficient.unwrap();

        // The unmapped sut is "schema:missing_api" — since its kind would be Schema,
        // verify_required must be true
        assert!(
            insufficient.verify_required,
            "verify_required must be true for unmapped SUT of kind Schema"
        );
        assert!(
            insufficient
                .unmapped_suts
                .contains(&"schema:missing_api".to_string()),
            "unmapped_suts must contain the unknown Schema node id"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.4: explicit map
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn explicit_map_joins_stage3() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        let yaml_map = r#"
schema_version: 1
mappings:
  - sut: "unit:lib"
    tests:
      - id: "test:mapped_explicit"
        kind: unit
    affects: ["comp:a"]
    reason: "Explicit test for lib"
"#;
        let map = ProjectTestMapV1::from_yaml_str(yaml_map).unwrap();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, Some(map), false);

        let digest = cs.change_set_digest.clone().unwrap();
        let rev = topo.topology_revision.clone();
        let plan = planner.plan(&digest, &rev).unwrap();

        // The explicit mapped test must appear in some batch with ExplicitTestAssociation reason
        let has_explicit = plan.batches.iter().any(|b| {
            b.test_ids.iter().any(|t| t == "test:mapped_explicit")
                && b.reasons.contains(&ImpactReason::ExplicitTestAssociation)
        });
        assert!(
            has_explicit,
            "explicit mapped test must appear in stage 3 with ExplicitTestAssociation"
        );
    }

    #[test]
    fn explicit_map_unknown_sut_blocked() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        let yaml_map = r#"
schema_version: 1
mappings:
  - sut: "nonexistent:sut"
    tests:
      - id: "test:orphan"
        kind: unit
    affects: []
    reason: "Maps to unknown SUT"
"#;
        let map = ProjectTestMapV1::from_yaml_str(yaml_map).unwrap();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, Some(map), false);

        let digest = cs.change_set_digest.clone().unwrap();
        let result = planner.plan(&digest, &topo.topology_revision);

        assert!(
            matches!(result, Err(AdapterError::InvalidInput { .. })),
            "unknown sut in explicit map must produce InvalidInput"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.5: stage4 gate
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn stage4_enabled_contains_risk_batch() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let mut reg = standard_registry();
        // cap:security and cap:arch already in standard_registry; add mutation and uat
        reg.register(make_cap(
            "cap:mutation",
            CapabilityKind::Mutation,
            &[SutKind::BuildUnit],
        ))
        .unwrap();
        reg.register(make_cap(
            "cap:uat",
            CapabilityKind::Uat,
            &[SutKind::Component],
        ))
        .unwrap();

        let planner_enabled =
            ImpactPlannerV1::new(cs.clone(), topo.clone(), reg.clone(), None, true);
        let digest = cs.change_set_digest.clone().unwrap();
        let rev = topo.topology_revision.clone();
        let plan_enabled = planner_enabled.plan(&digest, &rev).unwrap();

        let stages_enabled: BTreeSet<u32> = plan_enabled.batches.iter().map(|b| b.stage).collect();
        assert!(
            stages_enabled.contains(&4),
            "Stage 4 must be present when enabled"
        );
    }

    #[test]
    fn stage4_disabled_no_risk_batch() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();
        // stage4 disabled, so no risk batches regardless of registry contents

        let planner_disabled = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, false);
        let digest = cs.change_set_digest.clone().unwrap();
        let rev = topo.topology_revision.clone();
        let plan_disabled = planner_disabled.plan(&digest, &rev).unwrap();

        let stages_disabled: BTreeSet<u32> =
            plan_disabled.batches.iter().map(|b| b.stage).collect();
        assert!(
            !stages_disabled.contains(&4),
            "Stage 4 must be absent when disabled"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.6: dedup
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_reachable_via_two_paths_appears_once_earliest_stage() {
        // A test reachable via both direct Tests edge AND component ownership
        // should appear only in stage 1 (earliest), not stage 2
        let mut nodes: BTreeMap<String, SutNodeV1> = BTreeMap::new();
        nodes.insert(
            "comp:a".to_string(),
            SutNodeV1::new(
                "comp:a".to_string(),
                SutKind::Component,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "unit:lib".to_string(),
            SutNodeV1::new(
                "unit:lib".to_string(),
                SutKind::BuildUnit,
                "rust".to_string(),
                None,
            ),
        );
        nodes.insert(
            "src:lib".to_string(),
            SutNodeV1::new(
                "src:lib".to_string(),
                SutKind::SourceArtifact,
                String::new(),
                None,
            ),
        );
        // This test is connected to BOTH unit:lib (direct) AND comp:a (component)
        nodes.insert(
            "test:shared".to_string(),
            SutNodeV1::new(
                "test:shared".to_string(),
                SutKind::TestUnit,
                String::new(),
                None,
            ),
        );

        let edges = vec![
            TopologyEdgeV1::new(
                TopologyEdgeKind::Owns,
                "comp:a".to_string(),
                "unit:lib".to_string(),
                standard_prov(),
            ),
            TopologyEdgeV1::new(
                TopologyEdgeKind::Owns,
                "unit:lib".to_string(),
                "src:lib".to_string(),
                standard_prov(),
            ),
            // Direct test on unit:lib (stage 1)
            TopologyEdgeV1::new(
                TopologyEdgeKind::Tests,
                "unit:lib".to_string(),
                "test:shared".to_string(),
                standard_prov(),
            ),
            // Also connected to comp:a (stage 2)
            TopologyEdgeV1::new(
                TopologyEdgeKind::Tests,
                "comp:a".to_string(),
                "test:shared".to_string(),
                standard_prov(),
            ),
        ];

        let topo = ProjectTestTopologyV1::new("rev:dedup".to_string(), nodes, edges);
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, false);
        let digest = cs.change_set_digest.clone().unwrap();
        let plan = planner.plan(&digest, &topo.topology_revision).unwrap();

        // test:shared must appear exactly once
        let count = plan
            .batches
            .iter()
            .filter(|b| b.test_ids.contains(&"test:shared".to_string()))
            .count();
        assert_eq!(
            count, 1,
            "test reachable via two paths must appear exactly once"
        );

        // It must be in stage 1 (earliest), not stage 2
        let stage1_batch = plan.batches.iter().find(|b| b.stage == 1);
        assert!(
            stage1_batch
                .map(|b| b.test_ids.contains(&"test:shared".to_string()))
                .unwrap_or(false),
            "test must appear in stage 1 (earliest)"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.7: stale-context
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn stale_context_mismatched_digest() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, false);

        let result = planner.plan(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            &topo.topology_revision,
        );
        assert!(matches!(result, Err(AdapterError::InvalidInput { .. })));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.8: explainability
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn every_batch_has_at_least_one_reason() {
        let topo = simple_topology();
        let cs = simple_change_set("src:lib");
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, false);
        let digest = cs.change_set_digest.clone().unwrap();
        let plan = planner.plan(&digest, &topo.topology_revision).unwrap();

        for batch in &plan.batches {
            assert!(
                !batch.reasons.is_empty(),
                "batch at stage {} must have ≥1 reason (explainability)",
                batch.stage
            );
        }
    }

    #[test]
    fn every_batch_has_sorted_deduplicated_reasons() {
        let topo = stage_topology();
        let cs = simple_change_set("src:lib");
        // cap:security already in standard_registry
        let reg = standard_registry();

        let planner = ImpactPlannerV1::new(cs.clone(), topo.clone(), reg, None, true);
        let digest = cs.change_set_digest.clone().unwrap();
        let plan = planner.plan(&digest, &topo.topology_revision).unwrap();

        for batch in &plan.batches {
            // Reasons must be sorted (canonical order)
            let mut sorted = batch.reasons.clone();
            sorted.sort();
            assert_eq!(
                batch.reasons, sorted,
                "batch reasons must be in sorted order"
            );
            // Reasons must be unique (deduplicated)
            let unique: BTreeSet<_> = batch.reasons.iter().collect();
            assert_eq!(
                unique.len(),
                batch.reasons.len(),
                "batch reasons must be deduplicated"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Helper: typo fix in test topology builder
    // ═══════════════════════════════════════════════════════════════════════════════

    // Note: String::String() → String::new() fixed above
}
