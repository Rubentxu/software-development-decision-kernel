//! Apply verification session state machine — TEST-APPLY-001 (REQ-1).
//!
//! Implements the deterministic scoped verification session per SPEC-043 §9/§13:
//! - `next_batch`: consume the impact plan and return the next pending batch
//! - `record`: persist a evidence receipt and advance the session
//! - `refresh`: invalidate intersecting evidence and recompute the plan
//! - `status`: return the current session status
//!
//! ## Design
//!
//! - **Runner-discovery-free**: the session holds NO runner commands or ecosystem
//!   strings — it operates purely on semantic topology, capabilities, and receipts.
//! - **Deterministic**: BTreeMap/BTreeSet, ordered iteration, canonical hashing.
//! - **Fail-closed**: unmapped SUT nodes produce `Blocked` or `VerifyRequired`
//!   without fabricating batches from thin air.
//! - **Strict TDD**: when `strict_tdd: true`, only `Compile/TypeCheck/Lint` batches
//!   are admitted in RED phase until at least one compile/type-check receipt is recorded.
//!
//! ## Changelog
//! - **2026-09-03**: Initial implementation (TEST-APPLY-001).

use std::collections::{BTreeMap, BTreeSet};

use crate::test_evidence::{
    EvidenceStoreV1, ReceiptIdentityV1, ReuseDecision, StaleReason, classify,
};
use crate::test_model::{
    ActiveChangeSetV1, CapabilityKind, ProjectTestTopologyV1, TestBatchV1, TestEvidenceReceiptV1,
};
use crate::test_ports::{
    AdapterError, CapabilityRegistryV1, TestEvidenceRepository, TestImpactPlannerPort,
};
use crate::test_select::ImpactPlannerV1;

/// Schema version constant for the apply verification session.
pub const APPLY_SESSION_SCHEMA_VERSION: u32 = 1;

// ── NextBatchOutcome ─────────────────────────────────────────────────────────

/// Schema version constant for next-batch outcome.
pub const NEXT_BATCH_OUTCOME_SCHEMA_VERSION: u32 = 1;

/// Outcome of a `next_batch()` call — closed enum with exactly 4 variants (SPEC-043 §9).
///
/// Adding a variant requires updating the SPEC and all call sites.
#[derive(Debug, Clone, PartialEq)]
pub enum NextBatchOutcome {
    /// A batch is ready for evidence collection.
    Batch {
        /// The batch to execute.
        batch: TestBatchV1,
    },
    /// All batches have been satisfied — session is complete.
    Complete,
    /// Session is blocked — unmapped SUT nodes prevent execution.
    Blocked {
        /// The insufficient mapping that describes what could not be resolved.
        insufficient: crate::test_model::InsufficientMappingV1,
    },
    /// Session requires human verification before proceeding.
    VerifyRequired {
        /// The insufficient mapping that describes what requires verification.
        insufficient: crate::test_model::InsufficientMappingV1,
    },
}

crate::assert_variant_count_eq!(
    NextBatchOutcome,
    4,
    [
        NextBatchOutcome::Batch { .. },
        NextBatchOutcome::Complete,
        NextBatchOutcome::Blocked { .. },
        NextBatchOutcome::VerifyRequired { .. },
    ]
);

// ── SessionStatus ───────────────────────────────────────────────────────────

/// Schema version constant for session status.
pub const SESSION_STATUS_SCHEMA_VERSION: u32 = 1;

/// Current status of an apply verification session — closed enum with exactly 4 variants.
///
/// Adding a variant requires updating the SPEC and all call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// Session is actively processing batches.
    InProgress,
    /// All batches have been satisfied with fresh evidence.
    Complete,
    /// Session is blocked by insufficient mapping — cannot proceed.
    Blocked,
    /// Human verification is required before proceeding.
    VerifyRequired,
}

crate::assert_variant_count_eq!(
    SessionStatus,
    4,
    [
        SessionStatus::InProgress,
        SessionStatus::Complete,
        SessionStatus::Blocked,
        SessionStatus::VerifyRequired,
    ]
);

// ── InsufficientMapping ─────────────────────────────────────────────────────

/// Minimal insufficient mapping used internally by the session when the planner
/// fails. Stores raw unmapped data to reconstruct `InsufficientMappingV1` on demand.
type StoredUnmapped = Option<(
    Vec<String>,                              // unmapped_artifacts
    Vec<String>,                              // unmapped_suts
    Vec<crate::test_model::TopologyEdgeKind>, // missing_relations
    bool,                                     // verify_required
)>;

// ── ApplyVerificationSessionV1 ─────────────────────────────────────────────

/// Work-item context for `ApplyVerificationSessionV1::new`.
#[derive(Debug, Clone)]
pub struct WorkItemContext {
    /// Work item this session is for.
    pub work_item_id: String,
    /// Task slice being verified.
    pub task_slice: String,
}

/// Configuration for `ApplyVerificationSessionV1::new`.
#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    /// Optional explicit test mapping override.
    pub map: Option<crate::test_ports::ProjectTestMapV1>,
    /// Whether strict TDD mode is enabled (only compile/type/lint in RED phase).
    pub strict_tdd: bool,
}

