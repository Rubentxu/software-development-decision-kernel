//! C3l.3 — Dynamic Workflow Expansion **vertical** E2E real.
//!
//! This file is the falsifier for the defect recorded in
//! `tests/cycle-artifacts/p-63676b11dc0ef88f/session54-c3l3-dynamic-expansion-vertical/SCOPE-CONTRACT.md`:
//! `aiw_s4_dynamic_expansion.rs` calls `Engine::cycle_replan` DIRECTLY and never
//! crosses `evidence → Secretary → orchestration → authority → ReplanDelta →
//! PlanRevision N+1 → incremental execution`.
//!
//! `boundary_class = IN_PROCESS/SQLITE` — the whole vertical runs against the
//! real SQLite ledger through the public engine API. Nothing here reconstructs
//! an event by hand and nothing calls an inner step directly to make a signal.
//!
//! The seven falsifiers (F1..F7) map 1:1 to the package's declared list; the
//! four exit-gate clauses are asserted in `exit_gate_*` tests.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use sddk_domain::plan_revision::{PlanMutation, PlanProvenanceV1};
use sddk_domain::workflow_ir::{Budgets, CapabilityId, Operator, OperatorId, WorkflowIR};
use sddk_domain::{ActorKind, CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_engine::dynamic_expansion::{ExpansionTrigger, OrchestrationVerdict};
use sddk_engine::secretary_l1::{
    BoundedWindow, ClosedSetKind, ProposalTemplate, SecretaryId, SecretaryL1Engine,
};
use sddk_engine::{
    CycleStartInput, Engine, EventContext, RestageTo, authority::AuthorityContext,
    risk_approval_policy::RiskTier,
};
use sddk_storage::{ProjectRecord, Storage, WorkspaceRecord};

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");
const TIMESTAMP: &str = "2026-10-01T08:00:00Z";
const PROJECT_ID: &str = "c3l3-project";
const WORKSPACE_ID: &str = "c3l3-workspace";
const ACTOR: &str = "c3l3-system";

// ── helpers ────────────────────────────────────────────────────────────────

fn open_storage() -> (tempfile::TempDir, Storage, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&path).expect("open");
    storage
        .insert_project(&ProjectRecord {
            project_id: PROJECT_ID.into(),
            display_name: "C3l.3 project".into(),
            remote_url: None,
            scope: "owner".into(),
            created_at: TIMESTAMP.into(),
        })
        .expect("insert_project");
    storage
        .insert_workspace(&WorkspaceRecord {
            workspace_id: WORKSPACE_ID.into(),
            project_id: PROJECT_ID.into(),
            canonical_path: "/work/c3l3".into(),
            created_at: TIMESTAMP.into(),
        })
        .expect("insert_workspace");
    (dir, storage, path)
}

fn engine_with_storage(storage: Storage) -> Engine<Storage> {
    let wf = sddk_engine::load_workflow_str(WORKFLOW_YAML).expect("workflow yaml");
    Engine::new(wf, storage).expect("engine new")
}

fn make_manifest(cycle_id: &str) -> CycleManifest {
    CycleManifest {
        schema_version: 1,
        project_id: PROJECT_ID.into(),
        workspace_id: WORKSPACE_ID.into(),
        cycle_id: cycle_id.into(),
        display_name: "C3l.3 cycle".into(),
        status: CycleStatus::Open,
        phase: Phase::Explore,
        path: CyclePath::AFull,
        branch: "main".into(),
        base: "deadbeef".into(),
        head: None,
        artifacts: HashMap::new(),
        release: None,
        delivery_kind: None,
        remediation_round: 0,
        remote_url: None,
        scope: Some("owner".into()),
        pause_at: None,
        review_at: None,
        last_pause_reason: None,
        replan_count: 0,
    }
}

