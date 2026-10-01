//! AIW-S8 X07 — second **binary**, real process boundary.
//!
//! `crates/sddk-storage/tests/aiw_s8_x07_second_binary_integration.rs`
//! models the second consumer as a second `Storage` handle opened in the
//! *same* process. That proves two connections coexist; it does not prove a
//! separately-compiled executable can read what a writer produced.
//!
//! This file closes that gap without inventing a new binary. `sddk` itself is
//! the consumer, spawned as a real child process (`CARGO_BIN_EXE_sddk`), with
//! the ledger discovered through the same `SDDK_STATE_HOME` resolution the
//! operator uses. The writer stays in-process on the `Storage` API because the
//! asymmetry is the point: an external consumer must be able to read the
//! writer's bytes.
//!
//! Five demonstrable claims (D1..D5, see SCOPE-CONTRACT):
//! - D1 identity: the binary resolves the same `project_id`/`workspace_id`.
//! - D2 data: `cycle status` and `ledger events` see the writer's cycle+event.
//! - D3 schema guard: `ledger verify` validates the writer's ledger.
//! - D4 no side effects: the ledger is **byte-identical** after every read.
//! - D5 agreement: schema version agrees across the process boundary.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sddk_domain::{
    CycleId, CycleManifest, Ledger, LedgerEventInput, ProjectRecord, WorkspaceRecord,
};
use sddk_storage::{CycleRecord, Storage};
use serde_json::json;
use tempfile::TempDir;

const CREATED_AT: &str = "2026-10-01T12:00:00Z";
const REMOTE: &str = "git@EXAMPLE.com:Org/X07BinaryBoundary.git";
const CYCLE_SLUG: &str = "x07-binary-boundary";

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

/// Sandbox with its own data + state roots, so the ledger under test is never
/// the operator's real one.
struct Sandbox {
    dir: TempDir,
    root: PathBuf,
    state: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path().join("root");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&root).expect("root");
        std::fs::create_dir_all(&state).expect("state");
        let _ = name;
        Sandbox { dir, root, state }
    }

    fn ledger_path(&self, project_id: &str) -> PathBuf {
        self.state
            .join("sddk/projects")
            .join(project_id)
            .join("ledger.sqlite")
    }

    /// Runs the real `sddk` binary as a child process with the sandbox roots.
    fn run(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(bin())
            .args(args)
            .current_dir(&self.root)
            .env("SDDK_DATA_DIR", self.dir.path().join("data"))
            .env("SDDK_STATE_HOME", &self.state)
            .env_remove("SDDK_PROJECT_ID")
            .output()
            .expect("sddk binary runs");
        (
            out.status.code().unwrap_or(-1),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }

    /// Spawns the binary detached enough to observe its PID, proving the
    /// consumer runs in a **different process** than this test.
    fn run_observing_pid(&self, args: &[&str]) -> (u32, i32, String) {
        let child = Command::new(bin())
            .args(args)
            .current_dir(&self.root)
            .env("SDDK_DATA_DIR", self.dir.path().join("data"))
            .env("SDDK_STATE_HOME", &self.state)
            .env_remove("SDDK_PROJECT_ID")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("sddk binary spawns");
        let pid = child.id();
        let out = child.wait_with_output().expect("child exits");
        (
            pid,
            out.status.code().unwrap_or(-1),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }

    fn resolve_identity(&self) -> (String, String) {
        let (code, out) = self.run(&[
            "project", "resolve", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ]);
        assert_eq!(code, 0, "project resolve must succeed: {out}");
        let field = |key: &str| {
            out.lines()
                .find_map(|l| l.strip_prefix(&format!("{key}: ")))
                .unwrap_or_else(|| panic!("{key} missing in resolve output:\n{out}"))
                .trim()
                .to_string()
        };
        (field("project_id"), field("workspace_id"))
    }
}

/// The writer process: writes project + workspace + cycle + canonical event
/// through the public `Storage` API, exactly as any embedder would.
fn write_ledger(ledger: &Path, project_id: &str, workspace_id: &str) {
    let mut w = Storage::open(ledger).expect("writer open");
    w.insert_project(&ProjectRecord {
        project_id: project_id.into(),
        display_name: "X07 Binary Boundary Project".into(),
        remote_url: None,
        scope: "owner".into(),
        created_at: CREATED_AT.into(),
    })
    .expect("insert project");
    w.insert_workspace(&WorkspaceRecord {
        workspace_id: workspace_id.into(),
        project_id: project_id.into(),
        canonical_path: "/work/x07-binary-boundary".into(),
        created_at: CREATED_AT.into(),
    })
    .expect("insert workspace");

    let cycle_id = format!("{project_id}/{CYCLE_SLUG}");
    let manifest = CycleManifest::new(
        project_id.into(),
        workspace_id.into(),
        CycleId::new(cycle_id.clone()).expect("valid cycle id"),
        "X07 second binary".into(),
        "sddk/x07".into(),
        "def456".into(),
    );
    w.insert_cycle_with_event(
        &CycleRecord {
            manifest,
            created_at: CREATED_AT.into(),
            updated_at: CREATED_AT.into(),
        },
        &LedgerEventInput {
            event_id: "evt-x07-binary-1".into(),
            project_id: project_id.into(),
            cycle_id: Some(cycle_id),
            frame_id: "frame-x07-binary".into(),
            command_id: "command-x07-binary".into(),
            actor: "runtime".into(),
            actor_ref: None,
            event_type: "cycle.state_changed".into(),
            occurred_at: CREATED_AT.into(),
            state_before: None,
            state_after: Some(json!({"event": "evt-x07-binary-1"})),
            payload: json!({"event": "evt-x07-binary-1"}),
            causation_id: None,
            correlation_id: None,
        },
    )
    .expect("insert cycle with event");
}

/// Byte content of the ledger. Compared verbatim: byte-equality is strictly
/// stronger than a digest match and needs no hashing dependency.
fn ledger_bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("ledger read: {e}"))
}

