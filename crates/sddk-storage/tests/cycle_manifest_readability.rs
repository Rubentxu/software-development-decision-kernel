//! Characterization test for D2 (INC-DEBT-060): `get_cycle` cannot read a cycle
//! whose `manifest_json` is not a full [`CycleManifest`].
//!
//! This test **passes today**. That is the point. D2 is a real defect — 81 of
//! the 179 cycles in `p-63676b11dc0ef88f` have a `manifest_json` that does not
//! deserialize, and `get_cycle` returns an *error* for them instead of a
//! record — but fixing it means changing what `get_cycle` returns for those
//! rows, which is a contract decision reserved to a separate SCOPE
//! (STOP 1 in the SCOPE-CONTRACT of `cl-cycle-enumeration`).
//!
//! Pinning the current behaviour *before* the enumeration work lands is what
//! turns a later change into a decision instead of a drift. Without this test,
//! `get_cycle` could quietly start returning a partial record for a `{}` and
//! nobody would notice, because nothing was asserting either way.
//!
//! It also guards against a vacuous pass: a row with a well-formed manifest has
//! to read back fine through the same call, so a failure here means the
//! deserialization path is broken, not that the fixture is malformed.

use rusqlite::Connection;
use sddk_domain::cycle::{CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_domain::identity::CycleId;
use sddk_storage::Storage;
use tempfile::TempDir;

/// Opens a fresh ledger through `Storage`, which runs the migrations, then
/// seeds the `projects` / `workspaces` / `cycles` rows the composite foreign
/// key requires. The manifest is passed through verbatim so a test can plant
/// whatever shape it is about to assert on.
fn seeded_ledger(cycle_id: &str, manifest_json: &str) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let _ = Storage::open(&db_path).expect("fresh ledger opens");

    let conn = Connection::open(&db_path).unwrap();
    conn.execute(
        "INSERT INTO projects (project_id, display_name, remote_url, scope, created_at)
         VALUES ('p-test', 'test', 'https://example.test/x', '.', '2026-01-01')",
        [],
    )
    .expect("project row");
    conn.execute(
        "INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
         VALUES ('w-test', 'p-test', '/tmp/w-test', '2026-01-01')",
        [],
    )
    .expect("workspace row");
    conn.execute(
        "INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
         VALUES (?1, 'p-test', 'w-test', 'OPEN', 'explore', ?2, '2026-01-01', '2026-01-01')",
        rusqlite::params![cycle_id, manifest_json],
    )
    .expect("cycle row");
    drop(conn);

    (tmp, db_path)
}

/// A manifest built through the domain constructor rather than hand-written
/// JSON. Hand-writing would let a typo in a field name masquerade as a
/// production defect; the constructor cannot be wrong about its own shape.
fn valid_manifest_json() -> String {
    let id = CycleId::new("p-test/c-ok").expect("valid cycle id");
    let manifest = CycleManifest::new(
        "p-test".to_string(),
        "w-test".to_string(),
        id,
        "a valid cycle".to_string(),
        "main".to_string(),
        "0".repeat(40),
    );
    serde_json::to_string(&manifest).expect("manifest serializes")
}

/// Pre-condition, and the reason this file is not a tautology: the same call on
/// a well-formed row returns the record. If this fails, the test below proves
/// nothing about empty manifests — it would only prove that `get_cycle` is
/// broken for *every* cycle, which is a different defect entirely.
#[test]
fn a_well_formed_manifest_reads_back_through_get_cycle() {
    let (_tmp, db) = seeded_ledger("p-test/c-ok", &valid_manifest_json());
    let storage = Storage::open(&db).unwrap();

    let record = storage
        .get_cycle("p-test/c-ok")
        .expect("a complete manifest must be readable");

    assert_eq!(record.manifest.status, CycleStatus::Open);
    assert_eq!(record.manifest.phase, Phase::Explore);
    assert_eq!(record.manifest.path, CyclePath::AFull);
    assert_eq!(record.created_at, "2026-01-01");
}

/// D2, pinned. An empty `manifest_json` is what 79 of the 81 unreadable rows in
/// the real storage actually contain, so this is the exact shape, not a
/// caricature of it.
///
/// Today `get_cycle` returns `Err` naming the first missing field. The SCOPE
/// forbids changing that inside this cycle.
#[test]
fn get_cycle_rejects_an_empty_manifest_and_names_the_missing_field() {
    let (_tmp, db) = seeded_ledger("p-test/c-empty", "{}");
    let storage = Storage::open(&db).unwrap();

    let err = storage
        .get_cycle("p-test/c-empty")
        .expect_err("an empty manifest must not read back as a cycle record");

    let rendered = format!("{err}");
    assert!(
        rendered.contains("schema_version"),
        "the error must name the field that could not be deserialized, so a \
         reader can tell a malformed manifest from a missing row or a broken \
         table; got: {rendered}"
    );
}

/// The other unreadable shape in the real storage: two cycles whose manifest is
/// a closure note rather than a manifest. Written as its own test because it is
/// a different failure — the row exists, the cycle was closed deliberately, and
/// only the payload is a note — and because a future reader who "fixes" the
/// `{}` case by making the whole manifest optional would break this one
/// differently.
#[test]
fn get_cycle_rejects_a_closure_note_manifest() {
    let note = r#"{"reason":"cycle_completed_external_release_pipeline","notes":"docs-only"}"#;
    let (_tmp, db) = seeded_ledger("p-test/c-note", note);
    let storage = Storage::open(&db).unwrap();

    let err = storage
        .get_cycle("p-test/c-note")
        .expect_err("a closure note is not a CycleManifest");

    let rendered = format!("{err}");
    assert!(
        rendered.contains("schema_version"),
        "same contract as the empty manifest: the error must name the missing \
         field; got: {rendered}"
    );
}
