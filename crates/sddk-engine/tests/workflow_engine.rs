use std::collections::{BTreeMap, BTreeSet, HashMap};

use sddk_domain::StorageError;
use sddk_domain::{ArtifactRef, CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_engine::{
    CycleStartInput, DebtVerificationPolicy, Engine, EngineError, EventContext,
    GateEvaluationInput, GateReceiptRef, TransitionEvidence, TransitionOutcome, WorkflowLoadError,
    WorkflowValidationError, authority::AuthorityContext, load_workflow_path, load_workflow_str,
};
use sddk_storage::{CycleRecord, LedgerEventInput, ProjectRecord, Storage, WorkspaceRecord};
use serde_json::json;
use tempfile::tempdir;

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");
const TIMESTAMP: &str = "2026-08-03T12:00:00Z";

#[test]
fn loads_canonical_workflow_from_string_and_path() {
    let from_string = load_workflow_str(WORKFLOW_YAML).unwrap();
    let directory = tempdir().unwrap();
    let path = directory.path().join("workflow.yaml");
    std::fs::write(&path, WORKFLOW_YAML).unwrap();
    let from_path = load_workflow_path(&path).unwrap();

    assert_eq!(from_path, from_string);
    assert_eq!(from_path.workflow.id, "sddk-standard");

    let invalid = WORKFLOW_YAML.replacen("schema_version: 1", "schema_version: 99", 1);
    assert!(matches!(
        load_workflow_str(&invalid),
        Err(WorkflowLoadError::Validation(
            WorkflowValidationError::UnsupportedSchemaVersion { .. }
        ))
    ));
}

#[test]
fn creates_cycle_and_applies_declared_transition() {
    let mut engine = engine();
    let created = start_cycle(&mut engine, "event-create");
    assert_eq!(created.status, CycleStatus::Open);
    assert_eq!(created.phase, Phase::Explore);

    let evidence = explore_evidence(&mut engine, &created.cycle_id, true, true);
    let plan = engine
        .plan_transition(&created.cycle_id, "phase.explore.complete", evidence)
        .unwrap();
    assert_eq!(plan.outcome(), TransitionOutcome::Succeeded);
    assert_eq!(plan.state_after().phase, Phase::Specify);

    let applied = engine
        .apply_transition(&plan, &context("event-explore"), &auth())
        .unwrap();
    assert_eq!(applied.manifest.phase, Phase::Specify);
    assert_eq!(applied.event.sequence, 2);
    assert_eq!(
        engine
            .ledger()
            .get_cycle(&created.cycle_id)
            .unwrap()
            .manifest,
        applied.manifest
    );
}

#[test]
fn rejects_undeclared_transition_and_wrong_source_state() {
    let mut engine = engine();
    let cycle = start_cycle(&mut engine, "event-create");

    assert!(matches!(
        engine.plan_transition(
            &cycle.cycle_id,
            "phase.unknown",
            TransitionEvidence::default()
        ),
        Err(EngineError::UndeclaredTransition { .. })
    ));
    assert!(matches!(
        engine.plan_transition(
            &cycle.cycle_id,
            "phase.design.complete",
            TransitionEvidence::default()
        ),
        Err(EngineError::SourceStateMismatch {
            actual_phase: Phase::Explore,
            ..
        })
    ));
    assert!(matches!(
        engine.plan_transition(
            &cycle.cycle_id,
            "cycle.start",
            TransitionEvidence::default()
        ),
        Err(EngineError::CreationTransitionRequiresStartApi { .. })
    ));
}

#[test]
fn reduced_path_transitions_are_path_scoped() {
    let mut engine = engine();
    let input = CycleStartInput {
        manifest: manifest_for_path(CycleStatus::Blocked, Phase::Archive, CyclePath::BDirect),
        requirements: cycle_start_requirements(),
    };
    let start = engine.plan_cycle_start(input).unwrap();
    assert_eq!(start.state_after().phase, Phase::Build);
    let cycle = engine
        .apply_cycle_start(&start, &context("event-b-direct-create"))
        .unwrap()
        .manifest;

    assert!(matches!(
        engine.plan_transition(
            &cycle.cycle_id,
            "phase.build.complete",
            TransitionEvidence::default()
        ),
        Err(EngineError::TransitionPathMismatch { path, .. }) if path == "B-direct"
    ));

    let mut evidence = TransitionEvidence::default();
    evidence.artifacts.insert(
        "implementation-receipt".into(),
        ArtifactRef::new("implementation-receipt", "artifacts/implementation.json"),
    );
    evidence.gates.insert(
        "implementation-complete".into(),
        GateReceiptRef {
            receipt_id: pass_gate(
                &mut engine,
                &cycle.cycle_id,
                "phase.build.complete.b-direct",
                "implementation-complete",
            ),
        },
    );
    let verify = engine
        .plan_transition(&cycle.cycle_id, "phase.build.complete.b-direct", evidence)
        .unwrap();
    assert_eq!(verify.state_after().status, CycleStatus::Open);
    assert_eq!(verify.state_after().phase, Phase::Verify);
    let verified = engine
        .apply_transition(&verify, &context("event-b-direct-verify"), &auth())
        .unwrap()
        .manifest;

    let mut evidence = TransitionEvidence::default();
    evidence.artifacts.insert(
        "verification-report".into(),
        ArtifactRef::new("verification-report", "artifacts/verification.md"),
    );
    for gate in ["tests-pass", "policy-compliant"] {
        evidence.gates.insert(
            gate.into(),
            GateReceiptRef {
                receipt_id: pass_gate(
                    &mut engine,
                    &verified.cycle_id,
                    "phase.verify.complete.b-direct",
                    gate,
                ),
            },
        );
    }
    let release = engine
        .plan_transition(
            &verified.cycle_id,
            "phase.verify.complete.b-direct",
            evidence,
        )
        .unwrap();
    assert_eq!(release.state_after().status, CycleStatus::ReleasePending);
    assert_eq!(release.state_after().phase, Phase::Release);
}

#[test]
fn full_path_cannot_invoke_reduced_path_skip() {
    let manifest = manifest_at(CycleStatus::Open, Phase::Specify);
    let cycle_id = manifest.cycle_id.clone();
    let engine = engine_with_snapshot(manifest);

    assert!(matches!(
        engine.plan_transition(
            &cycle_id,
            "phase.specify.complete.a-min",
            TransitionEvidence::default()
        ),
        Err(EngineError::TransitionPathMismatch { path, .. }) if path == "A-full"
    ));
}

#[test]
fn rejects_missing_artifact_and_missing_gate() {
    let mut engine = engine();
    let cycle = start_cycle(&mut engine, "event-create");

    let artifact_evidence = explore_evidence(&mut engine, &cycle.cycle_id, false, true);
    assert!(matches!(
        engine.plan_transition(
            &cycle.cycle_id,
            "phase.explore.complete",
            artifact_evidence
        ),
        Err(EngineError::MissingArtifact { artifact, .. })
            if artifact == "exploration-report"
    ));
    let gate_evidence = explore_evidence(&mut engine, &cycle.cycle_id, true, false);
    assert!(matches!(
        engine.plan_transition(
            &cycle.cycle_id,
            "phase.explore.complete",
            gate_evidence
        ),
        Err(EngineError::MissingGateReceipt { gate, .. }) if gate == "exploration-sufficient"
    ));
}

#[test]
fn block_and_unblock_preserve_the_current_phase() {
    let mut engine = engine();
    let cycle = start_cycle(&mut engine, "event-create");
    let block_evidence = gate_evidence(
        &mut engine,
        &cycle.cycle_id,
        "cycle.block",
        "block-condition-met",
    );
    let block = engine
        .plan_transition(&cycle.cycle_id, "cycle.block", block_evidence)
        .unwrap();
    assert_eq!(block.state_after().status, CycleStatus::Blocked);
    assert_eq!(block.state_after().phase, Phase::Explore);
    engine
        .apply_transition(&block, &context("event-block"), &auth())
        .unwrap();

    let unblock_evidence = gate_evidence(
        &mut engine,
        &cycle.cycle_id,
        "cycle.unblock",
        "unblock-condition-met",
    );
    let unblock = engine
        .plan_transition(&cycle.cycle_id, "cycle.unblock", unblock_evidence)
        .unwrap();
    assert_eq!(unblock.state_after().status, CycleStatus::Open);
    assert_eq!(unblock.state_after().phase, Phase::Explore);
    engine
        .apply_transition(&unblock, &context("event-unblock"), &auth())
        .unwrap();
}

#[test]
fn failed_verification_uses_declared_remediation_target() {
    let manifest = manifest_at(CycleStatus::Open, Phase::Verify);
    let cycle_id = manifest.cycle_id.clone();
    let mut engine = engine_with_snapshot(manifest);
    let mut evidence = TransitionEvidence::default();
    evidence.artifacts.insert(
        "verification-report".into(),
        ArtifactRef::new("verification-report", "artifacts/verification.md"),
    );
    evidence.gates.insert(
        "tests-pass".into(),
        GateReceiptRef {
            receipt_id: engine
                .evaluate_gate(&GateEvaluationInput {
                    cycle_id: cycle_id.clone(),
                    transition_id: "phase.verify.complete".into(),
                    gate: "tests-pass".into(),
                    evaluator: sddk_engine::DEFAULT_EVALUATOR.into(),
                    evidence: serde_json::json!({"failed": 1}),
                    outcome: sddk_storage::GateOutcomeStatus::Failed,
                    evaluated_at: TIMESTAMP.into(),
                    actor: "test-runtime".into(),
                    command_id: "gate-tests-pass".into(),
                }, &sys_auth())
                .unwrap()
                .receipt_id,
        },
    );
    evidence.gates.insert(
        "policy-compliant".into(),
        GateReceiptRef {
            receipt_id: pass_gate(
                &mut engine,
                &cycle_id,
                "phase.verify.complete",
                "policy-compliant",
            ),
        },
    );
    evidence.gates.insert(
        "debt-severity-assigned".into(),
        GateReceiptRef {
            receipt_id: pass_gate(
                &mut engine,
                &cycle_id,
                "phase.verify.complete",
                "debt-severity-assigned",
            ),
        },
    );
    evidence.gates.insert(
        "debt-priority-assigned".into(),
        GateReceiptRef {
            receipt_id: pass_gate(
                &mut engine,
                &cycle_id,
                "phase.verify.complete",
                "debt-priority-assigned",
            ),
        },
    );

    let plan = engine
        .plan_transition(&cycle_id, "phase.verify.complete", evidence)
        .unwrap();
    assert_eq!(plan.outcome(), TransitionOutcome::Failed);
    assert_eq!(plan.failed_gates(), ["tests-pass"]);
    assert_eq!(plan.state_after().status, CycleStatus::Remediating);
    assert_eq!(plan.state_after().phase, Phase::Verify);
    let applied = engine
        .apply_transition(&plan, &context("event-remediation"), &auth())
        .unwrap();
    assert_eq!(applied.outcome, TransitionOutcome::Failed);
    assert_eq!(applied.manifest.status, CycleStatus::Remediating);
}

#[test]
fn duplicate_event_id_rolls_back_transition_snapshot() {
    let mut engine = engine();
    let cycle = start_cycle(&mut engine, "duplicate-event");
    let evidence = explore_evidence(&mut engine, &cycle.cycle_id, true, true);
    let plan = engine
        .plan_transition(&cycle.cycle_id, "phase.explore.complete", evidence)
        .unwrap();

    assert!(matches!(
        engine.apply_transition(&plan, &context("duplicate-event"), &auth()),
        Err(EngineError::Storage(StorageError::Database(_)))
    ));
    let stored = engine.ledger().get_cycle(&cycle.cycle_id).unwrap();
    assert_eq!(stored.manifest.phase, Phase::Explore);
    assert_eq!(
        engine
            .ledger()
            .list_cycle_events(&cycle.cycle_id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn replay_matches_the_latest_stored_snapshot() {
    let mut engine = engine();
    let cycle = start_cycle(&mut engine, "event-create");
    let evidence = explore_evidence(&mut engine, &cycle.cycle_id, true, true);
    let plan = engine
        .plan_transition(&cycle.cycle_id, "phase.explore.complete", evidence)
        .unwrap();
    engine
        .apply_transition(&plan, &context("event-explore"), &auth())
        .unwrap();

    let replayed = engine.verify_cycle_snapshot(&cycle.cycle_id).unwrap();
    assert_eq!(replayed.sequence, 2);
    assert_eq!(replayed.manifest.phase, Phase::Specify);
}

#[test]
fn replay_detects_missing_corrupt_and_non_object_state() {
    let cases = [
        (None, "missing"),
        (Some(json!({})), "corrupt"),
        (Some(json!(42)), "non-object"),
    ];

    for (state_after, expected) in cases {
        let manifest = manifest_at(CycleStatus::Open, Phase::Explore);
        let cycle_id = manifest.cycle_id.clone();
        let engine = engine_with_state_event(manifest, state_after);
        let error = engine.replay_cycle(&cycle_id).unwrap_err();
        assert!(
            matches!(
                (&error, expected),
                (EngineError::MissingStateAfter { .. }, "missing")
                    | (EngineError::CorruptStateAfter { .. }, "corrupt")
                    | (EngineError::NonObjectStateAfter { .. }, "non-object")
            ),
            "unexpected replay error: {error}"
        );
    }
}

#[test]
fn replay_detects_snapshot_divergence() {
    let manifest = manifest_at(CycleStatus::Open, Phase::Explore);
    let cycle_id = manifest.cycle_id.clone();
    let mut replayed = manifest.clone();
    replayed.phase = Phase::Specify;
    let engine = engine_with_state_event(manifest, Some(serde_json::to_value(replayed).unwrap()));

    assert!(matches!(
        engine.verify_cycle_snapshot(&cycle_id),
        Err(EngineError::SnapshotMismatch { .. })
    ));
}

#[test]
fn exposes_declared_debt_policy_and_rejects_unknown_paths() {
    let engine = engine();
    for path in ["A-min", "A-lite", "A-full"] {
        assert_eq!(
            engine.debt_verification_policy(path).unwrap(),
            DebtVerificationPolicy::Mandatory
        );
    }
    assert_eq!(
        engine.debt_verification_policy("B-direct").unwrap(),
        DebtVerificationPolicy::Disabled
    );
    assert!(matches!(
        engine.debt_verification_policy("C-unknown"),
        Err(EngineError::UnknownPath { .. })
    ));
}

fn engine() -> Engine<Storage> {
    let storage = storage_with_parents();
    Engine::new(load_workflow_str(WORKFLOW_YAML).unwrap(), storage).unwrap()
}

fn engine_with_snapshot(manifest: CycleManifest) -> Engine<Storage> {
    let storage = storage_with_parents();
    storage
        .insert_cycle(&CycleRecord {
            manifest,
            created_at: TIMESTAMP.into(),
            updated_at: TIMESTAMP.into(),
        })
        .unwrap();
    Engine::new(load_workflow_str(WORKFLOW_YAML).unwrap(), storage).unwrap()
}

fn engine_with_state_event(
    manifest: CycleManifest,
    state_after: Option<serde_json::Value>,
) -> Engine<Storage> {
    let cycle_id = manifest.cycle_id.clone();
    let mut storage = storage_with_parents();
    storage
        .insert_cycle(&CycleRecord {
            manifest,
            created_at: TIMESTAMP.into(),
            updated_at: TIMESTAMP.into(),
        })
        .unwrap();
    storage
        .append_event(&raw_state_event(&cycle_id, state_after))
        .unwrap();
    Engine::new(load_workflow_str(WORKFLOW_YAML).unwrap(), storage).unwrap()
}

fn storage_with_parents() -> Storage {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-1".into(),
            display_name: "Project One".into(),
            remote_url: Some("https://example.com/owner/project".into()),
            scope: "owner".into(),
            created_at: TIMESTAMP.into(),
        })
        .unwrap();
    storage
        .insert_workspace(&WorkspaceRecord {
            workspace_id: "workspace-1".into(),
            project_id: "project-1".into(),
            canonical_path: "/work/project".into(),
            created_at: TIMESTAMP.into(),
        })
        .unwrap();
    storage
}

fn start_cycle(engine: &mut Engine<Storage>, event_id: &str) -> CycleManifest {
    let input = CycleStartInput {
        manifest: manifest_at(CycleStatus::Blocked, Phase::Archive),
        requirements: cycle_start_requirements(),
    };
    let plan = engine.plan_cycle_start(input).unwrap();
    assert_eq!(plan.state_after().status, CycleStatus::Open);
    assert_eq!(plan.state_after().phase, Phase::Explore);
    engine
        .apply_cycle_start(&plan, &context(event_id))
        .unwrap()
        .manifest
}

fn manifest_at(status: CycleStatus, phase: Phase) -> CycleManifest {
    manifest_for_path(status, phase, CyclePath::AFull)
}

fn manifest_for_path(status: CycleStatus, phase: Phase, path: CyclePath) -> CycleManifest {
    CycleManifest {
        schema_version: 1,
        project_id: "project-1".into(),
        workspace_id: "workspace-1".into(),
        cycle_id: "cycle-1".into(),
        display_name: "Engine work".into(),
        status,
        phase,
        path,
        branch: "feat/engine".into(),
        base: "abc123".into(),
        head: None,
        artifacts: HashMap::new(),
        release: None,
        delivery_kind: None,
        remediation_round: 0,
        remote_url: Some("https://example.com/owner/project".into()),
        scope: Some("owner".into()),
        pause_at: None,
        review_at: None,
        last_pause_reason: None,
    }
}

fn cycle_start_requirements() -> BTreeSet<String> {
    [
        "project.adopted",
        "project.initialized",
        "worktree.clean",
        "cycle.no_active_conflict",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn context(event_id: &str) -> EventContext {
    EventContext {
        command_id: format!("command-{event_id}"),
        frame_id: format!("frame-{event_id}"),
        event_id: event_id.into(),
        actor: "test-runtime".into(),
        actor_ref: None,
        occurred_at: TIMESTAMP.into(),
        correlation_id: None,
        causation_id: None,
    }
}

fn auth() -> AuthorityContext {
    AuthorityContext::for_test(sddk_domain::ActorKind::Agent, "test-runtime")
}

fn sys_auth() -> AuthorityContext {
    AuthorityContext::for_test(sddk_domain::ActorKind::System, "test-system")
}

fn pass_gate(
    engine: &mut Engine<Storage>,
    cycle_id: &str,
    transition_id: &str,
    gate: &str,
) -> String {
    engine
        .evaluate_gate(&GateEvaluationInput {
            cycle_id: cycle_id.into(),
            transition_id: transition_id.into(),
            gate: gate.into(),
            evaluator: sddk_engine::DEFAULT_EVALUATOR.into(),
            evidence: serde_json::json!({
                "argv": ["cargo", "test", "--workspace"],
                "exit_code": 0,
                "output_digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            }),
            outcome: sddk_storage::GateOutcomeStatus::Passed,
            evaluated_at: TIMESTAMP.into(),
            actor: "test-runtime".into(),
            command_id: format!("gate-{gate}"),
        }, &sys_auth())
        .unwrap()
        .receipt_id
}

fn gate_evidence(
    engine: &mut Engine<Storage>,
    cycle_id: &str,
    transition_id: &str,
    name: &str,
) -> TransitionEvidence {
    TransitionEvidence {
        requirements: BTreeSet::new(),
        artifacts: BTreeMap::new(),
        gates: [(
            name.to_owned(),
            GateReceiptRef {
                receipt_id: pass_gate(engine, cycle_id, transition_id, name),
            },
        )]
        .into_iter()
        .collect(),
    }
}

fn explore_evidence(
    engine: &mut Engine<Storage>,
    cycle_id: &str,
    with_artifact: bool,
    with_gate: bool,
) -> TransitionEvidence {
    let mut evidence = TransitionEvidence::default();
    if with_artifact {
        evidence.artifacts.insert(
            "exploration-report".into(),
            ArtifactRef::new("exploration-report", "artifacts/exploration.md"),
        );
    }
    if with_gate {
        evidence.gates.insert(
            "exploration-sufficient".into(),
            GateReceiptRef {
                receipt_id: pass_gate(
                    engine,
                    cycle_id,
                    "phase.explore.complete",
                    "exploration-sufficient",
                ),
            },
        );
    }
    evidence
}

fn raw_state_event(cycle_id: &str, state_after: Option<serde_json::Value>) -> LedgerEventInput {
    LedgerEventInput {
        event_id: format!("event-{cycle_id}"),
        project_id: "project-1".into(),
        cycle_id: Some(cycle_id.into()),
        frame_id: "frame-replay".into(),
        command_id: "command-replay".into(),
        actor: "test-runtime".into(),
        actor_ref: None,
        event_type: "cycle.transitioned".into(),
        occurred_at: TIMESTAMP.into(),
        state_before: None,
        state_after,
        payload: json!({"transition_id": "test.corrupt"}),
        causation_id: None,
        correlation_id: None,
    }
}