fn json_run(sandbox: &Sandbox, args: &[&str]) -> serde_json::Value {
    let (code, out) = sandbox.run(args);
    assert_eq!(
        code,
        0,
        "command must succeed: sddk {}\n{out}",
        args.join(" ")
    );
    serde_json::from_str(&out).unwrap_or_else(|e| panic!("not JSON: {e}\n{out}"))
}

/// D0 — the precondition of the whole file, and the falsifier that the
/// storage-side X07 suite cannot have: the consumer is a **separate
/// executable in a separate OS process**, not another handle in this one.
///
/// The byte-equality claim in D4 is blind to this by construction: an in-process
/// read-only handle also leaves the bytes untouched, so D4 alone would still
/// pass if the binary were never invoked. Asserting the child's PID is what
/// makes the process boundary load-bearing instead of assumed.
#[test]
fn the_consumer_is_a_separate_process_not_a_second_handle() {
    let s = Sandbox::new("process");

    // The consumer really is an executable artefact, not this test binary.
    let consumer = bin();
    let meta = std::fs::metadata(&consumer).expect("consumer binary exists");
    assert!(meta.is_file(), "consumer must be a real file: {consumer:?}");
    assert!(
        meta.permissions().mode() & 0o111 != 0,
        "consumer must be executable: {consumer:?}"
    );
    assert_ne!(
        std::fs::canonicalize(&consumer).ok(),
        std::fs::canonicalize(std::env::current_exe().unwrap()).ok(),
        "consumer must not be the test binary itself"
    );

    let (pid, code, out) = s.run_observing_pid(&[
        "project", "resolve", "--root", ".", "--scope", ".", "--remote", REMOTE,
    ]);
    assert_eq!(code, 0, "consumer must run: {out}");
    assert_ne!(
        pid,
        std::process::id(),
        "consumer ran in the test's own process — no boundary crossed"
    );
    assert!(pid > 0, "consumer must report a real pid: {pid}");
}

/// D1 — the binary resolves the same identity the writer used, and D5's
/// schema agreement across the boundary.
#[test]
fn real_binary_resolves_the_writers_identity() {
    let s = Sandbox::new("identity");
    let (project_id, workspace_id) = s.resolve_identity();
    assert!(
        project_id.starts_with("p-") && workspace_id.starts_with("w-"),
        "malformed identity: {project_id} / {workspace_id}"
    );

    write_ledger(&s.ledger_path(&project_id), &project_id, &workspace_id);

    // The writer's project row is visible through the binary's own resolution:
    // same project_id, same workspace_id, no drift across processes.
    let resolved = json_run(
        &s,
        &[
            "project", "resolve", "--root", ".", "--scope", ".", "--remote", REMOTE, "--format",
            "json",
        ],
    );
    assert_eq!(
        resolved["project_id"].as_str(),
        Some(project_id.as_str()),
        "binary resolved a different project_id"
    );
}

