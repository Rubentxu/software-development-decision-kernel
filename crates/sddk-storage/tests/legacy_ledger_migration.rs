//! Canonical event stream export/recovery fixture (C1.5, WU-C15-4 rewrite).
//!
//! Legacy `ledger_events` is physically gone (MIGRATION_20). This suite now
//! validates export/import over the canonical `events_v1` stream ONLY:
//!
//! 1. Dado un Storage con N eventos canónicos (`emit_canonical_event` path).
//! 2. Cuando exportamos los eventos a un fixture en memoria
//!    (Vec<LedgerEventInput> + count + digest sha256), reconstruimos un
//!    Storage nuevo desde el fixture y lo volvemos a exportar.
//! 3. Entonces: la segunda exportación es idéntica a la primera (idempotente),
//!    counts y digest coinciden, un "restart" (reabrir el storage desde la
//!    misma ruta) produce el mismo digest, y una re-importación interrumpida
//!    a la mitad (failure injection: drop del storage) seguida de un reintento
//!    desde el fixture deja el mismo estado final.
//!
//! Nota sobre failure injection: soltar (`drop`) el Storage a mitad de la
//! re-importación deja un fichero SQLite parcialmente poblado; reintentar
//! sobre ese mismo fichero duplicaría claves de evento. El reintento canónico
//! desde el fixture se hace sobre un Storage NUEVO (path fresco), que es
//! exactamente lo que haría el recovery tooling real.

use sddk_domain::{LedgerEventInput, ProjectRecord};
use sddk_storage::Storage;
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const CREATED_AT: &str = "2026-09-01T12:00:00Z";

fn project_record() -> ProjectRecord {
    ProjectRecord {
        project_id: "p-mig".into(),
        display_name: "Migration Test Project".into(),
        remote_url: None,
        scope: ".".into(),
        created_at: CREATED_AT.into(),
    }
}

fn sample_input(i: usize) -> LedgerEventInput {
    LedgerEventInput {
        event_id: format!("evt-mig-{i:04}"),
        project_id: "p-mig".into(),
        cycle_id: None,
        frame_id: format!("frame-{i:04}"),
        command_id: format!("cmd-{i:04}"),
        actor: "system".into(),
        actor_ref: None,
        event_type: "workflow.phase.entered".into(),
        occurred_at: format!("2026-09-01T10:00:{:02}Z", i % 60),
        state_before: None,
        state_after: Some(json!({"phase": i})),
        payload: json!({"index": i, "note": "canonical migration fixture"}),
        causation_id: (i > 0).then(|| format!("evt-mig-{:04}", i - 1)),
        correlation_id: Some("frame-0000".into()),
    }
}

/// Helper de export (dentro del test, no en lib.rs): carga todos los eventos
/// del stream canónico (`events_v1` vía `list_events`) y produce un fixture
/// serializable con count + digest sha256. Digest calculado sobre la
/// serialización JSON canónica de los inputs en orden de secuencia.
fn export_fixture(storage: &Storage) -> (Vec<LedgerEventInput>, usize, String) {
    let events = storage.list_events().expect("list_events");
    let inputs: Vec<LedgerEventInput> = events.iter().map(|e| e.as_input()).collect();
    let count = inputs.len();
    let mut hasher = Sha256::new();
    for input in &inputs {
        let bytes = serde_json::to_vec(input).expect("serialize ledger input");
        hasher.update(&bytes);
    }
    let digest = format!("sha256:{:x}", hasher.finalize());
    (inputs, count, digest)
}

/// Reconstruye un Storage nuevo (path fresco) a partir del fixture, sembrando
/// por el path canónico (append-forwarding al stream events_v1).
/// Devuelve el storage; el caller decide cuándo dropearlo (failure injection).
fn import_fixture(dir: &TempDir, name: &str, inputs: &[LedgerEventInput]) -> Storage {
    let mut storage = Storage::open(dir.path().join(name)).expect("open fresh storage");
    storage
        .insert_project(&project_record())
        .expect("insert project");
    for input in inputs {
        append_canonical(&mut storage, input);
    }
    storage
}

