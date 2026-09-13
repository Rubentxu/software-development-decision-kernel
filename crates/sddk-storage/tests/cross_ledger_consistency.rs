//! Cross-ledger consistency tests (AC-EVT-LEDGER-06).
//!
//! WU-C15-3: `verify_cross_ledger_consistency` fue eliminado con el read
//! layer legacy (C1.5). Los tests de divergencia quedan deshabilitados hasta
//! que WU-C15-8 reescriba la suite como chain-integrity canónica. El test de
//! write-guard canónico se conserva abajo.

#![allow(deprecated)] // tests exercise the C1.3-deprecated forwarders / read-compat API by design

use rusqlite::{Connection, params};
use sddk_domain::{ActorKind, ActorRef, EventEnvelopeV1, ProjectRecord, WorkspaceRecord};
use sddk_storage::Storage;
use tempfile::TempDir;

const CREATED_AT: &str = "2026-09-01T12:00:00Z";

fn project_record() -> ProjectRecord {
    ProjectRecord {
        project_id: "p-test".into(),
        display_name: "Test Project".into(),
        remote_url: Some("https://example.com/test".into()),
        scope: ".".into(),
        created_at: CREATED_AT.into(),
    }
}

// Helpers de seed legacy: en desuso tras eliminar la suite cross-check en
// WU-C15-3; WU-C15-8 reescribe esta suite sobre events_v1 only.
#[allow(dead_code)]
fn workspace_record() -> WorkspaceRecord {
    WorkspaceRecord {
        workspace_id: "ws-test".into(),
        project_id: "p-test".into(),
        canonical_path: "/tmp/test".into(),
        created_at: CREATED_AT.into(),
    }
}

#[allow(dead_code)]
fn minimal_envelope(event_id: &str, stream_id: &str, project_id: &str) -> EventEnvelopeV1 {
    let mut env = EventEnvelopeV1 {
        event_id: event_id.into(),
        event_type: "workflow.phase.entered".into(),
        schema_version: 1,
        stream_id: stream_id.into(),
        sequence: 0,
        project_id: project_id.into(),
        occurred_at: "2026-09-01T10:00:00Z".into(),
        recorded_at: "2026-09-01T10:00:01Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "sddk-test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: None,
        },
        subjects: vec![],
        payload: serde_json::json!({}),
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some("c-1".into()),
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

/// Inserts an event into the events_v1 table using raw SQL on a shared connection.
#[allow(dead_code)]
fn insert_events_v1(conn: &Connection, env: &EventEnvelopeV1) {
    // Get next sequence for this stream
    let actor_json = serde_json::to_string(&env.actor).unwrap();
    let subjects_json = serde_json::to_string(&env.subjects).unwrap();
    let payload_json = serde_json::to_string(&env.payload).unwrap();
    let evidence_refs_json = serde_json::to_string(&env.evidence_refs).unwrap();
    let metadata_json = serde_json::to_string(&env.metadata).unwrap();
    let causation_id: Option<String> = env.causation_id.clone();
    let correlation_id: Option<String> = env.correlation_id.clone();
    let cycle_id: Option<String> = env.cycle_id.clone();
    let frame_id: Option<String> = env.frame_id.clone();
    let fork_id: Option<String> = env.fork_id.clone();

    // Get next sequence for this stream
    let next_seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM events_v1 WHERE stream_id = ?1",
            params![env.stream_id],
            |row| row.get(0),
        )
        .unwrap_or(1);

    // Compute chain_hash (simplified for tests: empty string)
    let chain_hash = "";

    conn.execute(
        "INSERT INTO events_v1 \
         (event_id, event_type, schema_version, stream_id, sequence, project_id, \
          occurred_at, recorded_at, actor_json, subjects_json, payload_json, \
          evidence_refs_json, content_hash, metadata_json, causation_id, \
          correlation_id, cycle_id, frame_id, fork_id, chain_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            env.event_id,
            env.event_type,
            env.schema_version,
            env.stream_id,
            next_seq,
            env.project_id,
            env.occurred_at,
            env.recorded_at,
            actor_json,
            subjects_json,
            payload_json,
            evidence_refs_json,
            env.content_hash,
            metadata_json,
            causation_id,
            correlation_id,
            cycle_id,
            frame_id,
            fork_id,
            chain_hash,
        ],
    )
    .expect("insert events_v1");
}