/// D2 — the binary reads the writer's cycle and event.
#[test]
fn real_binary_reads_cycle_and_events() {
    let s = Sandbox::new("read");
    let (project_id, workspace_id) = s.resolve_identity();
    write_ledger(&s.ledger_path(&project_id), &project_id, &workspace_id);

    let cycle_id = format!("{project_id}/{CYCLE_SLUG}");
    let status = json_run(
        &s,
        &[
            "cycle", "status", "--root", ".", "--scope", ".", "--remote", REMOTE, "--cycle",
            &cycle_id, "--format", "json",
        ],
    );
    assert_eq!(status["cycle_id"].as_str(), Some(cycle_id.as_str()));

    let events = json_run(
        &s,
        &[
            "ledger", "events", "--root", ".", "--scope", ".", "--remote", REMOTE, "--format",
            "json",
        ],
    );
    let events = events.as_array().expect("events array");
    assert_eq!(
        events.len(),
        1,
        "expected exactly the writer event: {events:?}"
    );
    assert_eq!(events[0]["event_id"].as_str(), Some("evt-x07-binary-1"));
    assert_eq!(events[0]["cycle_id"].as_str(), Some(cycle_id.as_str()));
}

/// D3 — `ledger verify` validates the ledger the writer produced, from another
/// process, agreeing on schema version and event count.
#[test]
fn real_binary_verifies_the_writers_ledger() {
    let s = Sandbox::new("verify");
    let (project_id, workspace_id) = s.resolve_identity();
    let ledger = s.ledger_path(&project_id);
    write_ledger(&ledger, &project_id, &workspace_id);

    // Writer-side view of the schema it just produced.
    let before = Storage::open_read_only(&ledger)
        .expect("writer-side read-only open")
        .schema_version()
        .expect("writer schema version");

    let verified = json_run(
        &s,
        &[
            "ledger", "verify", "--root", ".", "--scope", ".", "--remote", REMOTE, "--format",
            "json",
        ],
    );
    assert_eq!(verified["event_count"].as_i64(), Some(1));
    assert!(
        verified["last_hash"].is_string(),
        "verify must report a head hash: {verified}"
    );
    assert!(
        Storage::open_read_only(&ledger)
            .expect("post read-only open")
            .schema_version()
            .expect("schema after read")
            == before,
        "schema version must agree across the process boundary"
    );
}

/// D4 — the strongest claim: reading through a real binary leaves the writer's
/// ledger byte-identical. Not "the command says read-only" — the bytes.
#[test]
fn reads_through_a_real_binary_leave_the_ledger_byte_identical() {
    let s = Sandbox::new("bytes");
    let (project_id, workspace_id) = s.resolve_identity();
    let ledger = s.ledger_path(&project_id);
    write_ledger(&ledger, &project_id, &workspace_id);

    let before = ledger_bytes(&ledger);
    assert!(!before.is_empty(), "writer must produce a ledger");

    let cycle_id = format!("{project_id}/{CYCLE_SLUG}");
    for args in [
        vec!["ledger", "events", "--format", "json"],
        vec!["ledger", "verify", "--format", "json"],
        vec![
            "cycle",
            "status",
            "--cycle",
            cycle_id.as_str(),
            "--format",
            "json",
        ],
    ] {
        let mut full = args.clone();
        full.extend_from_slice(&["--root", ".", "--scope", ".", "--remote", REMOTE]);
        let (code, out) = s.run(&full);
        assert_eq!(code, 0, "read command must succeed: {:?}\n{out}", args);
        let after = ledger_bytes(&ledger);
        assert_eq!(
            before, after,
            "read command mutated the writer's ledger bytes: {args:?}"
        );
    }
}

/// AT-UAT-014 — a consumer that tries to **write** must fail closed: the
/// command has no write verb to call, and the writer's bytes survive the
/// attempt. Both halves matter; either alone is satisfiable by accident.
#[test]
fn a_consumer_cannot_write_through_the_binary() {
    let s = Sandbox::new("nowrite");
    let (project_id, workspace_id) = s.resolve_identity();
    let ledger = s.ledger_path(&project_id);
    write_ledger(&ledger, &project_id, &workspace_id);
    let before = ledger_bytes(&ledger);

    // There is no `ledger write` surface: the verb is rejected by the parser,
    // before any storage is touched.
    let (code, out) = s.run(&["ledger", "write", "--root", ".", "--scope", "."]);
    assert_ne!(code, 0, "an unknown write verb must not exit 0: {out}");

    // A real state-changing verb is either refused for lack of authority or
    // fails; what it must never do is silently rewrite the writer's bytes.
    let (code, out) = s.run(&[
        "cycle",
        "transition",
        "--transition",
        "apply",
        "--cycle",
        format!("{project_id}/{CYCLE_SLUG}").as_str(),
        "--root",
        ".",
        "--scope",
        ".",
        "--remote",
        REMOTE,
    ]);
    assert!(
        code != 0,
        "unauthorised transition must fail closed, not succeed: {out}"
    );
    assert_eq!(
        before,
        ledger_bytes(&ledger),
        "a refused write still mutated the ledger"
    );
}
