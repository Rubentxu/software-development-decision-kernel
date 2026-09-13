//! Canonical chain-integrity tests (AC-EVT-LEDGER-06, post-C1.5 rewrite).
//!
//! WU-C15-8: la suite original comparaba los streams legacy `ledger_events`
//! y `events_v1` (`verify_cross_ledger_consistency`). Con la tabla legacy
//! eliminada (WU-C15-4) ya no hay "dos ledgers": `events_v1` es la única
//! autoridad de eventos de dominio y su integridad se prueba directamente:
//!
//! 1. `chain_integrity_is_preserved_across_cycles` — una secuencia de
//!    eventos canónicos a través de varios ciclos mantiene la cadena
//!    verificable (`verify_ledger` verde, conteo exacto).
//! 2. `tampered_event_breaks_chain_verification` — una mutación ilegal de
//!    una fila existente de `events_v1` rompe la verificación de cadena
//!    (fallo detectable, no silencioso).
//! 3. `events_v1_is_append_only` — los triggers de inmutabilidad
//!    (`events_v1_no_update` / `events_v1_no_delete`) rechazan UPDATE y
//!    DELETE sobre filas ya escritas.
//! 4. `canonical_write_is_visible_in_canical_read_view` — la única vía de
//!    escritura de dominio (`emit_canonical_event`) persiste y es visible
//!    vía `list_events` (conservado de WU-C15-5).

use rusqlite::Connection;
use sddk_domain::{LedgerEventInput, ProjectRecord};
use sddk_storage::Storage;
use std::path::Path;
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

fn canonical_input(event_id: &str, cycle_id: Option<&str>) -> LedgerEventInput {
    LedgerEventInput {
        event_id: event_id.into(),
        project_id: "p-test".into(),
        cycle_id: cycle_id.map(str::to_owned),
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
    }
}

/// Opens an independent read-write connection over the same database file
/// (the tampering path a rogue writer would use).
fn raw_connection(db_path: &Path) -> Connection {
    Connection::open(db_path).expect("open raw connection")
}

/// Sección 1: la cadena canónica sobrevive escrituras multi-ciclo y
/// `verify_ledger` la valida completa.
#[test]
fn chain_integrity_is_preserved_across_cycles() {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("ledger.sqlite")).unwrap();

    storage.insert_project(&project_record()).unwrap();
    storage
        .emit_canonical_event(&canonical_input("evt-chain-1", Some("c-1")))
        .unwrap();
    storage
        .emit_canonical_event(&canonical_input("evt-chain-2", Some("c-1")))
        .unwrap();
    storage
        .emit_canonical_event(&canonical_input("evt-chain-3", Some("c-2")))
        .unwrap();

    let verification = storage.verify_ledger().expect("chain must verify");
    assert_eq!(verification.event_count, 3);
    assert!(verification.last_hash.is_some());
}

fn tampered_chain_fixture(dir: &TempDir) -> (Storage, std::path::PathBuf) {
    let db_path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage
        .emit_canonical_event(&canonical_input("evt-tamper-1", Some("c-1")))
        .unwrap();
    storage
        .emit_canonical_event(&canonical_input("evt-tamper-2", Some("c-1")))
        .unwrap();
    assert!(
        storage.verify_ledger().is_ok(),
        "pre-tamper chain must verify"
    );
    (storage, db_path)
}

/// Sección 2: mutar una fila existente de `events_v1` rompe la verificación
/// de cadena (el fallo es detectable, no silencioso). La mutación necesita
/// el trigger de UPDATE desactivado — los triggers normales la bloquean
/// (sección 3), así que este test lo desactiva explícitamente para simular
/// la vía de un escritor rogue con acceso SQL directo.
#[test]
fn tampered_event_breaks_chain_verification() {
    let dir = TempDir::new().unwrap();
    let (storage, db_path) = tampered_chain_fixture(&dir);

    // Rogue write: drop the append-only guard, mutate a payload, restore
    // the guard. The stored content no longer matches its content_hash.
    let conn = raw_connection(&db_path);
    conn.execute_batch(
        "DROP TRIGGER IF EXISTS events_v1_no_update;
         UPDATE events_v1 SET payload_json = '{\"tampered\":true}'
         WHERE event_id = 'evt-tamper-1';
         CREATE TRIGGER events_v1_no_update BEFORE UPDATE ON events_v1
         BEGIN SELECT RAISE(ABORT, 'events_v1 is append-only'); END;",
    )
    .expect("simulate rogue tampering");

    assert!(
        storage.verify_ledger().is_err(),
        "a mutated canonical row must break chain verification"
    );
}

/// Sección 3: `events_v1` es append-only por triggers — UPDATE y DELETE
/// sobre filas existentes se rechazan en la capa SQL, sin depender de la
/// disciplina del código de aplicación.
#[test]
fn events_v1_is_append_only() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).unwrap();

    storage.insert_project(&project_record()).unwrap();
    storage
        .emit_canonical_event(&canonical_input("evt-appendonly-1", Some("c-1")))
        .unwrap();

    let conn = raw_connection(&db_path);
    let update = conn.execute(
        "UPDATE events_v1 SET payload_json = '{\"x\":1}' WHERE event_id = 'evt-appendonly-1'",
        [],
    );
    assert!(update.is_err(), "UPDATE must be rejected by trigger");
    let delete = conn.execute(
        "DELETE FROM events_v1 WHERE event_id = 'evt-appendonly-1'",
        [],
    );
    assert!(delete.is_err(), "DELETE must be rejected by trigger");
}

/// WU-C15-5: la única vía de escritura de dominio es el stream canónico
/// `events_v1`: `emit_canonical_event` persiste y el evento es visible vía
/// `list_events`.
#[test]
fn canonical_write_is_visible_in_canical_read_view() {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("ledger.sqlite")).unwrap();

    storage.insert_project(&project_record()).unwrap();
    let appended = storage
        .emit_canonical_event(&canonical_input("evt-guard-1", None))
        .expect("canonical append must work (single write authority)");
    assert_eq!(appended.event_type, "workflow.phase.entered");

    // The canonical read view reflects the append: the event is visible
    // through the canonical list (C1.5; the cross-ledger report was removed
    // with the legacy read layer).
    let listed = storage.list_events().expect("list_events");
    assert!(
        listed.iter().any(|e| e.event_id == "evt-guard-1"),
        "canonical append output must be readable via list_events"
    );
}
