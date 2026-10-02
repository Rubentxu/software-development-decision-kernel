//! R3 — enumeration must read `cycles`, not the event log.
//!
//! INC-DEBT-060, D1. This is the test that could not be written in lote 1:
//! calling `list_cycles` when it does not exist is a **compile error**, not a
//! red assertion, and a red assertion bought by breaking the build takes the
//! whole crate down with it. It becomes writable the moment the function lands.
//!
//! The defect it guards is specific and easy to reintroduce by accident. An
//! enumeration built over `events_v1` — which is the tempting source, because
//! the event store already exposes `list_streams` and `verify_stream_chain`
//! there — would *look* correct and would hide exactly the rows this debt is
//! about. A cycle that emitted no event has nothing to enumerate from, and
//! **having no events is why those 97 cycles are invisible in the first
//! place**. Building the fix on the same substrate reproduces the bug while
//! looking like the fix.
//!
//! So the fixture here plants cycles that emit nothing at all.

use rusqlite::Connection;
use sddk_domain::cycle::CycleManifest;
use sddk_domain::identity::CycleId;
use sddk_storage::Storage;
use tempfile::TempDir;

struct Fixture {
    _tmp: TempDir,
    db: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("ledger.sqlite");
        let _ = Storage::open(&db).expect("fresh ledger opens");
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "INSERT INTO projects (project_id, display_name, remote_url, scope, created_at)
             VALUES ('p-test', 'test', 'https://example.test/x', '.', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
             VALUES ('w-test', 'p-test', '/tmp/w-test', '2026-01-01')",
            [],
        )
        .unwrap();
        drop(conn);
        Fixture { _tmp: tmp, db }
    }

    /// Inserts a `cycles` row directly, which is the only way to make a cycle
    /// that emitted no event: every write path the product has goes through the
    /// ledger and leaves a `cycle.created` behind.
    fn insert_cycle(&self, cycle_id: &str, status: &str, manifest_json: &str, created_at: &str) {
        let conn = Connection::open(&self.db).unwrap();
        conn.execute(
            "INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
             VALUES (?1, 'p-test', 'w-test', ?2, 'explore', ?3, ?4, ?4)",
            rusqlite::params![cycle_id, status, manifest_json, created_at],
        )
        .unwrap();
        drop(conn);
    }

    fn insert_valid_cycle(&self, cycle_id: &str, status: &str, created_at: &str) {
        let id = CycleId::new(cycle_id).expect("valid cycle id");
        let manifest = CycleManifest::new(
            "p-test".to_string(),
            "w-test".to_string(),
            id,
            "a cycle".to_string(),
            "main".to_string(),
            "0".repeat(40),
        );
        self.insert_cycle(
            cycle_id,
            status,
            &serde_json::to_string(&manifest).unwrap(),
            created_at,
        );
    }

    fn list(&self) -> Vec<sddk_storage::CycleSummary> {
        let storage = Storage::open(&self.db).unwrap();
        storage.list_cycles("p-test").expect("enumeration succeeds")
    }

    /// Zero events on purpose: nothing here ever writes to `events_v1`.
    fn event_count(&self) -> i64 {
        let conn = Connection::open(&self.db).unwrap();
        conn.query_row("SELECT COUNT(*) FROM events_v1", [], |r| r.get(0))
            .unwrap()
    }
}

/// The one that matters. Two cycles, **neither of which has ever emitted an
/// event**, and enumeration returns both.
#[test]
fn a_cycle_with_no_events_still_appears() {
    let f = Fixture::new();
    f.insert_valid_cycle("p-test/silent-one", "OPEN", "2026-01-01");
    f.insert_valid_cycle("p-test/silent-two", "OPEN", "2026-01-02");

    // The precondition, so this test cannot pass for the wrong reason: if the
    // fixture ever starts emitting events, it is no longer testing anything.
    assert_eq!(f.event_count(), 0, "fixture must emit no events at all");

    let listed = f.list();
    let ids: Vec<&str> = listed.iter().map(|c| c.cycle_id.as_str()).collect();

    assert_eq!(
        ids.len(),
        2,
        "both eventless cycles must be enumerated; got {ids:?}"
    );
    assert!(ids.contains(&"p-test/silent-one"), "{ids:?}");
    assert!(ids.contains(&"p-test/silent-two"), "{ids:?}");
}