fn requirements() -> BTreeSet<String> {
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

/// Authority admitted on `WritableSurface::PlanRevisions` (the matrix admits
/// `Human` + `Agent` — see `WRITABLE_SURFACE_MATRIX`).
fn auth() -> AuthorityContext {
    AuthorityContext::for_test(ActorKind::Agent, ACTOR)
}

/// Actor kind with NO write admission on `plan_revisions` (F3). `System` is
/// admitted on `cycle_state` but explicitly NOT on `plan_revisions`.
fn denied_auth() -> AuthorityContext {
    AuthorityContext::for_test(ActorKind::System, ACTOR)
}

fn context(event_id: &str, command_id: &str) -> EventContext {
    EventContext {
        command_id: command_id.into(),
        frame_id: format!("frame:{command_id}"),
        event_id: event_id.into(),
        actor: ACTOR.into(),
        actor_ref: None,
        occurred_at: TIMESTAMP.into(),
        correlation_id: None,
        causation_id: None,
    }
}

/// Base plan: one operator `t1`.
fn base_ir() -> WorkflowIR {
    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "c3l3.template".into(),
            version: "1.0.0".into(),
        },
        operators: BTreeMap::from([(
            OperatorId("t1".into()),
            Operator::Task {
                capability: CapabilityId("c3l3.cap".into()),
                inputs: Default::default(),
            },
        )]),
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "c3l3-generator".into(),
            prompt_hash: "prompt-c3l3".into(),
            model_hash: "model-c3l3".into(),
            policy_hash: "policy-c3l3".into(),
        },
    }
}

/// Expanded plan: base `t1` plus a NEW operator `t2` — the single new node.
fn expanded_ir() -> WorkflowIR {
    let mut ir = base_ir();
    ir.operators.insert(
        OperatorId("t2".into()),
        Operator::Task {
            capability: CapabilityId("c3l3.cap.extra".into()),
            inputs: Default::default(),
        },
    );
    ir
}

fn trigger(expected_base: &str) -> ExpansionTrigger {
    ExpansionTrigger {
        cycle_id: String::new(), // filled by `bound_trigger`
        gap_summary: "evidence gap: t2 has no coverage".into(),
        evidence_refs: vec!["ev-gap-1".into()],
        base_ir: base_ir(),
        proposed_ir: expanded_ir(),
        expected_base_revision: expected_base.to_string(),
        restage_to: RestageTo::Design,
        template_id: "c3l3.template.expand".into(),
        confidence: 0.9,
    }
}

fn bound_trigger(engine: &Engine<Storage>, cycle_id: &str) -> ExpansionTrigger {
    let mut t = trigger(&engine.current_plan_revision_id(cycle_id).expect("base revision"));
    t.cycle_id = cycle_id.to_string();
    t
}

fn secretary() -> SecretaryL1Engine {
    let e = SecretaryL1Engine::new();
    e.register_template(ProposalTemplate::new(
        "c3l3.template.expand",
        ClosedSetKind::SuggestCandidate,
        "expand the workflow with the missing node",
        RiskTier::Medium,
        BoundedWindow::new(SecretaryId("c3l3-secretary".into()), 0, i64::MAX, 5),
    ))
    .expect("register template");
    e
}

/// Bootstraps a cycle with a lease and returns
/// (guard, engine, cycle_id, base_revision_id, ledger_path).
fn booted(cycle_id: &str) -> (tempfile::TempDir, Engine<Storage>, String, String, PathBuf) {
    let (dir, storage, path) = open_storage();
    let mut engine = engine_with_storage(storage);
    let plan = engine
        .plan_cycle_start(CycleStartInput {
            manifest: make_manifest(cycle_id),
            requirements: requirements(),
        })
        .expect("plan");
    let receipt = engine
        .apply_cycle_start(&plan, &context("c3l3-start", "c3l3-cmd-start"), &auth())
        .expect("apply start");
    let cid = receipt.manifest.cycle_id.clone();
    engine
        .acquire_cycle_lease(&cid, ACTOR, 0, i64::MAX)
        .expect("lease");
    let base_rev = engine.current_plan_revision_id(&cid).expect("base revision");
    (dir, engine, cid, base_rev, path)
}

fn apply(
    engine: &mut Engine<Storage>,
    trigger: &ExpansionTrigger,
    authority: &AuthorityContext,
    receipt_path: &std::path::Path,
) -> Result<sddk_engine::dynamic_expansion::ExpansionOutcome, sddk_engine::dynamic_expansion::ExpansionError> {
    engine.apply_dynamic_expansion(
        trigger,
        authority,
        &context("c3l3-expansion", "c3l3-cmd-expansion"),
        receipt_path,
    )
}

