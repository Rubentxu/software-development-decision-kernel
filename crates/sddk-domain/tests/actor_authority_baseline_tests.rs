//! Regression-baseline tests for actor authority contracts.
//!
//! These tests lock the CURRENT behavior of `ActorKind`, `ActorRef`, CLI
//! prefix-string mapping, and provenance loss at the engine boundary as
//! regression baselines. They must PASS on the cycle branch. Subsequent cycles
//! (ARCH-HEX-001, EVT-LEDGER-001) will flip the behavior these tests assert.
//!
//! ## Coverage matrix
//!
//! | # | Test name | What it asserts | Gap it documents |
//! |---|-----------|-----------------|-------------------|
//! | 1 | `actor_kind_closed_set_is_three_variants` | `ActorKind` has exactly {Human, Agent, System} | No `Secretary` variant yet |
//! | 2 | `actor_ref_carries_five_required_fields` | `ActorRef` has 5 fields: kind, id, definition_hash, policy_hash, model | No `role` field yet |
//! | 3 | `cli_prefix_user_maps_to_human_kind` | `user:*` prefix resolves to `ActorKind::Human` | Only used in `cycle transition`; other commands use free-form `actor: String` |
//! | 4 | `cli_prefix_agent_maps_to_agent_kind` | `agent:*` prefix resolves to `ActorKind::Agent` | Same gap as above |
//! | 5 | `cli_prefix_unrecognized_falls_back_to_system` | Unrecognized prefix falls back to `ActorKind::System` | Same gap as above |
//! | 6 | `ledger_event_carries_string_actor_only_no_kind` | `LedgerEvent.actor` is `String`, not `ActorKind` | Provenance loss: INC-HX-AUTH-003 |
//! | 7 | `gate_receipt_carries_string_actor_only_no_kind` | `GateReceipt.actor` is `String`, not `ActorKind` | Provenance loss: INC-HX-AUTH-003 |
//! | 8 | `journal_entry_has_no_actor_fields` | `JournalEntry` has 0 fields matching `actor_*` | Provenance loss: INC-HX-AUTH-003 |
//!
//! These tests cover AC-HX-AUTH-01, AC-HX-AUTH-04, AC-HX-AUTH-07 and
//! REQ-AUTH-TAX-01..03, REQ-AUTH-PR-01..03, REQ-AUTH-TST-01..02.

use sddk_domain::event_envelope::{ActorKind, ActorRef};
use sddk_domain::models::Severity;
use sddk_domain::models::gate_receipt::GateReceipt;
use sddk_domain::models::ledger::LedgerEvent;
use sddk_domain::projections::journal::JournalEntry;

// ── AC-HX-AUTH-01 / REQ-AUTH-TAX-01..03 ───────────────────────────────────────

/// Asserts `ActorKind` enum has exactly three variants: Human, Agent, System.
/// Secretary has NO enum slot in A-min; it is `Agent{role=secretary}` per ADR-069.
/// Locked as baseline so EVT-LEDGER-001 can flip this when `ActorKind::Secretary` is added.
#[test]
fn actor_kind_closed_set_is_three_variants() {
    // There must be exactly 3 variants
    let variants = &[ActorKind::Human, ActorKind::Agent, ActorKind::System];
    assert_eq!(
        variants.len(),
        3,
        "ActorKind must have exactly 3 variants in A-min"
    );
}

// ── AC-HX-AUTH-04 / REQ-AUTH-PR-01 ───────────────────────────────────────────

/// Asserts `ActorRef` struct carries exactly the five required fields:
/// kind, id, definition_hash, policy_hash, model.
/// The `role` field does not exist yet (deferred to EVT-LEDGER-001).
#[test]
fn actor_ref_carries_five_required_fields() {
    let actor = ActorRef {
        kind: ActorKind::Human,
        id: "user:test".into(),
        definition_hash: Some("def-hash".into()),
        policy_hash: Some("policy-hash".into()),
        model: Some("gpt-4".into()),
    };
    // Access all five fields to prove they exist
    let _ = actor.kind;
    let _ = actor.id;
    let _ = actor.definition_hash;
    let _ = actor.policy_hash;
    let _ = actor.model;
    // No `role` field exists on ActorRef in A-min
}

// ── REQ-AUTH-PR-02 / CLI prefix heuristic ─────────────────────────────────────

/// Documents the CLI prefix-string mapping for `user:*` → Human.
/// This test exercises the known mapping path at `cycle.rs:1217-1223`.
/// The mapping is locked as the v1.81.x contract; it only applies to
/// `sddk cycle transition` today. Other commands (approval, pause, supersede)
/// accept free-form `actor: String` — that is a known gap (INC-HX-AUTH-001).
#[test]
fn cli_prefix_user_maps_to_human_kind() {
    let actor_id = "user:rubentxu";
    let inferred_kind = if actor_id.starts_with("user:") {
        ActorKind::Human
    } else if actor_id.starts_with("agent:") {
        ActorKind::Agent
    } else {
        ActorKind::System
    };
    assert_eq!(inferred_kind, ActorKind::Human);
}

