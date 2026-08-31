//! Property tests for ledger hashing and append verification.

use proptest::prelude::*;
use sddk_storage::{CapabilityStatus, LedgerEventInput, ProjectRecord, Storage, WorkspaceRecord};

fn storage_with_project() -> Storage {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-1".into(),
            display_name: "project".into(),
            remote_url: Some("https://example.com/owner/project".into()),
            scope: "owner".into(),
            created_at: "2026-08-04T10:00:00Z".into(),
        })
        .unwrap();
    storage
        .insert_workspace(&WorkspaceRecord {
            workspace_id: "workspace-1".into(),
            project_id: "project-1".into(),
            canonical_path: "/work/project".into(),
            created_at: "2026-08-04T10:00:00Z".into(),
        })
        .unwrap();
    storage
}

fn event(sequence_seed: &str, payload: &str) -> LedgerEventInput {
    LedgerEventInput {
        event_id: format!("evt-{sequence_seed}"),
        project_id: "project-1".into(),
        cycle_id: None,
        frame_id: format!("frame-{sequence_seed}"),
        command_id: format!("command-{sequence_seed}"),
        actor: "property-test".into(),
        event_type: "test.event".into(),
        occurred_at: "2026-08-04T10:00:00Z".into(),
        state_before: None,
        state_after: None,
        payload: serde_json::json!({"seed": sequence_seed, "data": payload}),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn appended_ledger_always_verifies(events in prop::collection::vec(any::<u8>(), 0..12)) {
        let mut storage = storage_with_project();
        for (index, byte) in events.iter().enumerate() {
            storage.append_event(&event(&format!("{index}-{byte}"), &byte.to_string())).unwrap();
        }
        let verification = storage.verify_ledger().unwrap();
        prop_assert_eq!(verification.event_count, events.len());
        if events.is_empty() {
            prop_assert!(verification.last_hash.is_none());
        } else {
            prop_assert!(verification.last_hash.is_some());
        }
    }

    #[test]
    fn distinct_payloads_yield_distinct_hashes(payload_a in ".{0,16}", payload_b in ".{0,16}") {
        prop_assume!(payload_a != payload_b);
        let mut storage = storage_with_project();
        let first = storage.append_event(&event("a", &payload_a)).unwrap();
        let second = storage.append_event(&event("b", &payload_b)).unwrap();
        prop_assert_ne!(first.event_hash, second.event_hash);
    }
}

#[test]
fn capability_status_serialization_is_stable() {
    for status in [
        CapabilityStatus::Started,
        CapabilityStatus::Succeeded,
        CapabilityStatus::Failed,
        CapabilityStatus::Unknown,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let roundtrip: CapabilityStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, status);
    }
}