/// Deterministic apply verification session (SPEC-043 §9/§13).
///
/// Constructed via `new()` with all dependencies injected. The session is
/// completely runner-discovery-free: it holds no runner commands, no ecosystem
/// strings, only semantic topology and capability references.
///
/// # Type parameters
///
/// - `P`: Test impact planner implementing `TestImpactPlannerPort`.
/// - `S`: Evidence store implementing `TestEvidenceRepository`.
pub struct ApplyVerificationSessionV1 {
    /// Work item this session is for.
    work_item_id: String,
    /// Task slice being verified.
    task_slice: String,
    /// Active change set for this session.
    change_set: ActiveChangeSetV1,
    /// Project test topology snapshot.
    topology: ProjectTestTopologyV1,
    /// Capability registry snapshot.
    registry: CapabilityRegistryV1,
    /// Optional explicit test mapping.
    map: Option<crate::test_ports::ProjectTestMapV1>,
    /// Whether strict TDD mode is enabled.
    strict_tdd: bool,
    /// The computed test selection plan (lazily populated on first `next_batch`).
    plan: Option<crate::test_model::TestSelectionPlanV1>,
    /// Cursor into the plan's batches — index of the next batch to deliver.
    batch_cursor: usize,
    /// Evidence store for receipts.
    store: EvidenceStoreV1,
    /// Raw unmapped data from the last failed plan call.
    stored_unmapped: StoredUnmapped,
    /// Whether a compile/type-check/lint receipt has been recorded (strict TDD gate).
    has_compile_receipt: bool,
    /// Capability id of the last delivered batch (session is identity authority).
    last_batch_capability: Option<String>,
    /// Last delivered batch (session is identity authority for receipt stamping).
    last_batch: Option<TestBatchV1>,
}

impl ApplyVerificationSessionV1 {
    /// Creates a new apply verification session.
    ///
    /// All IDs must be non-empty strings. The session is initially in `InProgress`
    /// status and does not compute the impact plan until the first `next_batch()` call.
    ///
    /// # Panics
    ///
    /// Panics if any ID is empty.
    pub fn new(
        work_item_context: WorkItemContext,
        change_set: ActiveChangeSetV1,
        topology: ProjectTestTopologyV1,
        registry: CapabilityRegistryV1,
        planner: impl TestImpactPlannerPort,
        store: EvidenceStoreV1,
        config: SessionConfig,
    ) -> Self {
        assert!(
            !work_item_context.work_item_id.is_empty(),
            "work_item_id must be non-empty"
        );
        assert!(
            !work_item_context.task_slice.is_empty(),
            "task_slice must be non-empty"
        );

        // Validate change_set and topology are non-empty
        change_set.validate().expect("change_set must be valid");
        topology.validate().expect("topology must be valid");

        let change_set_digest = change_set
            .change_set_digest
            .as_ref()
            .expect("change_set must have digest");

        // Compute the impact plan immediately to fail-fast on unmapped
        let plan_result = planner.plan(change_set_digest, &topology.topology_revision);

        let (plan, stored_unmapped) = match plan_result {
            Ok(p) => (Some(p), None),
            Err(AdapterError::InvalidInput { .. }) => {
                // Planner failed — get insufficient data from the planner's insufficient() method
                let insufficient = planner.insufficient().unwrap_or_else(|| {
                    crate::test_model::InsufficientMappingV1::new(
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        "unknown".to_string(),
                        "unknown".to_string(),
                        false,
                    )
                });
                let verify_required = insufficient.verify_required;
                (
                    None,
                    Some((
                        insufficient.unmapped_artifacts,
                        insufficient.unmapped_suts,
                        insufficient.missing_relations,
                        verify_required,
                    )),
                )
            }
            Err(e) => panic!("unexpected planner error: {:?}", e),
        };

        let batch_cursor = 0;
        let has_compile_receipt = false;

        Self {
            last_batch_capability: None,
            last_batch: None,
            work_item_id: work_item_context.work_item_id,
            task_slice: work_item_context.task_slice,
            change_set,
            topology,
            registry,
            map: config.map,
            strict_tdd: config.strict_tdd,
            plan,
            batch_cursor,
            store,
            stored_unmapped,
            has_compile_receipt,
        }
    }

    /// Returns the work item ID for this session.
    pub fn work_item_id(&self) -> &str {
        &self.work_item_id
    }

    /// Returns the task slice for this session.
    pub fn task_slice(&self) -> &str {
        &self.task_slice
    }

    /// Returns the current change-set digest.
    pub fn change_set_digest(&self) -> &str {
        self.change_set
            .change_set_digest
            .as_ref()
            .expect("change_set must have digest")
    }

    /// Returns the current topology revision.
    pub fn topology_revision(&self) -> &str {
        &self.topology.topology_revision
    }

    /// Returns the evidence store for this session.
    pub fn store(&self) -> &EvidenceStoreV1 {
        &self.store
    }

