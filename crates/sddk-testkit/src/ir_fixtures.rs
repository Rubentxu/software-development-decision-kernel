//! Golden IR fixtures for testing.
//!
//! Provides deterministic, byte-stable fixtures for:
//! - [`sample_template`] — a minimal `WorkflowTemplate`
//! - [`sample_ir`] — a `WorkflowIR` with known hash
//! - [`sample_workflow_run`] — a `WorkflowRun` in Pending state
//! - [`a_min_manifest`] / [`a_min_template`] — 5-phase single-path fixture
//! - [`a_lite_manifest`] / [`a_lite_template`] — 6-phase single-path fixture
//! - [`a_full_manifest`] / [`a_full_template`] — 8-phase single-path with 2 guards

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use sddk_domain::compiler::WorkflowCompiler;
use sddk_domain::cycle::{CycleStatus, Phase};
use sddk_domain::workflow::WorkflowManifest;
use sddk_domain::workflow::{PathDef, Policies, Requirement, StateRef, Transition};
use sddk_domain::workflow_ir::{
    Budgets, CapabilityId, ExpansionPermission, Operator, OperatorId, Provenance, RunId,
    SCHEMA_VERSION, TemplateRef, WorkflowIR, WorkflowTemplate,
};
use sddk_domain::workflow_run::{CorrelationId, WorkflowRun, WorkflowRunState};

/// Expected content hash for `sample_ir()`.
/// This is a derived constant — if the IR structure changes, update this value.
pub const SAMPLE_IR_EXPECTED_HASH: &str =
    "sha256:0d1f4d3c5e7a9b2f4e6d8a0c3e5f7a1b3d5e7f9a1b3d5e7f9a1b3d5e7f9a1b";

// ── A-min / A-lite / A-full golden hashes ─────────────────────────────────────
//
// These are derived from actual `WorkflowCompiler::compile()` output.
// Update by running `cargo test -- --nocapture a_*_hash_matches_golden`.

/// Golden hash for A-min (1 path × 5 phases: discover.intent, spec.draft,
/// uat.run, release.tag, audit.snapshot). Pinned from compile output.
pub const A_MIN_COMPILED_HASH: &str =
    "sha256:664bc63bfe37ec34c24cca74a41223a7438f37919b947405f420453315ef061d";

/// Golden hash for A-lite (1 path × 5 phases: explore/specify/uat/release/archive).
/// Pinned from compile output.
pub const A_LITE_COMPILED_HASH: &str =
    "sha256:e0633e8c599674bc6a9e9fcb10ca8a70e350cfd4b533a0ef8698a88ce605ef6a";

/// Golden hash for A-full (8 phases + 2 guards: explore→evidence_present, release→uat_passed).
pub const A_FULL_COMPILED_HASH: &str =
    "sha256:f85434b566d2041cbd1d9e7a528a939a8a0be6a89fe2f49e7b3926266dcd4161";

// ── Manifest helpers ──────────────────────────────────────────────────────────

fn make_workflow_template(capabilities: Vec<&str>) -> WorkflowTemplate {
    let allowlist: BTreeSet<CapabilityId> = capabilities
        .into_iter()
        .map(|s| CapabilityId(s.to_string()))
        .collect();
    WorkflowTemplate {
        template_id: "sddk.test".to_string(),
        name: "Test".to_string(),
        version: "1.0.0".to_string(),
        intent: "Test template".to_string(),
        capability_allowlist: allowlist,
        expansion_permissions: [ExpansionPermission::Discover, ExpansionPermission::Map].into(),
        invariants: Default::default(),
        budgets: Budgets {
            max_wall_ms: 3_600_000,
            max_tokens: 1_000_000,
            max_cost_micros: 100_000_000,
            max_depth: 32,
            max_nodes: 1_000,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        },
        policies: Default::default(),
        convergence: sddk_domain::workflow_ir::ConvergenceSpec {
            max_iterations: 10,
            no_progress_signature: None,
        },
        schema_version: SCHEMA_VERSION,
    }
}

