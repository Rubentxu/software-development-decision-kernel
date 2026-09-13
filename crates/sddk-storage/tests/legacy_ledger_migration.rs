//! Legacy `ledger_events` export/recovery fixture gate (WU-C1.1).
//!
//! Gate destructivo PREVIO a cualquier cutover/removal de la tabla legacy
//! `ledger_events`. Este test SOLO lee y exporta: no toca writers ni esquema.
//!
//! Escenario:
//! 1. Dado un Storage con N eventos en `ledger_events` (APIs públicas).
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
//! sobre ese mismo fichero duplicaría secuencias (el hash-chain se deriva del
//! último evento, ver `append_event_on` en lib.rs). El reintento canónico
//! desde el fixture se hace sobre un Storage NUEVO (path fresco), que es
//! exactamente lo que haría el recovery tooling real.

use serde_json::json;
use sha2::{Digest, Sha256};
use sddk_domain::{LedgerEventInput, ProjectRecord};
use sddk_storage::Storage;
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
        payload: json!({"index": i, "note": "legacy migration fixture"}),
        causation_id: (i > 0).then(|| format!("evt-mig-{:04}", i - 1)),
        correlation_id: Some("frame-0000".into()),
    }
}

/// Helper de export (dentro del test, no en lib.rs): carga todos los eventos
/// del ledger y produce un fixture serializable con count + digest sha256.
/// Digest calculado sobre la serialización JSON canónica de los inputs en
/// orden de secuencia.
fn export_fixture(storage: &Storage) -> (Vec<LedgerEventInput>, usize, String) {
    let events = storage.load_all_ledger_events().expect("load_all_ledger_events");
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

/// Reconstruye un Storage nuevo (path fresco) a partir del fixture.
/// Devuelve el storage; el caller decide cuándo dropearlo (failure injection).
fn import_fixture(dir: &TempDir, name: &str, inputs: &[LedgerEventInput]) -> Storage {
    let mut storage = Storage::open(dir.path().join(name)).expect("open fresh storage");
    storage.insert_project(&project_record()).expect("insert project");
    for input in inputs {
        storage.append_event(input).expect("append fixture event");
    }
    storage
}

#[test]
fn legacy_ledger_export_reimport_is_idempotent_and_restart_safe() {
    let n = 25usize;

    // --- Given: storage original con N eventos en ledger_events ---
    let dir = TempDir::new().unwrap();
    let original_path = dir.path().join("ledger.sqlite");
    {
        let mut storage = Storage::open(&original_path).expect("open original storage");
        storage.insert_project(&project_record()).unwrap();
        for i in 0..n {
            storage.append_event(&sample_input(i)).unwrap();
        }
        // storage dropeado al salir del bloque = primer "restart" pendiente
    }

    // --- Export 1 desde el storage persistido ---
    let storage_a = Storage::open(&original_path).unwrap();
    let (fixture_a, count_a, digest_a) = export_fixture(&storage_a);
    assert_eq!(count_a, n, "original storage debe contener exactamente N eventos");
    assert_eq!(fixture_a.len(), count_a);

    // --- When: reconstruir un Storage nuevo desde el fixture y re-exportar ---
    let rebuilt = import_fixture(&dir, "rebuilt.sqlite", &fixture_a);
    let (fixture_b, count_b, digest_b) = export_fixture(&rebuilt);
    drop(rebuilt);

    // --- Then: idempotencia ---
    assert_eq!(count_b, count_a, "count debe coincidir tras re-import");
    assert_eq!(digest_b, digest_a, "digest debe coincidir tras re-import (idempotente)");
    assert_eq!(fixture_a, fixture_b, "fixtures deben ser idénticos campo a campo");

    // --- Restart: reabrir el storage reconstruido desde la misma ruta ---
    let reopened = Storage::open(dir.path().join("rebuilt.sqlite")).unwrap();
    let (fixture_c, count_c, digest_c) = export_fixture(&reopened);
    drop(reopened);
    assert_eq!(count_c, count_a, "count sobrevive al restart");
    assert_eq!(digest_c, digest_a, "digest sobrevive al restart");

    // --- Failure injection: re-import interrumpida a la mitad ---
    let half = n / 2;
    {
        // append_event hace commit por evento, así que la "interrupción" se
        // modela dejando la importación incompleta (solo la primera mitad)
        // y dropeando el storage sin completarla.
        let aborted = import_fixture(&dir, "aborted.sqlite", &fixture_a[..half]);
        let (_, partial_count, _) = export_fixture(&aborted);
        assert_eq!(partial_count, half, "import abortada debe dejar exactamente la mitad");
        drop(aborted); // drop del storage = interrupción sin completar la importación
    }

    // --- Reintento canónico desde el fixture (storage NUEVO, path fresco) ---
    let recovered = import_fixture(&dir, "recovered.sqlite", &fixture_a);
    let (fixture_d, count_d, digest_d) = export_fixture(&recovered);
    drop(recovered);

    assert_eq!(count_d, count_a, "recuperación debe restaurar el count completo");
    assert_eq!(digest_d, digest_a, "recuperación debe restaurar el digest exacto");
    assert_eq!(fixture_d, fixture_a, "estado final tras recovery == fixture original");

    // --- El storage original sigue intacto (solo lectura/exportación) ---
    let final_check = Storage::open(&original_path).unwrap();
    let (_, count_final, digest_final) = export_fixture(&final_check);
    assert_eq!(count_final, count_a);
    assert_eq!(digest_final, digest_a, "original no debe mutar por exportar");
}