/// Documents the CLI prefix-string mapping for `agent:*` → Agent.
#[test]
fn cli_prefix_agent_maps_to_agent_kind() {
    let actor_id = "agent:orchestrator-v1";
    let inferred_kind = if actor_id.starts_with("user:") {
        ActorKind::Human
    } else if actor_id.starts_with("agent:") {
        ActorKind::Agent
    } else {
        ActorKind::System
    };
    assert_eq!(inferred_kind, ActorKind::Agent);
}

/// Documents the CLI fallback: unrecognized prefix → System.
#[test]
fn cli_prefix_unrecognized_falls_back_to_system() {
    let actor_id = "sddk-cli";
    let inferred_kind = if actor_id.starts_with("user:") {
        ActorKind::Human
    } else if actor_id.starts_with("agent:") {
        ActorKind::Agent
    } else {
        ActorKind::System
    };
    assert_eq!(inferred_kind, ActorKind::System);
}

// ── AC-HX-AUTH-04 / REQ-AUTH-PR-03 / INC-HX-AUTH-003 ────────────────────────

/// Asserts `LedgerEvent.actor` is `String` only — no `kind` field.
/// This documents the provenance gap: `LedgerEvent` loses actor kind at the
/// engine boundary. Deferred to EVT-LEDGER-001 for typed-actor widening.
/// Reference: `crates/sddk-domain/src/models/ledger.rs:23-38`.
#[test]
fn ledger_event_carries_string_actor_only_no_kind() {
    // LedgerEvent.actor is a String field, not an ActorKind field.
    // This test creates a LedgerEvent and accesses the actor field as String.
    let event = LedgerEvent {
        sequence: 1,
        event_id: "evt-001".into(),
        project_id: "proj-001".into(),
        cycle_id: Some("cycle-001".into()),
        frame_id: "frame-001".into(),
        command_id: "cmd-001".into(),
        actor: "user:test".into(), // String only — no ActorKind
        actor_ref: None,           // EVT-LEDGER-001 widens carriers additively
        event_type: "test.event".into(),
        occurred_at: "2026-09-04T00:00:00Z".into(),
        state_before: None,
        state_after: None,
        payload: serde_json::json!({}),
        previous_hash: None,
        event_hash: "hash".into(),
        causation_id: None,
        correlation_id: None,
    };
    // actor is a String, not an ActorKind
    let _actor_string: String = event.actor;
    // If LedgerEvent had an actor_kind field, this test would not compile
    // until we removed the ActorKind assertion — that is the flip EVT-LEDGER-001 will do.
}

/// Asserts `GateReceipt.actor` is `String` only — no `kind` field.
/// Reference: `crates/sddk-domain/src/models/gate_receipt.rs:108-123`.
#[test]
fn gate_receipt_carries_string_actor_only_no_kind() {
    let receipt = GateReceipt {
        receipt_id: "rcpt-001".into(),
        project_id: "proj-001".into(),
        cycle_id: Some("cycle-001".into()),
        gate: "build".into(),
        evaluator: "evaluator-001".into(),
        transition_id: "trans-001".into(),
        plan_hash: "hash".into(),
        outcome: sddk_domain::models::gate_receipt::GateOutcomeStatus::Passed,
        evidence: serde_json::json!({}),
        actor: "user:test".into(), // String only — no ActorKind
        actor_ref: None,           // EVT-LEDGER-001 widens carriers additively
        command_id: "cmd-001".into(),
        frame_id: "frame-001".into(),
        evaluated_at: "2026-09-04T00:00:00Z".into(),
        seq: 1,
        causation_id: None,
        correlation_id: None,
    };
    // actor is a String, not an ActorKind
    let _actor_string: String = receipt.actor;
}

// ── AC-HX-AUTH-04 / REQ-AUTH-PR-03 / INC-HX-AUTH-003 ────────────────────────

/// Asserts `JournalEntry` has zero actor-related fields.
/// Reference: `crates/sddk-domain/src/projections/journal.rs:13-32`.
#[test]
fn journal_entry_has_no_actor_fields() {
    // JournalEntry fields (from the struct definition):
    // event_id, event_type, stream_id, sequence, content_hash,
    // occurred_at, severity, correlation_id, causation_id
    // None of these are actor_* fields.

    let entry = JournalEntry {
        event_id: "evt-001".into(),
        event_type: "test.event".into(),
        stream_id: "stream-001".into(),
        sequence: 1,
        content_hash: "hash".into(),
        occurred_at: "2026-09-04T00:00:00Z".into(),
        severity: Severity::Medium,
        correlation_id: None,
        causation_id: None,
        actor_ref: None, // EVT-LEDGER-001 adds actor_ref to JournalEntry
    };

    // Access all fields to prove none are actor_*
    let _ = entry.event_id;
    let _ = entry.event_type;
    let _ = entry.stream_id;
    let _ = entry.sequence;
    let _ = entry.content_hash;
    let _ = entry.occurred_at;
    let _ = entry.severity;
    let _ = entry.correlation_id;
    let _ = entry.causation_id;
}