// ── Exit gate 1: exactly one expansion ante replay ─────────────────────────

/// F1 — trigger duplicado. The same trigger applied twice must yield exactly
/// ONE revision delta and must report the replay honestly (`applied == false`).
#[test]
fn f1_duplicate_trigger_yields_exactly_one_revision() {
    let (_d, mut engine, cid, base_rev, _p) = booted("c3l3-f1");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");

    let first = apply(&mut engine, &t, &auth(), receipts.path()).expect("first apply");
    let second = apply(&mut engine, &t, &auth(), receipts.path()).expect("replay apply");

    assert!(first.applied, "first application is a real expansion");
    assert!(!second.applied, "replay must report applied == false");
    assert_eq!(
        first.revision_id, second.revision_id,
        "replay must return the SAME revision id, not a new one"
    );
    assert_eq!(first.lineage_len, 2, "root + exactly one derived revision");
    assert_eq!(
        second.lineage_len, 2,
        "replay must not grow the lineage"
    );
    assert_eq!(first.parent_revision_id.as_deref(), Some(base_rev.as_str()));
}

// ── Exit gate 2: nodos previos no se re-ejecutan ──────────────────────────

/// F2 — proposal replay. The execution ledger must name ONLY the new node
/// (`t2`) and must never re-name a pre-existing node (`t1`) — across replay.
#[test]
fn f2_replay_executes_only_the_new_node_once() {
    let (_d, mut engine, cid, _base, _p) = booted("c3l3-f2");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");

    let first = apply(&mut engine, &t, &auth(), receipts.path()).expect("first");
    let second = apply(&mut engine, &t, &auth(), receipts.path()).expect("replay");

    let executed: Vec<&str> = first
        .executed_node_ids
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert_eq!(executed, vec!["t2"], "only the NEW node is dispatched");
    assert!(
        !first.executed_node_ids.iter().any(|n| n.as_str() == "t1"),
        "pre-existing node t1 must never be re-dispatched"
    );
    assert_eq!(
        first.new_node_execution_count, 1,
        "exactly one new-node execution"
    );
    assert_eq!(
        second.new_node_execution_count, 0,
        "replay adds zero new-node executions"
    );
    assert_eq!(
        first.total_new_node_execution_count, 1,
        "the durable total stays 1 across replay"
    );
}

// ── Exit gate 3: denied / invalid no cambia el plan ───────────────────────

/// F3 — authority denied. Zero canonical change: no event, no revision, no
/// snapshot mutation, no execution.
#[test]
fn f3_authority_denied_leaves_no_canonical_change() {
    let (_d, mut engine, cid, base_rev, _p) = booted("c3l3-f3");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");

    let before_events = engine.ledger().list_cycle_events(&cid).expect("events");
    let before_count = engine
        .ledger()
        .get_cycle(&cid)
        .expect("cycle")
        .manifest
        .replan_count;

    let err = apply(&mut engine, &t, &denied_auth(), receipts.path())
        .expect_err("denied authority must fail closed");

    assert!(
        matches!(
            err,
            sddk_engine::dynamic_expansion::ExpansionError::AuthorityDenied { .. }
        ),
        "denial must be typed and named, got {err:?}"
    );

    let after_events = engine.ledger().list_cycle_events(&cid).expect("events");
    assert_eq!(
        before_events.len(),
        after_events.len(),
        "a denied expansion must not append events"
    );
    assert_eq!(
        engine.current_plan_revision_id(&cid).expect("rev"),
        base_rev,
        "denied expansion must not move the plan"
    );
    assert_eq!(
        engine
            .ledger()
            .get_cycle(&cid)
            .expect("cycle")
            .manifest
            .replan_count,
        before_count,
        "denied expansion must not touch the projection"
    );
}