/// Seed path canónico desde WU-C15-5: `emit_canonical_event` es la vía
/// pública al stream `events_v1` (el wrapper deprecated `append_event`
/// fue eliminado junto a la tabla `ledger_events`).
fn append_canonical(storage: &mut Storage, input: &LedgerEventInput) {
    storage
        .emit_canonical_event(input)
        .expect("append canonical event");
}

#[test]
fn canonical_export_reimport_is_idempotent_and_restart_safe() {
    let n = 25usize;

    // --- Given: storage original con N eventos canónicos (events_v1) ---
    let dir = TempDir::new().unwrap();
    let original_path = dir.path().join("ledger.sqlite");
    {
        let mut storage = Storage::open(&original_path).expect("open original storage");
        storage.insert_project(&project_record()).unwrap();
        for i in 0..n {
            append_canonical(&mut storage, &sample_input(i));
        }
        // storage dropeado al salir del bloque = primer "restart" pendiente
    }

    // --- Export 1 desde el storage persistido ---
    let storage_a = Storage::open(&original_path).unwrap();
    let (fixture_a, count_a, digest_a) = export_fixture(&storage_a);
    assert_eq!(
        count_a, n,
        "original storage debe contener exactamente N eventos"
    );
    assert_eq!(fixture_a.len(), count_a);

    // --- When: reconstruir un Storage nuevo desde el fixture y re-exportar ---
    let rebuilt = import_fixture(&dir, "rebuilt.sqlite", &fixture_a);
    let (fixture_b, count_b, digest_b) = export_fixture(&rebuilt);
    drop(rebuilt);

    // --- Then: idempotencia ---
    assert_eq!(count_b, count_a, "count debe coincidir tras re-import");
    assert_eq!(
        digest_b, digest_a,
        "digest debe coincidir tras re-import (idempotente)"
    );
    assert_eq!(
        fixture_a, fixture_b,
        "fixtures deben ser idénticos campo a campo"
    );

    // --- Restart: reabrir el storage reconstruido desde la misma ruta ---
    let reopened = Storage::open(dir.path().join("rebuilt.sqlite")).unwrap();
    let (_fixture_c, count_c, digest_c) = export_fixture(&reopened);
    drop(reopened);
    assert_eq!(count_c, count_a, "count sobrevive al restart");
    assert_eq!(digest_c, digest_a, "digest sobrevive al restart");

    // --- Failure injection: re-import interrumpida a la mitad ---
    let half = n / 2;
    {
        // El append canónico hace commit por evento, así que la
        // "interrupción" se modela dejando la importación incompleta (solo la
        // primera mitad) y dropeando el storage sin completarla.
        let aborted = import_fixture(&dir, "aborted.sqlite", &fixture_a[..half]);
        let (_, partial_count, _) = export_fixture(&aborted);
        assert_eq!(
            partial_count, half,
            "import abortada debe dejar exactamente la mitad"
        );
        drop(aborted); // drop del storage = interrupción sin completar la importación
    }

    // --- Reintento canónico desde el fixture (storage NUEVO, path fresco) ---
    let recovered = import_fixture(&dir, "recovered.sqlite", &fixture_a);
    let (fixture_d, count_d, digest_d) = export_fixture(&recovered);
    drop(recovered);

    assert_eq!(
        count_d, count_a,
        "recuperación debe restaurar el count completo"
    );
    assert_eq!(
        digest_d, digest_a,
        "recuperación debe restaurar el digest exacto"
    );
    assert_eq!(
        fixture_d, fixture_a,
        "estado final tras recovery == fixture original"
    );

    // --- El storage original sigue intacto (solo lectura/exportación) ---
    let final_check = Storage::open(&original_path).unwrap();
    let (_, count_final, digest_final) = export_fixture(&final_check);
    assert_eq!(count_final, count_a);
    assert_eq!(
        digest_final, digest_a,
        "original no debe mutar por exportar"
    );
}