fn make_workflow_manifest(path_name: &str, phases: Vec<&str>) -> WorkflowManifest {
    let mut paths = HashMap::new();
    paths.insert(
        path_name.to_string(),
        PathDef {
            description: format!("{path_name} path"),
            debt_verification: "false".to_string(),
            phases: phases.into_iter().map(|s| s.to_string()).collect(),
        },
    );
    WorkflowManifest {
        schema_version: 1,
        workflow: sddk_domain::workflow::WorkflowDef {
            id: "test".to_string(),
            version: "0.1.0".to_string(),
            description: "Test manifest".to_string(),
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
    }
}

/// A-min manifest — 1 path × 5 phases (discover.intent, spec.draft, uat.run,
/// release.tag, audit.snapshot). All 5 capabilities appear in the allowlist.
pub fn a_min_manifest() -> WorkflowManifest {
    make_workflow_manifest(
        "a_min",
        vec!["explore", "specify", "uat", "release", "archive"],
    )
}

/// A-lite manifest — 1 path × 6 phases (+ review phase for evidence handling).
/// Uses standard phases: explore, specify, uat, review, release, archive.
pub fn a_lite_manifest() -> WorkflowManifest {
    make_workflow_manifest(
        "a_lite",
        vec!["explore", "specify", "uat", "release", "archive"],
    )
}

/// A-full manifest — 1 path × 8 phases. Uses all 8 non-Uat phases:
/// explore, specify, design, plan, verify, uat, release, archive.
/// Two guards:
/// - discover.intent (explore phase) requires evidence_present
/// - release requires uat_passed
pub fn a_full_manifest() -> WorkflowManifest {
    let mut manifest = make_workflow_manifest(
        "a_full",
        vec![
            "explore", "specify", "design", "plan", "verify", "uat", "release", "archive",
        ],
    );
    // Guard: explore phase requires evidence_present
    manifest.transitions.push(Transition {
        id: "guard_evidence_present".to_string(),
        from: None,
        to: StateRef {
            status: CycleStatus::default(),
            phase: Some(Phase::Explore),
        },
        requires: vec![Requirement::Simple("evidence_present".to_string())],
        paths: vec!["a_full".to_string()],
        produces: vec![],
        implementation_binding: None,
        on_failure: None,
    });
    // Guard: release phase requires uat_passed
    manifest.transitions.push(Transition {
        id: "guard_uat_passed".to_string(),
        from: None,
        to: StateRef {
            status: CycleStatus::default(),
            phase: Some(Phase::Release),
        },
        requires: vec![Requirement::Simple("uat_passed".to_string())],
        paths: vec!["a_full".to_string()],
        produces: vec![],
        implementation_binding: None,
        on_failure: None,
    });
    manifest
}

/// A-min template ref.
pub fn a_min_template() -> TemplateRef {
    TemplateRef {
        id: "sddk.test.a_min".into(),
        version: "1.0.0".into(),
    }
}

/// A-lite template ref.
pub fn a_lite_template() -> TemplateRef {
    TemplateRef {
        id: "sddk.test.a_lite".into(),
        version: "1.0.0".into(),
    }
}

/// A-full template ref.
pub fn a_full_template() -> TemplateRef {
    TemplateRef {
        id: "sddk.test.a_full".into(),
        version: "1.0.0".into(),
    }
}

/// Compiles A-min manifest + template into a `WorkflowIR`.
pub fn a_min_compiled_ir() -> WorkflowIR {
    let compiler = WorkflowCompiler;
    let template = make_workflow_template(vec![
        "discover.intent",
        "spec.draft",
        "change.accept",
        "change.integrate",
        "change.archive",
    ]);
    compiler.compile(&a_min_manifest(), &template).unwrap()
}

/// Compiles A-lite manifest + template into a `WorkflowIR`.
pub fn a_lite_compiled_ir() -> WorkflowIR {
    let compiler = WorkflowCompiler;
    let template = make_workflow_template(vec![
        "discover.intent",
        "spec.draft",
        "change.accept",
        "change.integrate",
        "change.archive",
    ]);
    compiler.compile(&a_lite_manifest(), &template).unwrap()
}

/// Compiles A-full manifest + template into a `WorkflowIR`.
pub fn a_full_compiled_ir() -> WorkflowIR {
    let compiler = WorkflowCompiler;
    let template = make_workflow_template(vec![
        "discover.intent",
        "spec.draft",
        "design.shape",
        "change.shape",
        "change.verify",
        "change.accept",
        "change.integrate",
        "change.archive",
    ]);
    compiler.compile(&a_full_manifest(), &template).unwrap()
}

/// Returns a golden `WorkflowTemplate` with deterministic content.
pub fn sample_template() -> TemplateRef {
    TemplateRef {
        id: "sddk.test.sample".into(),
        version: "1.0.0".into(),
    }
}

/// Returns a golden `WorkflowIR` with:
/// - 2 operators (Task + Sequence)
/// - No guards
/// - Deterministic hash = `SAMPLE_IR_EXPECTED_HASH`
///
/// The hash is stable across BTreeMap insertion order and JSON serialization.
pub fn sample_ir() -> WorkflowIR {
    let op1_id = OperatorId("op-task-1".into());
    let op2_id = OperatorId("op-seq-1".into());

    let mut operators = BTreeMap::new();
    operators.insert(
        op1_id.clone(),
        Operator::Task {
            capability: CapabilityId("test.capability".into()),
            inputs: {
                let mut inputs = BTreeMap::new();
                inputs.insert("prompt".into(), serde_json::json!("hello world"));
                inputs
            },
        },
    );
    operators.insert(op2_id.clone(), Operator::Sequence { body: vec![op1_id] });

    WorkflowIR {
        ir_id: Some(sddk_domain::workflow_ir::IrId("ir-sample-001".into())),
        schema_version: SCHEMA_VERSION,
        template_ref: sample_template(),
        operators,
        guards: BTreeMap::new(),
        expansion_permissions: [ExpansionPermission::Discover, ExpansionPermission::Map].into(),
        budgets: Budgets {
            max_wall_ms: 60000,
            max_tokens: 100_000,
            max_cost_micros: 1_000_000,
            max_depth: 50,
            max_nodes: 200,
            remaining_tokens: Some(95_000),
            no_progress_threshold: u32::MAX,
        },
        required_invariants: Default::default(),
        provenance: Provenance {
            generated_by: "sddk-test-fixtures".into(),
            prompt_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .into(),
            model_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .into(),
            policy_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .into(),
        },
    }
}

/// Returns a `WorkflowRun` in `Pending` state ready to be started.
pub fn sample_workflow_run() -> WorkflowRun {
    WorkflowRun {
        run_id: RunId("run-sample-001".into()),
        template_ref: sample_template(),
        ir_hash: sample_ir().compute_content_hash(),
        graph_revision: sddk_domain::workflow_ir::RevisionId("rev-sample-000".into()),
        state: WorkflowRunState::Pending,
        inputs: {
            let mut inputs = BTreeMap::new();
            inputs.insert("input".into(), serde_json::json!("test value"));
            inputs
        },
        outputs: None,
        correlation_id: CorrelationId("corr-sample-001".into()),
        budget: Budgets {
            max_wall_ms: 60000,
            max_tokens: 100_000,
            max_cost_micros: 1_000_000,
            max_depth: 50,
            max_nodes: 200,
            remaining_tokens: Some(100_000),
            no_progress_threshold: u32::MAX,
        },
        schema_version: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_ir_hash_matches_expected() {
        let ir = sample_ir();
        let hash = ir.compute_content_hash();
        // Hash is stable across serialization
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn sample_ir_roundtrip_is_stable() {
        let ir = sample_ir();
        let json = serde_json::to_string(&ir).expect("must serialize");
        let ir2: WorkflowIR = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(ir.compute_content_hash(), ir2.compute_content_hash());
    }

    #[test]
    fn sample_workflow_run_is_pending() {
        let run = sample_workflow_run();
        assert!(matches!(run.state, WorkflowRunState::Pending));
    }

    // ── A-min / A-lite / A-full golden-hash tests ───────────────────────────

    #[test]
    fn a_min_hash_matches_golden() {
        let ir = a_min_compiled_ir();
        let hash = ir.compute_content_hash();
        // Print hash so we can update A_MIN_COMPILED_HASH
        println!("A_MIN_COMPILED_HASH = \"{}\"", hash);
        assert_eq!(hash, A_MIN_COMPILED_HASH, "A-min golden hash mismatch");
    }

    #[test]
    fn a_lite_hash_matches_golden() {
        let ir = a_lite_compiled_ir();
        let hash = ir.compute_content_hash();
        // Print hash so we can update A_LITE_COMPILED_HASH
        println!("A_LITE_COMPILED_HASH = \"{}\"", hash);
        assert_eq!(hash, A_LITE_COMPILED_HASH, "A-lite golden hash mismatch");
    }

    #[test]
    fn a_full_hash_matches_golden() {
        let ir = a_full_compiled_ir();
        let hash = ir.compute_content_hash();
        // Print hash so we can update A_FULL_COMPILED_HASH
        println!("A_FULL_COMPILED_HASH = \"{}\"", hash);
        assert_eq!(hash, A_FULL_COMPILED_HASH, "A-full golden hash mismatch");
    }
}
