//! ATOM-PER-ROW write sites must survive a competing writer that holds the
//! SQLite write lock for longer than the connection's `busy_timeout`.
//!
//! Contract: `crates/sddk-storage/src/planning_substrate_contract.md` claims
//! for these sites that "concurrent inserts with distinct ids each succeed".
//! Session-30 found the implementation did not honour that claim: the 5s
//! `busy_timeout` waits, then genuinely expires, and the raw `DatabaseBusy`
//! propagated. Under the full workspace suite this failed the release gate
//! (step 1/14) as `concurrency_planning_substrate`.
//!
//! These tests force the contention that the existing suite only produces
//! incidentally, so the guarantee is pinned rather than hoped for. Provenance
//! of the harness, both learned the hard way:
//!
//! 1. The competing writer must perform a real write. An `UPDATE` matching no
//!    row reports 0 rows and never takes the write lock, which made an earlier
//!    version of this file silently vacuous.
//! 2. `HOLD_MS` must EXCEED `busy_timeout`, or the subject just waits inside
//!    SQLite and no `DatabaseBusy` is ever raised.
//!
//! Mutation evidence (session-30): with the retry budget forced to zero both
//! tests fail with `database is locked`; restoring it turns them green.

use sddk_domain::planning::{DecisionKind, DecisionRecordRecord, WorkItemRecord, WorkItemStatus};
use sddk_storage::{CycleRecord, ProjectRecord, Storage, WorkspaceRecord};

const CREATED_AT: i64 = 1_725_836_000;

/// How long the competing writer holds the write lock.
///
/// Must EXCEED the connection's 5s `busy_timeout`, otherwise the subject
/// insert simply waits inside SQLite and no `DatabaseBusy` is ever raised —
/// which makes these tests vacuous rather than red.
const HOLD_MS: u64 = 7_000;

/// Holds a write transaction open for `hold_ms`, keeping SQLite's write lock
/// busy the whole time, then releases it.
///
/// The INSERT is load-bearing: an `UPDATE` that matches no row reports
/// `rows affected = 0` and never takes the write lock, which made an earlier
/// version of this harness silently vacuous. Asserting the row count is what
/// proves the lock is genuinely held.
fn hold_write_lock(db_path: &std::path::Path, hold_ms: u64) -> std::thread::JoinHandle<()> {
    let path = db_path.to_path_buf();
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        let n = conn
            .execute(
                "INSERT INTO decision_records_v1 (
                    id, work_item_id, kind, rationale,
                    actor_ref_kind, actor_ref_id, actor_ref_label, schema_version
                 ) VALUES ('__lock__', 'wi-lock-probe', '\"Accept\"', 'lock holder',
                    'Agent', 'agent:test', 'test', 1)",
                [],
            )
            .unwrap_or(0);
        assert_eq!(n, 1, "holder must really write, else the lock is not held");
        println!("MUTATION-PROBE holder holds write lock (rows={n})");
        std::thread::sleep(std::time::Duration::from_millis(hold_ms));
        let _ = conn.execute_batch("ROLLBACK");
    })
}

fn seed(db_path: &std::path::Path) -> String {
    use sddk_domain::{CycleId, CycleManifest};

    let storage = Storage::open(db_path).unwrap();
    let project_id = "p-lock-probe";
    let workspace_id = "ws-lock-probe";
    let cycle_id = "p-lock-probe/lock-probe";

    storage
        .insert_project(&ProjectRecord {
            project_id: project_id.into(),
            display_name: "Lock Probe".into(),
            remote_url: Some("https://example.com/lock".into()),
            scope: "test".into(),
            created_at: CREATED_AT.to_string(),
        })
        .unwrap();
    storage
        .insert_workspace(&WorkspaceRecord {
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            canonical_path: "/test".into(),
            created_at: CREATED_AT.to_string(),
        })
        .unwrap();
    let manifest = CycleManifest::new(
        project_id.into(),
        workspace_id.into(),
        CycleId::new(cycle_id).expect("valid cycle id"),
        "Lock probe cycle".into(),
        "sddk/lock-probe".into(),
        "abc123def456abc123def456abc123def456abc123def456abc123def456abc1".into(),
    );
    storage
        .insert_cycle(&CycleRecord {
            manifest,
            created_at: CREATED_AT.to_string(),
            updated_at: CREATED_AT.to_string(),
        })
        .unwrap();
    storage
        .insert_work_item(&WorkItemRecord {
            id: "wi-lock-probe".into(),
            cycle_id: cycle_id.into(),
            title: "lock probe".into(),
            description: "lock probe".into(),
            status: WorkItemStatus::Draft,
            actor_ref_kind: Some("Agent".into()),
            actor_ref_id: Some("agent:test".into()),
            actor_ref_label: Some("test".into()),
            created_at: CREATED_AT,
            schema_version: 1,
            spine_order: None,
            spine_horizon: None,
            spine_status: None,
            exit_gate: None,
        })
        .unwrap();
    cycle_id.to_string()
}

fn decision(id: &str) -> DecisionRecordRecord {
    DecisionRecordRecord {
        id: id.into(),
        work_item_id: "wi-lock-probe".into(),
        kind: DecisionKind::Accept,
        rationale: format!("locked decision {id}"),
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        schema_version: 1,
    }
}

/// The mutation under test: an insert attempted while a competing writer holds
/// the write lock must still succeed, because `execute_with_busy_retry`
/// retries past the DatabaseBusy.
#[test]
fn decision_insert_survives_competing_write_lock() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ledger.sqlite");
    seed(&db_path);

    // The holder sleeps HOLD_MS (> the 5s busy_timeout), so the subject's
    // first attempt is guaranteed to burn its whole SQLite budget and return
    // DatabaseBusy. Only the retry budget can carry it to success.
    let holder = hold_write_lock(&db_path, HOLD_MS);

    // Give the holder time to actually take the lock.
    std::thread::sleep(std::time::Duration::from_millis(200));

    let storage = Storage::open(&db_path).unwrap();
    let result = storage.insert_decision_record(&decision("dec-under-lock"));

    holder.join().unwrap();

    match &result {
        Ok(()) => println!("MUTATION-PROBE decision insert under lock: OK (retry worked)"),
        Err(e) => panic!("MUTATION-PROBE decision insert under lock FAILED: {e}"),
    }
}

/// Same for the work-item insert path, the other site the failing tests hit.
#[test]
fn work_item_insert_survives_competing_write_lock() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ledger.sqlite");
    let cycle_id = seed(&db_path);

    let holder = hold_write_lock(&db_path, HOLD_MS);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let storage = Storage::open(&db_path).unwrap();
    let result = storage.insert_work_item(&WorkItemRecord {
        id: "wi-under-lock".into(),
        cycle_id,
        title: "under lock".into(),
        description: "under lock".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: 2,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    });

    holder.join().unwrap();

    match &result {
        Ok(()) => println!("MUTATION-PROBE work item insert under lock: OK (retry worked)"),
        Err(e) => panic!("MUTATION-PROBE work item insert under lock FAILED: {e}"),
    }
}