/// WU-C1.2: desde el redirect, `append_event` ya no escribe en
/// `ledger_events`. Los fixtures que necesitan una fila legacy siembran la
/// tabla directamente (mismo mecanismo que usan para `events_v1`).
#[allow(dead_code)]
fn insert_legacy_event(conn: &Connection, event_id: &str) {
    conn.execute(
        "INSERT INTO ledger_events (
            sequence, event_id, project_id, cycle_id, frame_id, command_id,
            actor, event_type, occurred_at, state_before_json,
            state_after_json, payload_json, previous_hash, event_hash
         ) VALUES (
            COALESCE((SELECT MAX(sequence) FROM ledger_events), 0) + 1,
            ?1, 'p-test', NULL, 'frame-1', 'cmd-1', 'system',
            'workflow.phase.entered', '2026-09-01T10:00:00Z',
            NULL, NULL, '{}', NULL, 'sha256:legacy'
         )",
        params![event_id],
    )
    .expect("insert legacy event");
}

// =============================================================================
// AC-EVT-LEDGER-06: verify_cross_ledger_consistency
// =============================================================================

// =============================================================================
// WU-C1.3 / CLOSE-01: legacy domain write hard-disable
// =============================================================================

/// C1.3 hard-disable: attempting a domain-event append through the legacy
/// `ledger_events` write path must fail with the typed
/// `StorageError::LegacyDomainWriteForbidden` error, never write a row.
#[test]
fn legacy_domain_write_is_rejected_with_typed_error() {
    use sddk_domain::SddkErrorCode;
    use sddk_storage::legacy_domain_write_forbidden;

    let dir = TempDir::new().unwrap();
    let mut storage = Storage::open(dir.path().join("ledger.sqlite")).unwrap();

    // The direct typed constructor: the only legacy-domain-write factory left
    // in the tree (`append_event_on` is `#[cfg(test)]`-gated and always
    // returns this error).
    let err = legacy_domain_write_forbidden("cycle.transitioned");
    assert_eq!(
        err.code(),
        "STORAGE_LEGACY_DOMAIN_WRITE_FORBIDDEN",
        "typed error code must be stable (event_store:<code> prefix contract): {err:?}"
    );
    let message = err.to_string();
    assert!(
        message.contains("legacy ledger_events write forbidden"),
        "error message must name the forbidden legacy path: {message}"
    );
    assert!(
        message.contains("cycle.transitioned"),
        "error message must name the rejected event type: {message}"
    );

    // The guard is the terminal state of the strangler: `append_event` (the
    // public wrapper) redirects to the canonical stream and SUCCEEDS — the
    // rejection is for raw legacy writes only.
    storage.insert_project(&project_record()).unwrap();
    let writable = &mut storage;
    let redirected = writable
        .append_event(&sddk_storage::LedgerEventInput {
            event_id: "evt-guard-1".into(),
            project_id: "p-test".into(),
            cycle_id: None,
            frame_id: "frame-1".into(),
            command_id: "cmd-1".into(),
            actor: "system".into(),
            actor_ref: None,
            event_type: "workflow.phase.entered".into(),
            occurred_at: "2026-09-01T10:00:00Z".into(),
            state_before: None,
            state_after: None,
            payload: serde_json::json!({}),
            causation_id: None,
            correlation_id: None,
        })
        .expect("canonical redirect must still work after the hard-disable");
    assert_eq!(redirected.event_type, "workflow.phase.entered");

    // And the canonical-only read view reflects the redirect: the event is
    // visible through the canonical list (C1.5; the cross-ledger report was
    // removed with the legacy read layer).
    let listed = storage.list_events().expect("list_events");
    assert!(
        listed.iter().any(|e| e.event_id == "evt-guard-1"),
        "canonical redirect output must be readable via list_events"
    );
}