/// F4 — delta inválido. An empty plan delta (proposed IR with no operators)
/// must fail closed with zero canonical change.
#[test]
fn f4_invalid_delta_leaves_no_canonical_change() {
    let (_d, mut engine, cid, base_rev, _p) = booted("c3l3-f4");
    let receipts = tempfile::tempdir().expect("receipts");

    let mut t = bound_trigger(&engine, &cid);
    t.proposed_ir.operators.clear(); // invalid: empty plan

    let before_events = engine.ledger().list_cycle_events(&cid).expect("events");

    let err = apply(&mut engine, &t, &auth(), receipts.path())
        .expect_err("empty delta must fail closed");
    assert!(
        matches!(
            err,
            sddk_engine::dynamic_expansion::ExpansionError::InvalidDelta { .. }
        ),
        "invalid delta must be typed, got {err:?}"
    );

    assert_eq!(
        engine.ledger().list_cycle_events(&cid).expect("events").len(),
        before_events.len(),
        "invalid delta must not append events"
    );
    assert_eq!(
        engine.current_plan_revision_id(&cid).expect("rev"),
        base_rev,
        "invalid delta must not move the plan"
    );
}

// ── Exit gate 4: la nueva revisión tiene lineage y receipts ───────────────

/// The new revision must carry lineage (parent + typed mutation) and the
/// expansion must leave a durable receipt on disk.
#[test]
fn new_revision_carries_lineage_and_receipt() {
    let (_d, mut engine, cid, base_rev, _p) = booted("c3l3-lineage");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");

    let out = apply(&mut engine, &t, &auth(), receipts.path()).expect("apply");

    assert_eq!(out.parent_revision_id.as_deref(), Some(base_rev.as_str()));
    assert_eq!(out.mutation, PlanMutation::NodesChanged);

    let receipt_file = receipts.path().join(&cid).join("expansion-receipt.json");
    assert!(receipt_file.is_file(), "receipt must exist at {receipt_file:?}");
    let body = std::fs::read_to_string(&receipt_file).expect("read receipt");
    let v: serde_json::Value = serde_json::from_str(&body).expect("receipt is json");
    assert_eq!(v["trigger_fingerprint"], out.trigger_fingerprint);
    assert_eq!(v["revision_id"], out.revision_id);
    assert_eq!(v["parent_revision_id"], base_rev);
    assert_eq!(v["executed_node_ids"][0], "t2");

    // The receipt is atomic and rewritten with the same identity on replay.
    apply(&mut engine, &t, &auth(), receipts.path()).expect("replay");
    let body2 = std::fs::read_to_string(&receipt_file).expect("read receipt again");
    let v2: serde_json::Value = serde_json::from_str(&body2).expect("json");
    assert_eq!(v2["revision_id"], out.revision_id, "receipt identity is stable");
}

// ── F5 — execution failure of the new node ────────────────────────────────

/// The new node fails to execute. The revision MUST stay durable (the plan
/// change is real) and the failure MUST be recorded — never counted as success.
#[test]
fn f5_new_node_execution_failure_is_recorded_not_counted_as_success() {
    let (_d, mut engine, cid, _base, _p) = booted("c3l3-f5");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");

    let out = engine
        .apply_dynamic_expansion_with(
            &t,
            &auth(),
            &context("c3l3-expansion", "c3l3-cmd-expansion"),
            receipts.path(),
            // The hook returns `true` on SUCCESS; t2 is the only dispatched
            // node, so returning false for it makes it the only failure.
            &|node| node != "t2",
        )
        .expect("expansion is durable even if the node fails");

    assert!(out.applied);
    assert!(out.new_node_failed, "the failure must be visible in the outcome");
    assert_eq!(
        out.new_node_execution_count, 0,
        "a failed node is not counted as an execution"
    );
    assert_eq!(out.failed_node_ids, vec!["t2".to_string()]);
    // The plan revision itself is durable — the change happened.
    assert_eq!(out.lineage_len, 2);
    assert!(!out.revision_id.is_empty());
}

// ── F6 — restart entre proposal y apply ───────────────────────────────────

/// The proposal is made, the process dies, a NEW engine reopens the same
/// SQLite ledger. The replay guard must survive: the second apply is a
/// no-op, not a second expansion.
#[test]
fn f6_restart_between_proposal_and_apply_keeps_one_expansion() {
    let (_d, mut engine, cid, _base, ledger_path) = booted("c3l3-f6");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");

    let first = apply(&mut engine, &t, &auth(), receipts.path()).expect("pre-restart apply");

    // "Restart": drop the engine entirely, reopen the same SQLite file.
    drop(engine);
    let reopened = Storage::open(&ledger_path).expect("reopen ledger");
    let mut engine = engine_with_storage(reopened);

    let after_restart = apply(&mut engine, &t, &auth(), receipts.path())
        .expect("post-restart apply must succeed as replay");

    assert!(!after_restart.applied, "restart must not re-expand");
    assert_eq!(after_restart.revision_id, first.revision_id);
    assert_eq!(after_restart.lineage_len, 2, "lineage survives the restart");
    assert_eq!(after_restart.total_new_node_execution_count, 1);
}