/// C1.5 pin: the post-v20 schema physically has no `ledger_events` table,
/// index, or triggers (fresh repository path, v0 → v20 in one open).
#[test]
fn fresh_v0_schema_has_no_legacy_table() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&path).expect("open fresh storage");

    assert_eq!(storage.schema_version().unwrap(), 20);

    let conn = storage.connection_for_tests();
    let legacy_objects: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'ledger_events%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        legacy_objects, 0,
        "sqlite_master must contain no ledger_events objects after MIGRATION_20"
    );
}

/// C1.5 pin: a migrated v19 database carrying the legacy table (with rows)
/// drops it on first open (v19 → v20), and re-opening is idempotent.
#[test]
fn migrated_v19_database_drops_legacy_table_on_upgrade() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ledger.sqlite");

    // Build a v19-shaped database WITH the legacy ledger_events table seeded
    // with rows, using the exact pre-C1.5 MIGRATION_1 DDL fragment.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE projects (
                project_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                remote_url TEXT,
                scope TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE workspaces (
                workspace_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(project_id),
                canonical_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE cycles (
                cycle_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                status TEXT NOT NULL,
                phase TEXT NOT NULL,
                manifest_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(project_id, cycle_id)
            );
            CREATE TABLE ledger_events (
                sequence INTEGER PRIMARY KEY CHECK (sequence > 0),
                event_id TEXT NOT NULL UNIQUE,
                project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
                cycle_id TEXT,
                frame_id TEXT NOT NULL,
                command_id TEXT NOT NULL,
                actor TEXT NOT NULL,
                event_type TEXT NOT NULL,
                occurred_at TEXT NOT NULL,
                state_before_json TEXT,
                state_after_json TEXT,
                payload_json TEXT NOT NULL,
                previous_hash TEXT,
                event_hash TEXT NOT NULL UNIQUE,
                CHECK (
                    (sequence = 1 AND previous_hash IS NULL)
                    OR (sequence > 1 AND previous_hash IS NOT NULL)
                ),
                FOREIGN KEY (project_id, cycle_id)
                    REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
            );
            CREATE INDEX ledger_events_cycle_sequence_idx
                ON ledger_events(cycle_id, sequence);
            CREATE TRIGGER ledger_events_no_update
            BEFORE UPDATE ON ledger_events
            BEGIN
                SELECT RAISE(ABORT, 'ledger events are append-only');
            END;
            CREATE TRIGGER ledger_events_no_delete
            BEFORE DELETE ON ledger_events
            BEGIN
                SELECT RAISE(ABORT, 'ledger events are append-only');
            END;
            "#,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects VALUES ('p-mig', 'Migration Test Project', NULL, '.', '2026-09-01T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ledger_events (
                sequence, event_id, project_id, cycle_id, frame_id, command_id,
                actor, event_type, occurred_at, payload_json, event_hash
             ) VALUES (
                1, 'evt-legacy-0001', 'p-mig', NULL, 'frame-0001', 'cmd-0001',
                'system', 'workflow.phase.entered', '2026-09-01T10:00:00Z',
                '{\"index\":0}', 'sha256:deadbeef'
             )",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 19).unwrap();
    }

    // First open with v20 code: MIGRATION_20 runs and drops the corpus.
    let storage = Storage::open(&path).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 20);
    let conn = storage.connection_for_tests();
    let legacy_objects: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'ledger_events%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        legacy_objects, 0,
        "v19→v20 upgrade must drop every ledger_events object"
    );
    drop(storage);

    // Re-open is idempotent: version stays 20, no error from the IF EXISTS DDL.
    let reopened = Storage::open(&path).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), 20);
    let conn = reopened.connection_for_tests();
    let legacy_objects: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'ledger_events%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(legacy_objects, 0, "re-open must stay legacy-free");
}
