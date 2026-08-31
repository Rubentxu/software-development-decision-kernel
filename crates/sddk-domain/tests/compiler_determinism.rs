//! Proptest: `WorkflowCompiler::compile` is deterministic — same `WorkflowManifest` +
//! `WorkflowTemplate` input → same `compute_content_hash()` output, regardless of
//! insertion order or any other non-deterministic factor.
//!
//! Cycle 3 REQ-K3-002 acceptance scenario 1 (was deferred in cycle 2 verify-report).
//!
//! Strategy: use a small parametric manifest template (single path, 2-7 phases
//! chosen from the 10 standard Phase variants) and verify the hash is
//! deterministic across repeated compile calls. 1000 iterations with
//! deterministic seed.

#![cfg(test)]

use std::collections::{BTreeSet, HashMap};

use proptest::prelude::*;
use sddk_domain::compiler::WorkflowCompiler;
use sddk_domain::workflow::{PathDef, Policies, WorkflowDef, WorkflowManifest};
use sddk_domain::workflow_ir::{
    Budgets, CapabilityId, ConvergenceSpec, ExpansionPermission, Operator, SCHEMA_VERSION,
    WorkflowTemplate,
};

const PHASE_NAMES: &[&str] = &[
    "explore", "specify", "design", "plan", "build", "verify", "uat", "review", "release",
    "archive",
];

#[derive(Debug, Clone)]
struct ProptestFixture {
    manifest: WorkflowManifest,
    template: WorkflowTemplate,
}

fn arb_fixture() -> impl Strategy<Value = ProptestFixture> {
    (0usize..=16, 2usize..=7).prop_map(|(path_id, phase_count)| {
        let phases: Vec<String> = (0..phase_count)
            .map(|i| PHASE_NAMES[i % PHASE_NAMES.len()].to_string())
            .collect();

        let path_id_str = format!("path-{path_id}");
        let mut paths = HashMap::new();
        paths.insert(
            path_id_str.clone(),
            PathDef {
                description: format!("generated path {path_id_str}"),
                debt_verification: "false".to_string(),
                phases,
            },
        );

        let manifest = WorkflowManifest {
            schema_version: 1,
            workflow: WorkflowDef {
                id: format!("test-{path_id}"),
                version: "0.1.0".to_string(),
                description: format!("proptest manifest {path_id}"),
            },
            statuses: vec![],
            phases: vec![],
            paths,
            policies: Policies::default(),
            transitions: vec![],
            artifacts: HashMap::new(),
            gates: HashMap::new(),
            forge: None,
            storage: None,
            project_identity: None,
        };

        let mut caps: BTreeSet<CapabilityId> = BTreeSet::new();
        for path in manifest.paths.values() {
            for phase in &path.phases {
                // Match the compiler's phase_str_to_capability mapping
                let cap_name = match phase.as_str() {
                    "explore" => "discover.intent",
                    "specify" => "spec.draft",
                    "design" => "design.shape",
                    "plan" => "change.shape",
                    "build" => "code.implement",
                    "verify" => "change.verify",
                    "uat" => "change.accept",
                    "review" => "change.review",
                    "release" => "change.integrate",
                    "archive" => "change.archive",
                    _ => continue,
                };
                caps.insert(CapabilityId(cap_name.to_string()));
            }
        }
        let template = WorkflowTemplate {
            template_id: "sddk.test.proptest".into(),
            name: "proptest".into(),
            version: "1.0.0".into(),
            intent: "determinism proptest".into(),
            capability_allowlist: caps,
            expansion_permissions: [ExpansionPermission::Discover].into(),
            invariants: BTreeSet::new(),
            budgets: Budgets {
                max_wall_ms: 60_000,
                max_tokens: 100_000,
                max_cost_micros: 1_000_000,
                max_depth: 32,
                max_nodes: 1_000,
                remaining_tokens: None,
                no_progress_threshold: u32::MAX,
            },
            policies: Default::default(),
            convergence: ConvergenceSpec {
                max_iterations: 10,
                no_progress_signature: None,
            },
            schema_version: SCHEMA_VERSION,
        };

        ProptestFixture { manifest, template }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Given a valid `WorkflowManifest`, *when* compiled twice with the same
    /// `WorkflowTemplate`, *then* `compute_content_hash()` produces the same
    /// value every time.
    #[test]
    fn compile_determinism(seed in 0u32..1000, fixture in arb_fixture()) {
        let compiler = WorkflowCompiler;
        let ir1 = compiler
            .compile(&fixture.manifest, &fixture.template)
            .expect("proptest-generated manifest must compile");
        let ir2 = compiler
            .compile(&fixture.manifest, &fixture.template)
            .expect("proptest-generated manifest must compile");

        let h1 = ir1.compute_content_hash();
        let h2 = ir2.compute_content_hash();

        prop_assert_eq!(h1.clone(), h2, "compile hash must be deterministic (seed={})", seed);
        prop_assert!(h1.starts_with("sha256:"), "hash must start with sha256:");
        prop_assert_eq!(h1.len(), 71, "hash must be 71 chars");

        // Single-path design Decision 6: root operator must be a Sequence
        let has_seq = ir1.operators.values().any(|op| matches!(op, Operator::Sequence { .. }));
        prop_assert!(has_seq, "single-path manifest must compile to a Sequence root");
    }
}
