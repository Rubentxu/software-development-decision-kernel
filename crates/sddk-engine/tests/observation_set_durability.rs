// S4 durability (PR-UAT-024) — `observation.set.appended` roundtrip.
//
// Proves that a static-evidence `ObservationSet` survives crash/reopen
// without loss or duplication:
//   1. Serialize an `ObservationSet` into the frozen v1 payload envelope
//      (`schema_version` + `observation_set` + `content_digest`).
//   2. Append it as a ledger event via `Storage::emit_canonical_event`.
//   3. Reopen the storage (fresh handle, same file — crash/reopen path).
//   4. Validate the payload against `std_registry()` (observation.set.appended v1).
//   5. Deserialize back into an `ObservationSet` and compare with the original.
//
// Closes the S4 semantic-loss STOP (Option B): the JSON shape is owned by
// Rust types (Serialize/Deserialize on the observation tree) and the payload
// contract is registered, versioned, and validated by the EventSchemaRegistry.

use sddk_domain::event_registry::schemas::std_registry;
use sddk_domain::models::ledger::LedgerEventInput;
use sddk_engine::architectural_contract::ComponentRef;
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef};
use sddk_engine::knowledge::{EventTime, KnowledgeBasis};
use sddk_engine::observation::types::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    SoftwareEntityRef, SoftwareObservation, SoftwareRelation,
};
use sddk_engine::semantic_kind::CoreRelationKind;
use sddk_storage::Storage;
use serde_json::json;
use tempfile::tempdir;

fn unit(s: &str) -> SoftwareEntityRef {
    SoftwareEntityRef::Unit(SoftwareUnitRef::new(s))
}
fn component(s: &str) -> SoftwareEntityRef {
    SoftwareEntityRef::Component(ComponentRef::new(s).expect("component"))
}
fn relation() -> SoftwareRelation {
    SoftwareRelation::new(
        unit("unit:comp:auth"),
        CoreRelationKind::DependsOn,
        component("component:comp:log"),
    )
}
fn basis() -> ObservationBasis {
    ObservationBasis::new(
        "rev:1",
        KnowledgeBasis::empty(EventTime(1)).basis_hash().clone(),
        "input:abc",
    )
}
fn obs_basis(revision: &str, input: &str) -> ObservationBasis {
    ObservationBasis::new(
        revision,
        KnowledgeBasis::empty(EventTime(1)).basis_hash().clone(),
        input,
    )
}
fn evidence(locator: &str) -> EvidenceRef {
    EvidenceRef::new(EvidenceKind::Adhoc, locator)
}

fn sample_set() -> ObservationSet {
    let mut set = ObservationSet::new();
    set.insert(SoftwareObservation::declare(
        ObservationSubject::SoftwareRelation(relation()),
        ObservationStance::Affirms,
        evidence("ev:1"),
        ObservationOrigin::DeterministicLocal,
        basis(),
        None,
        "s4-durability-test",
    ));
    set
}

fn observation_event_input(event_id: &str, payload: serde_json::Value) -> LedgerEventInput {
    LedgerEventInput {
        event_id: event_id.into(),
        project_id: "project-s4".into(),
        cycle_id: None,
        frame_id: "frame-s4".into(),
        command_id: "command-s4".into(),
        actor: "runtime".into(),
        actor_ref: None,
        event_type: "observation.set.appended".into(),
        occurred_at: "2026-09-20T12:00:00Z".into(),
        state_before: None,
        state_after: None,
        payload,
        causation_id: None,
        correlation_id: None,
    }
}

fn payload_for(set: &ObservationSet) -> serde_json::Value {
    json!({
        "schema_version": 1,
        "observation_set": serde_json::to_value(set).expect("serialize ObservationSet"),
        "content_digest": set.canonical_digest(),
    })
}

#[test]
fn observation_set_survives_crash_reopen_without_loss_or_duplication() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("s4-durability.sqlite");

    let payload = {
        let set = sample_set();
        payload_for(&set)
    };

    // 1. Append while the process is "live".
    {
        let storage = Storage::open(&db_path).expect("open");
        storage
            .emit_canonical_event(&observation_event_input("evt-s4-1", payload.clone()))
            .expect("emit observation.set.appended");
    } // handle dropped = simulated crash

    // 2. Reopen with a fresh handle (crash/reopen path).
    let storage = Storage::open(&db_path).expect("reopen");

    // 3. Read the event back from the ledger.
    let events = storage.list_events().expect("list events");
    let evts: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "observation.set.appended")
        .collect();
    assert_eq!(evts.len(), 1, "exactly one observation event after reopen");
    let stored = evts[0];

    // 4. Validate the payload against the registered schema (v1).
    let registry = std_registry();
    let schema = registry
        .get("observation.set.appended", 1)
        .expect("schema registered");
    schema
        .validate_payload(&stored.payload)
        .expect("payload conforms to observation.set.appended v1");

    // 5. Deserialize back into an ObservationSet; loss and duplication checks.
    let original = sample_set();
    let restored: ObservationSet =
        serde_json::from_value(stored.payload["observation_set"].clone())
            .expect("deserialize ObservationSet");

    assert_eq!(
        restored, original,
        "roundtrip must preserve the set exactly (no loss, no duplication)"
    );
    assert_eq!(
        restored.canonical_digest(),
        stored.payload["content_digest"].as_str().expect("digest"),
        "restored digest must match the persisted content_digest"
    );
    assert_eq!(
        restored.canonical_digest(),
        original.canonical_digest(),
        "digest must be stable across the roundtrip"
    );
}