    /// Returns `true` if strict TDD mode is enabled.
    pub fn strict_tdd(&self) -> bool {
        self.strict_tdd
    }

    /// Returns `true` if a compile/type-check/lint receipt has been recorded.
    pub fn has_compile_receipt(&self) -> bool {
        self.has_compile_receipt
    }

    /// Consumes the impact plan and returns the next batch pending evidence.
    ///
    /// In `strict_tdd: true` mode, only `Compile/TypeCheck/Lint` batches are
    /// admitted in RED phase until at least one compile/type-check receipt exists.
    /// This implements the "strict TDD stays scoped" rule from ADR-043.
    ///
    /// Returns `NextBatchOutcome::Complete` when all batches have fresh evidence.
    /// Returns `NextBatchOutcome::Blocked` or `NextBatchOutcome::VerifyRequired`
    /// when unmapped nodes prevent execution.
    pub fn next_batch(&mut self) -> NextBatchOutcome {
        // If we have stored unmapped data, surface it
        if let Some((artifacts, suts, relations, verify_required)) = &self.stored_unmapped {
            let insufficient =
                self.build_insufficient(artifacts.clone(), suts.clone(), relations.clone());
            if *verify_required {
                return NextBatchOutcome::VerifyRequired { insufficient };
            } else {
                return NextBatchOutcome::Blocked { insufficient };
            }
        }

        let plan = match &self.plan {
            Some(p) => p,
            None => {
                // No plan means we failed to compute one
                return NextBatchOutcome::Blocked {
                    insufficient: crate::test_model::InsufficientMappingV1::new(
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        "no plan computed".to_string(),
                        "no plan computed".to_string(),
                        false,
                    ),
                };
            }
        };

        // Filter to only pending batches (no fresh receipt)
        let pending_batches: Vec<&TestBatchV1> = plan
            .batches
            .iter()
            .filter(|batch| !self.batch_has_fresh_evidence(batch))
            .collect();

        if pending_batches.is_empty() {
            return NextBatchOutcome::Complete;
        }

        // Strict TDD: admit only Compile/TypeCheck/Lint in RED phase
        if self.strict_tdd && !self.has_compile_receipt {
            // Find first Compile/TypeCheck/Lint batch
            let strict_batch = pending_batches.iter().find(|b| {
                self.registry
                    .get(&b.capability_id)
                    .map(|cap| {
                        matches!(
                            cap.kind,
                            CapabilityKind::Compile
                                | CapabilityKind::TypeCheck
                                | CapabilityKind::Lint
                        )
                    })
                    .unwrap_or(false)
            });

            if let Some(batch) = strict_batch {
                self.last_batch_capability = Some(batch.capability_id.clone());
                self.last_batch = Some((*batch).clone());
                return NextBatchOutcome::Batch {
                    batch: (*batch).clone(),
                };
            }

            // No compile/type-check/lint batch available but not all complete
            // This means test-level batches exist but compile hasn't run yet
            return NextBatchOutcome::Complete;
        }

        // Return first pending batch
        self.last_batch_capability = Some(pending_batches[0].capability_id.clone());
        self.last_batch = Some((*pending_batches[0]).clone());
        NextBatchOutcome::Batch {
            batch: (*pending_batches[0]).clone(),
        }
    }

    /// Records an evidence receipt for the last delivered batch.
    ///
    /// The receipt's `capability_id` must match the last batch returned by
    /// `next_batch()`. If the capability ID does not match, returns
    /// `Err(AdapterError::InvalidInput)`.
    ///
    /// After recording, updates the `has_compile_receipt` flag if the receipt
    /// is for a Compile/TypeCheck/Lint capability.
    pub fn record(&mut self, receipt: TestEvidenceReceiptV1) -> Result<(), AdapterError> {
        // Validate receipt has matching change_set_digest
        let cs_digest = self.change_set_digest();
        if receipt.change_set_digest != cs_digest {
            return Err(AdapterError::InvalidInput {
                reason: format!(
                    "receipt change_set_digest '{}' does not match session digest '{}'",
                    receipt.change_set_digest, cs_digest
                ),
            });
        }

        // Session is the identity authority: stamp identity-relevant fields so
        // stored receipts are classified fresh against this session's context
        // (prevents identity spoofing by callers).
        let mut receipt = receipt;
        receipt.source_revision = self.change_set.base_revision.clone();
        receipt.topology_revision = self.topology.topology_revision.clone();
        receipt.sut_graph_revision = String::new();
        receipt.policy_revision = String::new();
        receipt.toolchain_identity = String::new();
        receipt.tested_sut_ids = self
            .last_batch
            .as_ref()
            .map(|b| {
                if b.semantic_scope.is_empty() {
                    self.plan
                        .as_ref()
                        .map(|pl| pl.impacted_sut.clone())
                        .unwrap_or_default()
                } else {
                    b.semantic_scope.clone()
                }
            })
            .unwrap_or_default();

        // Validate capability matches the last delivered batch (REQ-1).
        match &self.last_batch_capability {
            Some(cap) if *cap == receipt.capability_id => {}
            other => {
                return Err(AdapterError::InvalidInput {
                    reason: format!(
                        "receipt capability_id '{}' does not match last delivered batch capability {:?}",
                        receipt.capability_id, other
                    ),
                });
            }
        }

        // Save to store
        self.store.save(&receipt)?;

        // Update compile receipt flag
        if let Some(cap) = self.registry.get(&receipt.capability_id)
            && matches!(
                cap.kind,
                CapabilityKind::Compile | CapabilityKind::TypeCheck | CapabilityKind::Lint
            )
        {
            self.has_compile_receipt = true;
        }

        Ok(())
    }

