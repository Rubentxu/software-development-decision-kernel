//! Canned test data fixtures for `sddk-testkit`.
//!
//! Pre-built [`InMemoryLedger`] snapshots and helper constructors that encode
//! common SDDK testing scenarios.  Each fixture is documented with its intended
//! use case so tests remain self-documenting.

use crate::{CycleBuilder, EventBuilder, InMemoryLedger};
use sddk_domain::{CyclePath, Ledger};

/// Builds an empty [`InMemoryLedger`] with no events and no cycles.
///
/// # Use case
/// Baseline for tests that only need the ledger interface (e.g. engine
/// unit tests that supply their own data).
pub fn empty_ledger() -> InMemoryLedger {
    InMemoryLedger::new()
}

/// Builds a ledger containing a single active cycle in the `Explore` phase.
///
/// Returns the ledger (already populated) and the [`CycleRecord`] that was inserted.
///
/// # Use case
/// Tests that exercise cycle-state transitions without a full project setup.
pub fn single_cycle_ledger(cycle_id: &str) -> (InMemoryLedger, sddk_domain::CycleRecord) {
    let mut ledger = InMemoryLedger::new();
    let cycle = CycleBuilder::new(CyclePath::AFull)
        .with_id(cycle_id)
        .with_phase(sddk_domain::Phase::Explore)
        .build();
    let event_input = EventBuilder::new("cycle.created")
        .with_cycle(cycle_id)
        .with_project(&cycle.manifest.project_id)
        .build();

    let _inserted = ledger
        .insert_cycle_with_event(&cycle, &event_input)
        .expect("fixtures must not fail");
    (ledger, cycle)
}

/// Builds a ledger containing two sequential events on the same cycle.
///
/// Returns the ledger and a vector of the two inserted [`LedgerEvent`]s.
///
/// # Use case
/// Tests that need to verify hash-chaining, sequence ordering, or
/// event enumeration over a multi-event stream.
pub fn two_event_stream(
    cycle_id: &str,
    project_id: &str,
) -> (InMemoryLedger, Vec<sddk_domain::LedgerEvent>) {
    let mut ledger = InMemoryLedger::new();
    let cycle = CycleBuilder::new(CyclePath::BDirect)
        .with_id(cycle_id)
        .with_project(project_id)
        .build();

    let ev1 = EventBuilder::new("cycle.created")
        .with_cycle(cycle_id)
        .with_project(project_id)
        .build();
    let inserted1 = ledger
        .insert_cycle_with_event(&cycle, &ev1)
        .expect("fixtures must not fail");

    let ev2 = EventBuilder::new("phase.changed")
        .with_cycle(cycle_id)
        .with_project(project_id)
        .with_payload(serde_json::json!({
            "from": "explore",
            "to": "spec"
        }))
        .build();
    let inserted2 = ledger
        .insert_cycle_with_event(&cycle, &ev2)
        .expect("fixtures must not fail");

    (ledger, vec![inserted1, inserted2])
}

/// Builds a ledger with an active cycle lease held by a specific owner.
///
/// Returns the ledger, cycle record, and the lease acquired.
///
/// # Use case
/// Tests that exercise lease acquisition, renewal, release, or fencing-token
/// mismatch scenarios.
pub fn ledger_with_lease(
    cycle_id: &str,
    owner: &str,
    now_ms: i64,
    expires_ms: i64,
) -> (
    InMemoryLedger,
    sddk_domain::CycleRecord,
    sddk_domain::CycleLease,
) {
    let mut ledger = InMemoryLedger::new();
    let cycle = CycleBuilder::new(CyclePath::AFull)
        .with_id(cycle_id)
        .build();

    let ev_input = EventBuilder::new("cycle.created")
        .with_cycle(cycle_id)
        .with_project(&cycle.manifest.project_id)
        .build();
    ledger
        .insert_cycle_with_event(&cycle, &ev_input)
        .expect("fixtures must not fail");

    let lease = ledger
        .acquire_cycle_lease(cycle_id, owner, now_ms, expires_ms)
        .expect("fixtures must not fail");

    (ledger, cycle, lease)
}

/// Builds a ledger containing one gate receipt with a "pass" outcome.
///
/// # Use case
/// Tests that exercise gate receipt lookup or verification.
pub fn ledger_with_pass_gate_receipt(
    cycle_id: &str,
    project_id: &str,
    gate: &str,
) -> (InMemoryLedger, sddk_domain::GateReceipt) {
    let mut ledger = InMemoryLedger::new();

    let input = sddk_domain::GateReceiptNextSeqInput {
        project_id: project_id.to_string(),
        cycle_id: Some(cycle_id.to_string()),
        gate: gate.to_string(),
        evaluator: "sddk-testkit".to_string(),
        transition_id: format!("t-{}", uuid::Uuid::new_v4()),
        plan_hash: "sha256:deadbeef".to_string(),
        outcome: sddk_domain::GateOutcomeStatus::Passed,
        evidence: serde_json::json!({ "fixture": true }),
        actor: "sddk-testkit".to_string(),
        actor_ref: None,
        evaluated_at: "2026-08-19T00:00:00Z".to_string(),
        command_id: format!("cmd-{}", uuid::Uuid::new_v4()),
        frame_id: format!("frame-{}", uuid::Uuid::new_v4()),
    };

    let receipt = ledger
        .insert_gate_receipt_next_seq(&input)
        .expect("fixtures must not fail");

    (ledger, receipt)
}

/// Returns a stable "genesis" event for use in hash-chain tests.
///
/// The event has `event_id = "evt-genesis"`, `sequence = 1`, and a
/// predictable `event_hash` so tests can anchor assertions on a known
/// starting point without computing SHA-256 in the test itself.
pub fn genesis_event() -> sddk_domain::LedgerEventInput {
    EventBuilder::new("ledger.genesis")
        .with_event_id("evt-genesis")
        .with_project("p-test")
        .occurred_at("2026-01-01T00:00:00Z")
        .with_payload(serde_json::json!({ "genesis": true }))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger_has_zero_sequence() {
        let ledger = empty_ledger();
        assert_eq!(ledger.sequence(), 0);
        assert!(ledger.events().is_empty());
    }

    #[test]
    fn single_cycle_ledger_inserts_one_event() {
        let (ledger, cycle) = single_cycle_ledger("c-test");
        assert_eq!(ledger.events().len(), 1);
        assert_eq!(cycle.manifest.cycle_id, "c-test");
    }

    #[test]
    fn two_event_stream_has_correct_sequence() {
        let (_ledger, events) = two_event_stream("c-test", "p-test");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
        // Verify hash chaining
        assert_eq!(
            events[1].previous_hash.as_deref(),
            Some(events[0].event_hash.as_str())
        );
    }

    #[test]
    fn ledger_with_lease_returns_valid_lease() {
        let (_ledger, _cycle, lease) = ledger_with_lease("c-test", "owner-a", 1000, 2000);
        assert_eq!(lease.cycle_id, "c-test");
        assert_eq!(lease.owner, "owner-a");
        assert_eq!(lease.expires_at_ms, 2000);
    }

    #[test]
    fn ledger_with_pass_gate_receipt_works() {
        let (_ledger, receipt) = ledger_with_pass_gate_receipt("c-test", "p-test", "verify");
        assert_eq!(receipt.gate, "verify");
        assert!(matches!(
            receipt.outcome,
            sddk_domain::GateOutcomeStatus::Passed
        ));
    }
}