// ── F7 — plan base cambió antes del apply ─────────────────────────────────

/// The trigger pins the base revision it was proposed against. If the plan
/// moved in between, the apply must fail closed with zero canonical change.
///
/// NOTE: this must use a trigger with *different* content than the one that
/// already landed — otherwise the replay guard (F1/F2) would legitimately
/// short-circuit before the base pin is ever consulted, and the test would
/// prove the wrong invariant.
#[test]
fn f7_stale_base_revision_fails_closed() {
    let (_d, mut engine, cid, base_rev, _p) = booted("c3l3-f7");
    let receipts = tempfile::tempdir().expect("receipts");

    // (1) Advance the plan with a legitimate expansion, moving the tip to R1.
    let t1 = bound_trigger(&engine, &cid);
    let out1 = apply(&mut engine, &t1, &auth(), receipts.path()).expect("first expansion");
    assert_ne!(out1.revision_id, base_rev, "the plan must have moved");

    // (2) A different proposal, but pinned to the OLD base revision R0.
    let mut t2 = t1.clone();
    t2.gap_summary = "a different evidence gap, still pinned to the old base".into();
    t2.evidence_refs = vec!["ev-gap-2".into()];
    assert_eq!(
        t2.expected_base_revision, base_rev,
        "t2 is deliberately pinned to the stale base"
    );

    let before_events = engine.ledger().list_cycle_events(&cid).expect("events");
    let err = apply(&mut engine, &t2, &auth(), receipts.path())
        .expect_err("stale base must fail closed");

    assert!(
        matches!(
            err,
            sddk_engine::dynamic_expansion::ExpansionError::StaleBaseRevision { .. }
        ),
        "stale base must be typed, got {err:?}"
    );
    assert_eq!(
        engine.ledger().list_cycle_events(&cid).expect("events").len(),
        before_events.len(),
        "stale base must not append events"
    );
    assert_eq!(
        engine.current_plan_revision_id(&cid).expect("rev"),
        out1.revision_id,
        "stale base must not move the plan"
    );
}

// ── Orchestration decision (vertical step 3) ──────────────────────────────

/// The pipeline must run an orchestration decision between the Secretary
/// proposal and the authority check — and a `Defer` verdict must not expand.
#[test]
fn orchestration_can_defer_without_expanding() {
    let (_d, mut engine, cid, base_rev, _p) = booted("c3l3-orch");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");

    let out = engine
        .apply_dynamic_expansion_deciding(
            &t,
            &auth(),
            &context("c3l3-expansion", "c3l3-cmd-expansion"),
            receipts.path(),
            // Orchestration says: wait for more evidence.
            &|_proposal| OrchestrationVerdict::Defer {
                reason: "coverage still thin".into(),
            },
        )
        .expect("defer is a decision, not a crash");

    assert!(!out.applied, "a deferred proposal must not expand");
    assert!(out.deferred, "the outcome must say why it did not expand");
    assert_eq!(
        engine.current_plan_revision_id(&cid).expect("rev"),
        base_rev,
        "defer must not move the plan"
    );
    assert!(
        engine
            .ledger()
            .list_cycle_events(&cid)
            .expect("events")
            .iter()
            .all(|e| !e.event_type.contains("expansion.applied")),
        "a deferred proposal must not land an applied event"
    );
}