#[test]
fn schema_rejects_unversioned_or_malformed_payloads() {
    let registry = std_registry();
    let schema = registry
        .get("observation.set.appended", 1)
        .expect("schema registered");

    // Missing schema_version.
    let bad = json!({
        "observation_set": {"observations": []},
        "content_digest": "deadbeef"
    });
    assert!(schema.validate_payload(&bad).is_err());

    // Wrong schema_version.
    let bad = json!({
        "schema_version": 2,
        "observation_set": {"observations": []},
        "content_digest": "deadbeef"
    });
    assert!(schema.validate_payload(&bad).is_err());

    // Missing content_digest.
    let bad = json!({
        "schema_version": 1,
        "observation_set": {"observations": []}
    });
    assert!(schema.validate_payload(&bad).is_err());

    // observation_set not an object.
    let bad = json!({
        "schema_version": 1,
        "observation_set": [1, 2, 3],
        "content_digest": "deadbeef"
    });
    assert!(schema.validate_payload(&bad).is_err());

    // Valid minimal payload.
    let good = json!({
        "schema_version": 1,
        "observation_set": {"observations": []},
        "content_digest": "OBSET|empty"
    });
    assert!(
        schema.validate_payload(&good).is_ok(),
        "empty set is valid v1"
    );
}

// ── AIW-S1b gate: UAT-A10 — probe the CURRENT writer before any successor shape ──
//
// Roadmap (MILESTONES.md, AIW-S1b): "Probar primero writer actual. Si no
// permite guardar dos observaciones contradictorias con identity+basis+relation
// sin pérdida, aprobar AIW-ADR-03 ... Si no se demuestra carencia, cancelar
// slice."
//
// This test IS that probe. If it passes, the carencia is NOT demonstrated and
// AIW-S1b must be CANCELLED per the roadmap (no successor shape needed).

#[test]
fn uat_a10_current_writer_preserves_two_contradictory_observations_across_reboot() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("s1b-probe.sqlite");

    // Two observations of the SAME subject (same relation), with DIFFERENT
    // stance AND different basis/evidence — a genuine contradiction
    // (one producer affirms, another denies, each against its own basis).
    let mut live = ObservationSet::new();
    let id_affirm = {
        let o = SoftwareObservation::declare(
            ObservationSubject::SoftwareRelation(relation()),
            ObservationStance::Affirms,
            evidence("ev:affirm"),
            ObservationOrigin::StaticProvider,
            obs_basis("rev:1", "input:affirm"),
            None,
            "producer-a",
        );
        let id = o.id.clone();
        live.insert(o);
        id
    };
    let id_deny = {
        let o = SoftwareObservation::declare(
            ObservationSubject::SoftwareRelation(relation()),
            ObservationStance::Denies,
            evidence("ev:deny"),
            ObservationOrigin::StaticProvider,
            obs_basis("rev:1", "input:deny"),
            None,
            "producer-b",
        );
        let id = o.id.clone();
        live.insert(o);
        id
    };

    // Precondition for a genuine contradiction probe: both must coexist in
    // memory (distinct identity — stance and basis participate in the id),
    // and the relation must be queryable with BOTH observations attached.
    assert_ne!(
        id_affirm, id_deny,
        "contradiction requires distinct identity"
    );
    assert_eq!(live.observations().len(), 2, "no latest-wins overwrite");
    let rel_id = relation().id();
    let for_rel = live.for_relation(&rel_id);
    assert_eq!(
        for_rel.len(),
        2,
        "both stances attached to the same relation"
    );

    // Persist via the canonical v1 event and reopen (crash/reboot path).
    {
        let storage = Storage::open(&db_path).expect("open");
        storage
            .emit_canonical_event(&observation_event_input("evt-s1b-1", payload_for(&live)))
            .expect("emit");
    }
    let storage = Storage::open(&db_path).expect("reopen");
    let events = storage.list_events().expect("list");
    let stored = events
        .iter()
        .find(|e| e.event_type == "observation.set.appended")
        .expect("event present");
    std_registry()
        .get("observation.set.appended", 1)
        .expect("schema")
        .validate_payload(&stored.payload)
        .expect("schema-valid");

    let rebooted: ObservationSet =
        serde_json::from_value(stored.payload["observation_set"].clone()).expect("deserialize");

    // DUR+COND assertions (UAT-A10): identities survive, both relations
    // (stances) survive, no aggregation, no latest-wins.
    assert_eq!(rebooted, live, "set survives reboot exactly");
    assert_eq!(rebooted.observations().len(), 2);
    assert!(rebooted.observations().iter().any(|o| o.id == id_affirm));
    assert!(rebooted.observations().iter().any(|o| o.id == id_deny));
    let for_rel_after = rebooted.for_relation(&rel_id);
    assert_eq!(for_rel_after.len(), 2, "no score aggregation, no loss");
    assert!(
        for_rel_after
            .iter()
            .any(|o| o.stance == ObservationStance::Affirms)
    );
    assert!(
        for_rel_after
            .iter()
            .any(|o| o.stance == ObservationStance::Denies)
    );
}

#[test]
fn uat_a10_probe_negative_duplicate_identity_collapses() {
    // The ONE sanctioned collapse is byte-identical identity (dedup by id).
    // This is not latest-wins: same id means same content hash; the insert is
    // idempotent. Everything else coexists.
    let mut set = ObservationSet::new();
    let mk = || {
        SoftwareObservation::declare(
            ObservationSubject::SoftwareRelation(relation()),
            ObservationStance::Affirms,
            evidence("ev:same"),
            ObservationOrigin::StaticProvider,
            obs_basis("rev:1", "input:same"),
            None,
            "producer-same",
        )
    };
    let first = mk();
    let id = first.id.clone();
    set.insert(first);
    set.insert(mk());
    assert_eq!(
        set.observations().len(),
        1,
        "identical identity is idempotent insert"
    );
    assert!(set.observations().iter().any(|o| o.id == id));
}