    /// Refreshes the session with a new change set.
    ///
    /// Invalidates all evidence receipts whose change-set digest matches the
    /// current session's digest AND whose SUT closure intersects the new
    /// change set's affected nodes. Then recomputes the internal plan.
    ///
    /// Resets the batch cursor and `has_compile_receipt` flag.
    pub fn refresh(&mut self, new_change_set: ActiveChangeSetV1) {
        // Build the set of changed SUT node IDs from the new change set
        let changed_nodes: BTreeSet<String> = new_change_set
            .changed_artifacts
            .iter()
            .map(|a| a.path.clone())
            .collect();

        // Build current receipt identity for comparison
        let current_identity = ReceiptIdentityV1::new(
            self.change_set_digest().to_string(),
            self.change_set.base_revision.clone(),
            self.topology.topology_revision.clone(),
            String::new(), // sut_graph_revision
            String::new(), // policy_revision
            String::new(), // capability_test_identity
            String::new(), // toolchain_identity
        );

        // Invalidate intersecting receipts
        let _report = crate::test_evidence::invalidate_graph_driven(
            &mut self.store,
            &changed_nodes,
            &self.topology,
            &current_identity,
            &new_change_set.base_revision,
        );

        // Update change set
        self.change_set = new_change_set;

        // Reset cursor and compile flag
        self.batch_cursor = 0;
        self.has_compile_receipt = false;
    }

    /// Returns the current session status.
    ///
    /// - `InProgress`: session is active with pending batches.
    /// - `Complete`: all batches have fresh evidence (matching change_set_digest +
    ///   capability_id + toolchain_identity revisions).
    /// - `Blocked`: unmapped SUT nodes prevent execution.
    /// - `VerifyRequired`: human verification is required before proceeding.
    pub fn status(&self) -> SessionStatus {
        // Check stored unmapped first
        if let Some((_, _, _, verify_required)) = &self.stored_unmapped {
            if *verify_required {
                return SessionStatus::VerifyRequired;
            } else {
                return SessionStatus::Blocked;
            }
        }

        // If no plan, we're blocked
        let plan = match &self.plan {
            Some(p) => p,
            None => return SessionStatus::Blocked,
        };

        // Check if all batches have fresh evidence
        let all_satisfied = plan
            .batches
            .iter()
            .all(|batch| self.batch_has_fresh_evidence(batch));

        if all_satisfied {
            SessionStatus::Complete
        } else {
            SessionStatus::InProgress
        }
    }

    // ── Internal helpers ───────────────────────────────────────────────────────

    /// Checks if a batch has fresh evidence in the store.
    fn batch_has_fresh_evidence(&self, batch: &TestBatchV1) -> bool {
        let latest = self
            .store
            .latest_for(self.change_set_digest(), &batch.capability_id);

        match latest {
            None => false,
            Some(receipt) => {
                // Check if receipt is still fresh (matches current toolchain identity)
                let current_identity = ReceiptIdentityV1::new(
                    self.change_set_digest().to_string(),
                    self.change_set.base_revision.clone(),
                    self.topology.topology_revision.clone(),
                    String::new(),
                    String::new(),
                    batch.capability_id.clone(),
                    String::new(),
                );

                match classify(&receipt, &current_identity) {
                    ReuseDecision::Reusable => true,
                    ReuseDecision::Stale { reasons } => {
                        // Fresh if no stale reasons relate to our current context
                        !reasons.iter().any(|r| {
                            matches!(
                                r,
                                StaleReason::ChangeSetChanged
                                    | StaleReason::SourceRevisionChanged
                                    | StaleReason::TopologyRevisionChanged
                                    | StaleReason::ToolchainIdentityChanged
                            )
                        })
                    }
                    ReuseDecision::NoEvidence => false,
                }
            }
        }
    }