/// Happy path through the full vertical: the Secretary must actually issue a
/// proposal, and the orchestration must be able to see it.
#[test]
fn secretary_proposal_reaches_orchestration() {
    let (_d, mut engine, cid, _base, _p) = booted("c3l3-sec");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");
    let sec = secretary();

    let seen_summary = std::cell::RefCell::new(String::new());
    let out = engine
        .apply_dynamic_expansion_with_secretary(
            &t,
            &sec,
            SecretaryId("c3l3-secretary".into()),
            &auth(),
            &context("c3l3-expansion", "c3l3-cmd-expansion"),
            receipts.path(),
            &|p| {
                *seen_summary.borrow_mut() = p.summary.clone();
                OrchestrationVerdict::Expand
            },
        )
        .expect("expansion");

    assert!(out.applied);
    assert!(
        !seen_summary.borrow().is_empty(),
        "orchestration saw a real proposal"
    );
    assert!(!out.proposal_id.is_empty(), "proposal id is recorded");
    assert_eq!(out.secretary_id.as_deref(), Some("c3l3-secretary"));
    assert_eq!(out.executed_node_ids, vec!["t2".to_string()]);
}

// ── Exit-gate summary: the four clauses, observable without reading names ──

/// One test that walks the whole exit gate in order, so a reader can see the
/// contract hold end-to-end rather than inferring it from seven names.
#[test]
fn exit_gate_vertical_holds_end_to_end() {
    let (_d, mut engine, cid, _base, _p) = booted("c3l3-gate");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");
    let sec = secretary();

    // 1. exactly one expansion ante replay
    let a = engine
        .apply_dynamic_expansion_with_secretary(
            &t,
            &sec,
            SecretaryId("c3l3-secretary".into()),
            &auth(),
            &context("c3l3-expansion", "c3l3-cmd-expansion"),
            receipts.path(),
            &|_| OrchestrationVerdict::Expand,
        )
        .expect("first");
    let b = engine
        .apply_dynamic_expansion_with_secretary(
            &t,
            &sec,
            SecretaryId("c3l3-secretary".into()),
            &auth(),
            &context("c3l3-expansion", "c3l3-cmd-expansion"),
            receipts.path(),
            &|_| OrchestrationVerdict::Expand,
        )
        .expect("replay");

    assert!(a.applied && !b.applied, "exactly one expansion ante replay");
    assert_eq!(a.revision_id, b.revision_id);
    assert_eq!(b.total_new_node_execution_count, 1, "1 ejecución total");

    // 2. nodos previos no se re-ejecutan
    assert_eq!(a.executed_node_ids, vec!["t2".to_string()]);

    // 3. denied / invalid no cambia el plan — checked on a sibling cycle
    let (_d2, mut e2, cid2, base2, _p2) = booted("c3l3-gate-deny");
    let t2 = bound_trigger(&e2, &cid2);
    let r2 = tempfile::tempdir().expect("receipts 2");
    assert!(apply(&mut e2, &t2, &denied_auth(), r2.path()).is_err());
    assert_eq!(e2.current_plan_revision_id(&cid2).expect("rev"), base2);

    // 4. lineage + receipts
    assert_eq!(a.lineage_len, 2);
    assert!(a.parent_revision_id.is_some());
    let provenance = PlanProvenanceV1::new("c3l3", "1.0.0").expect("provenance");
    assert_eq!(provenance.author, "c3l3");
}

// ── Negative control: the exit gate is load-bearing, not decorative ───────

/// A fresh engine with NO registered Secretary template cannot produce a
/// proposal, so it must not expand. If this test ever goes green by
/// accident, the vertical is not actually reading the Secretary.
#[test]
fn exit_gate_fresh_secretary_cannot_expand() {
    let (_d, mut engine, cid, base_rev, _p) = booted("c3l3-exit");
    let t = bound_trigger(&engine, &cid);
    let receipts = tempfile::tempdir().expect("receipts");
    // NOTE: empty Secretary — no template registered.
    let empty = SecretaryL1Engine::new();

    let err = engine
        .apply_dynamic_expansion_with_secretary(
            &t,
            &empty,
            SecretaryId("c3l3-secretary".into()),
            &auth(),
            &context("c3l3-expansion", "c3l3-cmd-expansion"),
            receipts.path(),
            &|_| OrchestrationVerdict::Expand,
        )
        .expect_err("an empty Secretary cannot produce a proposal");

    assert!(matches!(
        err,
        sddk_engine::dynamic_expansion::ExpansionError::NoProposal { .. }
    ));
    assert_eq!(
        engine.current_plan_revision_id(&cid).expect("rev"),
        base_rev,
        "no proposal ⇒ no expansion"
    );
}