/// R2 at the storage level: a cycle that is not the current one still appears.
/// The `CLOSED` row is the shape that `cycle status` cannot reach, because that
/// command resolves through a live lease.
#[test]
fn enumeration_is_not_filtered_by_status() {
    let f = Fixture::new();
    f.insert_valid_cycle("p-test/open-one", "OPEN", "2026-01-01");
    f.insert_valid_cycle("p-test/closed-one", "CLOSED", "2026-01-02");
    f.insert_valid_cycle("p-test/paused-one", "PAUSED", "2026-01-03");

    let listed = f.list();
    assert_eq!(listed.len(), 3, "every status is enumerated: {listed:?}");
    let statuses: Vec<&str> = listed.iter().map(|c| c.status.as_str()).collect();
    assert!(
        statuses.contains(&"OPEN") && statuses.contains(&"CLOSED") && statuses.contains(&"PAUSED"),
        "all three statuses survive the enumeration, none is filtered away: {statuses:?}"
    );

    // `None` is the "no filter" case and it must mean *every* status. It is
    // asserted here rather than only in `filtering_narrows_the_listing_without_
    // changing_it` because a default that quietly meant "the interesting ones"
    // would hide the 91 `OPEN` rows that carry this debt — which is the defect
    // itself, reappearing as a default.
    let storage = Storage::open(&f.db).unwrap();
    let unfiltered = storage.list_cycles_by_status("p-test", None).unwrap();
    assert_eq!(
        unfiltered.len(),
        3,
        "an absent filter means every status, not a curated subset: {unfiltered:?}"
    );
}

/// A row whose manifest cannot be deserialized is **still listed**, flagged.
/// 81 of the 179 rows in the real storage are in this shape; dropping them
/// would swap "invisible" for "silently omitted", which is worse because the
/// count would stop adding up and nothing would say why.
#[test]
fn an_unreadable_manifest_is_listed_and_flagged_not_dropped() {
    let f = Fixture::new();
    f.insert_cycle(
        "p-test/good",
        "OPEN",
        &valid_manifest("p-test/good"),
        "2026-01-01",
    );
    f.insert_cycle("p-test/empty", "OPEN", "{}", "2026-01-02");
    f.insert_cycle(
        "p-test/note",
        "CLOSED",
        r#"{"reason":"cycle_completed_external_release_pipeline","notes":"docs-only"}"#,
        "2026-01-03",
    );

    let listed = f.list();
    assert_eq!(listed.len(), 3, "no row is dropped: {listed:?}");

    let flagged: Vec<&str> = listed
        .iter()
        .filter(|c| !c.manifest_readable)
        .map(|c| c.cycle_id.as_str())
        .collect();
    assert_eq!(
        flagged.len(),
        2,
        "exactly the two malformed manifests are flagged, and only those: {listed:?}"
    );
    assert!(flagged.contains(&"p-test/empty"), "{flagged:?}");
    assert!(flagged.contains(&"p-test/note"), "{flagged:?}");
    assert!(
        listed
            .iter()
            .find(|c| c.cycle_id == "p-test/good")
            .unwrap()
            .manifest_readable,
        "a complete manifest is readable: {listed:?}"
    );
}

/// Status carried as stored, not dropped when this binary does not recognise
/// the value. `MIGRATION_21` exists because the stored set and the domain enum
/// could disagree; a row must survive the enumeration anyway.
#[test]
fn an_unknown_status_is_carried_through_not_swallowed() {
    let f = Fixture::new();
    // `status` has a CHECK constraint, so an unknown value cannot be planted
    // directly. `RELEASED` is a real value this test asserts is carried
    // verbatim rather than normalised or dropped.
    f.insert_valid_cycle("p-test/released", "RELEASED", "2026-01-01");

    let listed = f.list();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(
        listed[0].status, "RELEASED",
        "status is carried exactly as stored: {listed:?}"
    );
}

/// The filtered form narrows and does not replace: `None` is every status.
#[test]
fn filtering_narrows_the_listing_without_changing_it() {
    let f = Fixture::new();
    f.insert_valid_cycle("p-test/a", "OPEN", "2026-01-01");
    f.insert_valid_cycle("p-test/b", "CLOSED", "2026-01-02");

    let storage = Storage::open(&f.db).unwrap();
    assert_eq!(
        storage.list_cycles_by_status("p-test", None).unwrap().len(),
        2,
        "an absent filter means every status"
    );
    let closed = storage
        .list_cycles_by_status("p-test", Some("CLOSED"))
        .unwrap();
    assert_eq!(closed.len(), 1, "the filter narrows");
    assert_eq!(closed[0].cycle_id, "p-test/b", "and picks the right one");
}

/// Enumeration is scoped to one project. A ledger holding several projects must
/// not answer for another one's cycles.
#[test]
fn enumeration_is_scoped_to_its_project() {
    let f = Fixture::new();
    f.insert_valid_cycle("p-test/mine", "OPEN", "2026-01-01");

    let storage = Storage::open(&f.db).unwrap();
    assert_eq!(storage.list_cycles("p-test").unwrap().len(), 1);
    assert_eq!(
        storage.list_cycles("p-other").unwrap().len(),
        0,
        "another project's cycles are not this project's"
    );
}

fn valid_manifest(cycle_id: &str) -> String {
    let manifest = CycleManifest::new(
        "p-test".to_string(),
        "w-test".to_string(),
        CycleId::new(cycle_id).unwrap(),
        "a cycle".to_string(),
        "main".to_string(),
        "0".repeat(40),
    );
    serde_json::to_string(&manifest).unwrap()
}