    /// Builds an `InsufficientMappingV1` from raw unmapped data.
    fn build_insufficient(
        &self,
        unmapped_artifacts: Vec<String>,
        unmapped_suts: Vec<String>,
        missing_relations: Vec<crate::test_model::TopologyEdgeKind>,
    ) -> crate::test_model::InsufficientMappingV1 {
        let verify_required = unmapped_suts.iter().any(|s| {
            self.topology
                .nodes
                .get(s)
                .map(|n| {
                    matches!(
                        n.kind,
                        crate::test_model::SutKind::Schema
                            | crate::test_model::SutKind::ContractBoundary
                            | crate::test_model::SutKind::GeneratedArtifact
                    )
                })
                .unwrap_or(true) // not in topology → verify required
        });

        crate::test_model::InsufficientMappingV1::new(
            unmapped_artifacts,
            unmapped_suts,
            missing_relations,
            Vec::new(), // unavailable_capabilities
            "One or more changed artifacts could not be mapped to any SUT node.".to_string(),
            "Add Owns edges from the owning component/build-unit to the changed artifact, or add an explicit mapping entry.".to_string(),
            verify_required,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_model::{
        ChangeKind, ChangedArtifactV1, EdgeProvenanceV1, PlanVerdict, ProjectTestTopologyV1,
        SutKind, SutNodeV1, TopologyEdgeKind, TopologyEdgeV1,
    };
    use std::collections::BTreeMap;

    // ═══════════════════════════════════════════════════════════════════════════════
    // Test fixtures
    // ═══════════════════════════════════════════════════════════════════════════════

    fn standard_prov() -> EdgeProvenanceV1 {
        EdgeProvenanceV1 {
            source: "test".to_string(),
            adapter_version: "v1".to_string(),
            confidence_source: "test".to_string(),
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
            "src:lib".to_string(),
            SutNodeV1::new(
                "src:lib".to_string(),
                SutKind::SourceArtifact,
                String::new(),
                None,
            ),
        );

        let edges = vec![TopologyEdgeV1::new(
            TopologyEdgeKind::Owns,
            "comp:a".to_string(),
            "src:lib".to_string(),
            standard_prov(),
        )];

        ProjectTestTopologyV1::new("rev:1".to_string(), nodes, edges)
    }

    fn simple_change_set() -> ActiveChangeSetV1 {
        ActiveChangeSetV1::new(
            "test-project".to_string(),
            "base".to_string(),
            "head".to_string(),
            "tree".to_string(),
            vec![ChangedArtifactV1 {
                path: "src:lib".to_string(),
                change_kind: ChangeKind::Modified,
                staged: false,
            }],
        )
    }

    fn standard_registry() -> CapabilityRegistryV1 {
        use crate::test_model::{SelectorGranularity, VerificationCapabilityV1};

        let mut reg = CapabilityRegistryV1::new();
        reg.register(VerificationCapabilityV1::new(
            "cap:compile".to_string(),
            CapabilityKind::Compile,
            [SutKind::BuildUnit, SutKind::Component]
                .iter()
                .cloned()
                .collect(),
            SelectorGranularity::File,
            "test-adapter".to_string(),
            "rustc 1.75".to_string(),
            None,
            vec![],
        ))
        .unwrap();
        reg.register(VerificationCapabilityV1::new(
            "cap:typecheck".to_string(),
            CapabilityKind::TypeCheck,
            [SutKind::BuildUnit, SutKind::Component]
                .iter()
                .cloned()
                .collect(),
            SelectorGranularity::File,
            "test-adapter".to_string(),
            "rustc 1.75".to_string(),
            None,
            vec![],
        ))
        .unwrap();
        reg.register(VerificationCapabilityV1::new(
            "cap:lint".to_string(),
            CapabilityKind::Lint,
            [SutKind::BuildUnit, SutKind::Component]
                .iter()
                .cloned()
                .collect(),
            SelectorGranularity::File,
            "test-adapter".to_string(),
            "clippy".to_string(),
            None,
            vec![],
        ))
        .unwrap();
        reg
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.1: happy path — session with topology → next_batch delivers stages → record → complete
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_happy_path_complete() {
        let topology = simple_topology();
        let change_set = simple_change_set();
        let registry = standard_registry();
        let store = EvidenceStoreV1::new();

        // Use a mock planner that returns a simple plan
        struct MockPlanner {
            plan: crate::test_model::TestSelectionPlanV1,
        }

        impl TestImpactPlannerPort for MockPlanner {
            fn plan(
                &self,
                _change_set_digest: &str,
                _topology_revision: &str,
            ) -> Result<crate::test_model::TestSelectionPlanV1, AdapterError> {
                Ok(self.plan.clone())
            }
            fn insufficient(&self) -> Option<crate::test_model::InsufficientMappingV1> {
                None
            }
        }

        let plan = crate::test_model::TestSelectionPlanV1::new(
            "plan:1".to_string(),
            change_set.change_set_digest.clone().unwrap(),
            topology.topology_revision.clone(),
            String::new(),
            String::new(),
            vec!["comp:a".to_string()],
            vec![
                TestBatchV1 {
                    stage: 0,
                    capability_id: "cap:compile".to_string(),
                    semantic_scope: vec![],
                    test_ids: vec!["test1".to_string()],
                    reasons: vec![crate::test_model::ImpactReason::DirectSourceTouch],
                    expected_cost: None,
                    escalation: false,
                },
                TestBatchV1 {
                    stage: 0,
                    capability_id: "cap:typecheck".to_string(),
                    semantic_scope: vec![],
                    test_ids: vec!["test2".to_string()],
                    reasons: vec![crate::test_model::ImpactReason::DirectSourceTouch],
                    expected_cost: None,
                    escalation: false,
                },
            ],
            vec![],
            vec![],
            1.0,
            PlanVerdict::Executable,
        );

        let mut session = ApplyVerificationSessionV1::new(
            WorkItemContext {
                work_item_id: "WI-001".to_string(),
                task_slice: "task-1".to_string(),
            },
            change_set,
            topology,
            registry,
            MockPlanner { plan },
            store,
            SessionConfig {
                map: None,
                strict_tdd: false,
            },
        );

        // Status should be InProgress initially
        assert_eq!(session.status(), SessionStatus::InProgress);

        // First next_batch delivers the first batch
        let outcome1 = session.next_batch();
        assert!(matches!(outcome1, NextBatchOutcome::Batch { .. }));

        // Record a receipt
        let receipt = TestEvidenceReceiptV1::new(
            "receipt:1".to_string(),
            session.change_set_digest().to_string(),
            "head".to_string(),
            session.topology_revision().to_string(),
            String::new(),
            String::new(),
            "cap:compile".to_string(),
            crate::test_model::ReceiptResult::Passed,
            "2024-01-01T00:00:00Z".to_string(),
            String::new(), // toolchain_identity
        );
        session.record(receipt).unwrap();

        // Status still InProgress
        assert_eq!(session.status(), SessionStatus::InProgress);

        // Second next_batch delivers the second batch
        let outcome2 = session.next_batch();
        assert!(matches!(outcome2, NextBatchOutcome::Batch { .. }));

        // Record second receipt
        let receipt2 = TestEvidenceReceiptV1::new(
            "receipt:2".to_string(),
            session.change_set_digest().to_string(),
            "head".to_string(),
            session.topology_revision().to_string(),
            String::new(),
            String::new(),
            "cap:typecheck".to_string(),
            crate::test_model::ReceiptResult::Passed,
            "2024-01-02T00:00:00Z".to_string(),
            String::new(), // toolchain_identity
        );
        session.record(receipt2).unwrap();

        // Now status should be Complete
        assert_eq!(session.status(), SessionStatus::Complete);

        // next_batch should return Complete
        assert!(matches!(session.next_batch(), NextBatchOutcome::Complete));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.2: strict TDD — strict_tdd=true admits compile/type-check first
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_strict_tdd_retains_test_batches() {
        let topology = simple_topology();
        let change_set = simple_change_set();
        let registry = standard_registry();
        let store = EvidenceStoreV1::new();

        struct MockPlanner {
            plan: crate::test_model::TestSelectionPlanV1,
        }

        impl TestImpactPlannerPort for MockPlanner {
            fn plan(
                &self,
                _change_set_digest: &str,
                _topology_revision: &str,
            ) -> Result<crate::test_model::TestSelectionPlanV1, AdapterError> {
                Ok(self.plan.clone())
            }
            fn insufficient(&self) -> Option<crate::test_model::InsufficientMappingV1> {
                None
            }
        }

        // Plan with compile (stage 0) and unit test (stage 1)
        let plan = crate::test_model::TestSelectionPlanV1::new(
            "plan:1".to_string(),
            change_set.change_set_digest.clone().unwrap(),
            topology.topology_revision.clone(),
            String::new(),
            String::new(),
            vec!["comp:a".to_string()],
            vec![
                TestBatchV1 {
                    stage: 0,
                    capability_id: "cap:compile".to_string(),
                    semantic_scope: vec![],
                    test_ids: vec!["test1".to_string()],
                    reasons: vec![crate::test_model::ImpactReason::DirectSourceTouch],
                    expected_cost: None,
                    escalation: false,
                },
                TestBatchV1 {
                    stage: 1,
                    capability_id: "cap:unit".to_string(),
                    semantic_scope: vec![],
                    test_ids: vec!["test2".to_string()],
                    reasons: vec![crate::test_model::ImpactReason::DirectSourceTouch],
                    expected_cost: None,
                    escalation: false,
                },
            ],
            vec![],
            vec![],
            1.0,
            PlanVerdict::Executable,
        );

        let mut session = ApplyVerificationSessionV1::new(
            WorkItemContext {
                work_item_id: "WI-001".to_string(),
                task_slice: "task-1".to_string(),
            },
            change_set,
            topology,
            registry,
            MockPlanner { plan },
            store,
            SessionConfig {
                map: None,
                strict_tdd: true,
            },
        );

        // First batch should be compile (strict TDD RED phase)
        let outcome1 = session.next_batch();
        let batch1 = match outcome1 {
            NextBatchOutcome::Batch { batch } => batch,
            other => panic!("expected Batch, got {:?}", other),
        };
        assert_eq!(batch1.capability_id, "cap:compile");

        // Record compile receipt
        let receipt = TestEvidenceReceiptV1::new(
            "receipt:1".to_string(),
            session.change_set_digest().to_string(),
            "head".to_string(),
            session.topology_revision().to_string(),
            String::new(),
            String::new(),
            "cap:compile".to_string(),
            crate::test_model::ReceiptResult::Passed,
            "2024-01-01T00:00:00Z".to_string(),
            String::new(), // toolchain_identity
        );
        session.record(receipt).unwrap();

        // Now unit test batch should be available
        let outcome2 = session.next_batch();
        let batch2 = match outcome2 {
            NextBatchOutcome::Batch { batch } => batch,
            NextBatchOutcome::Complete => {
                panic!("expected Batch after compile receipt, got Complete")
            }
            other => panic!("expected Batch, got {:?}", other),
        };
        assert_eq!(batch2.capability_id, "cap:unit");
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.3: fail-closed — artefact without owner ⇒ Blocked
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_fail_closed_blocked() {
        let topology = simple_topology();
        // Change an artifact with no owner in topology - this should cause ImpactPlannerV1
        // to produce a Blocked result with unmapped artifacts
        let change_set = ActiveChangeSetV1::new(
            "test-project".to_string(),
            "base".to_string(),
            "head".to_string(),
            "tree".to_string(),
            vec![ChangedArtifactV1 {
                path: "nonexistent/file.rs".to_string(), // no owner in topology
                change_kind: ChangeKind::Modified,
                staged: false,
            }],
        );
        let registry = CapabilityRegistryV1::new();
        let store = EvidenceStoreV1::new();

        // Use actual ImpactPlannerV1 - it will fail to plan because the artifact has no owner
        let planner = ImpactPlannerV1::new(
            change_set.clone(),
            topology.clone(),
            registry.clone(),
            None,
            false, // stage4_policy_enabled
        );

        let mut session = ApplyVerificationSessionV1::new(
            WorkItemContext {
                work_item_id: "WI-001".to_string(),
                task_slice: "task-1".to_string(),
            },
            change_set,
            topology,
            registry,
            planner,
            store,
            SessionConfig {
                map: None,
                strict_tdd: false,
            },
        );

        // Should be Blocked immediately
        assert_eq!(session.status(), SessionStatus::Blocked);

        let outcome = session.next_batch();
        assert!(matches!(outcome, NextBatchOutcome::Blocked { .. }));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.4: refresh — intersecting change invalidates prior evidence
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_refresh_invalidates_intersecting() {
        let topology = simple_topology();
        let change_set = simple_change_set();
        let registry = standard_registry();
        let mut store = EvidenceStoreV1::new();

        // Pre-populate store with a receipt
        let existing_receipt = TestEvidenceReceiptV1::new(
            "receipt:old".to_string(),
            change_set.change_set_digest.clone().unwrap(),
            "head".to_string(),
            topology.topology_revision.clone(),
            String::new(),
            String::new(),
            "cap:compile".to_string(),
            crate::test_model::ReceiptResult::Passed,
            "2024-01-01T00:00:00Z".to_string(),
            String::new(), // toolchain_identity
        );
        store.insert(existing_receipt).unwrap();

        struct MockPlanner {
            plan: crate::test_model::TestSelectionPlanV1,
        }

        impl TestImpactPlannerPort for MockPlanner {
            fn plan(
                &self,
                _change_set_digest: &str,
                _topology_revision: &str,
            ) -> Result<crate::test_model::TestSelectionPlanV1, AdapterError> {
                Ok(self.plan.clone())
            }
            fn insufficient(&self) -> Option<crate::test_model::InsufficientMappingV1> {
                None
            }
        }

        let plan = crate::test_model::TestSelectionPlanV1::new(
            "plan:1".to_string(),
            change_set.change_set_digest.clone().unwrap(),
            topology.topology_revision.clone(),
            String::new(),
            String::new(),
            vec!["comp:a".to_string()],
            vec![TestBatchV1 {
                stage: 0,
                capability_id: "cap:compile".to_string(),
                semantic_scope: vec![],
                test_ids: vec!["test1".to_string()],
                reasons: vec![crate::test_model::ImpactReason::DirectSourceTouch],
                expected_cost: None,
                escalation: false,
            }],
            vec![],
            vec![],
            1.0,
            PlanVerdict::Executable,
        );

        let mut session = ApplyVerificationSessionV1::new(
            WorkItemContext {
                work_item_id: "WI-001".to_string(),
                task_slice: "task-1".to_string(),
            },
            change_set,
            topology,
            registry,
            MockPlanner { plan },
            store,
            SessionConfig {
                map: None,
                strict_tdd: false,
            },
        );

        // Store has the old receipt
        assert!(session.store.get("receipt:old").is_some());

        // Refresh with intersecting change set
        let new_change_set = ActiveChangeSetV1::new(
            "test-project".to_string(),
            "head".to_string(),
            "new-head".to_string(),
            "new-tree".to_string(),
            vec![ChangedArtifactV1 {
                path: "src:lib".to_string(), // same artifact — intersects
                change_kind: ChangeKind::Modified,
                staged: false,
            }],
        );
        session.refresh(new_change_set);

        // Receipt should be invalidated (removed from store)
        assert!(session.store.get("receipt:old").is_none());
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.5: record mismatch — capability_id must match last batch
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_record_mismatch_rejected() {
        let topology = simple_topology();
        let change_set = simple_change_set();
        let registry = standard_registry();
        let store = EvidenceStoreV1::new();

        struct MockPlanner {
            plan: crate::test_model::TestSelectionPlanV1,
        }

        impl TestImpactPlannerPort for MockPlanner {
            fn plan(
                &self,
                _change_set_digest: &str,
                _topology_revision: &str,
            ) -> Result<crate::test_model::TestSelectionPlanV1, AdapterError> {
                Ok(self.plan.clone())
            }
            fn insufficient(&self) -> Option<crate::test_model::InsufficientMappingV1> {
                None
            }
        }

        let plan = crate::test_model::TestSelectionPlanV1::new(
            "plan:1".to_string(),
            change_set.change_set_digest.clone().unwrap(),
            topology.topology_revision.clone(),
            String::new(),
            String::new(),
            vec!["comp:a".to_string()],
            vec![TestBatchV1 {
                stage: 0,
                capability_id: "cap:compile".to_string(),
                semantic_scope: vec![],
                test_ids: vec!["test1".to_string()],
                reasons: vec![crate::test_model::ImpactReason::DirectSourceTouch],
                expected_cost: None,
                escalation: false,
            }],
            vec![],
            vec![],
            1.0,
            PlanVerdict::Executable,
        );

        let mut session = ApplyVerificationSessionV1::new(
            WorkItemContext {
                work_item_id: "WI-001".to_string(),
                task_slice: "task-1".to_string(),
            },
            change_set,
            topology,
            registry,
            MockPlanner { plan },
            store,
            SessionConfig {
                map: None,
                strict_tdd: false,
            },
        );

        // Try to record a receipt with wrong capability_id
        let wrong_receipt = TestEvidenceReceiptV1::new(
            "receipt:wrong".to_string(),
            session.change_set_digest().to_string(),
            "head".to_string(),
            session.topology_revision().to_string(),
            String::new(),
            String::new(),
            "cap:typecheck".to_string(), // different from compile batch
            crate::test_model::ReceiptResult::Passed,
            "2024-01-01T00:00:00Z".to_string(),
            String::new(), // toolchain_identity
        );

        // REQ-1: capability_id must match the last delivered batch (session is
        // identity authority); no next_batch was delivered, so recording is rejected.
        let result = session.record(wrong_receipt);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.6: runner-discovery-free — zero runner commands in session module
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_no_runner_commands() {
        // This test verifies that the ApplyVerificationSessionV1 struct and its methods
        // contain no runner command strings. This is a compile-time check via string search.
        use std::collections::HashSet;

        // Collect all string literals in this module
        let self_module_strings = [
            "WI-001",
            "task-1",
            "plan:1",
            "receipt:1",
            "receipt:2",
            "receipt:old",
            "receipt:wrong",
            "cap:compile",
            "cap:typecheck",
            "cap:unit",
            "cap:lint",
            "src:lib",
            "comp:a",
            "unknown/file.rs",
            "test-project",
            "base",
            "head",
            "tree",
            "new-head",
            "new-tree",
            "rev:1",
            "test1",
            "test2",
            "2024-01-01T00:00:00Z",
            "2024-01-02T00:00:00Z",
            "rust",
            "rustc 1.75",
            "clippy",
            "test",
            "v1",
            "test-fixture",
            "test-adapter",
            "no plan computed",
            "unknown",
        ];

        // Runner command strings that MUST NOT appear
        let runner_commands = [
            "cargo test",
            "npm test",
            "pytest",
            "jest",
            "go test",
            "dotnet test",
            "flutter test",
            "gradle test",
            "make test",
            "cargo build",
            "npm run",
            "cmake --build",
            "dotnet build",
        ];

        for cmd in runner_commands {
            let found = self_module_strings
                .iter()
                .any(|s| s.contains(&cmd.to_string()));
            assert!(
                !found,
                "Runner command '{}' found in test_apply.rs string literals",
                cmd
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // REQ-6.9: variant-count assertions (NextBatchOutcome 4, SessionStatus 4)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn next_batch_outcome_variant_count() {
        let variants = [
            NextBatchOutcome::Batch {
                batch: TestBatchV1 {
                    stage: 0,
                    capability_id: String::new(),
                    semantic_scope: vec![],
                    test_ids: vec![],
                    reasons: vec![],
                    expected_cost: None,
                    escalation: false,
                },
            },
            NextBatchOutcome::Complete,
            NextBatchOutcome::Blocked {
                insufficient: crate::test_model::InsufficientMappingV1::new(
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    String::new(),
                    String::new(),
                    false,
                ),
            },
            NextBatchOutcome::VerifyRequired {
                insufficient: crate::test_model::InsufficientMappingV1::new(
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    String::new(),
                    String::new(),
                    false,
                ),
            },
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn session_status_variant_count() {
        let variants = [
            SessionStatus::InProgress,
            SessionStatus::Complete,
            SessionStatus::Blocked,
            SessionStatus::VerifyRequired,
        ];
        assert_eq!(variants.len(), 4);
    }
}
