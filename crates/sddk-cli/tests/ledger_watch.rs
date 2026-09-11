//! E2E tests for `sddk ledger watch` (M9.5 live-mode streaming).
//!
//! The watch command polls `Storage::list_events_after` and emits one event
//! per line. These tests exercise the four behavioural contracts:
//!
//!   * empty ledger → idle timeout exits with zero events emitted
//!   * pre-existing events with `--from-tail` → ignored (not re-emitted)
//!   * new `append_event` while watch is running → emitted in NDJSON
//!   * `--max-events` cap → exits cleanly after N events
//!
//! Follows the same XDG-isolation pattern as `cli_approval_e2e.rs`.

use sddk_domain::LedgerEventInput;
use sddk_domain::ProjectRecord;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;

/// Computes the stable project ID for a fallback seed + scope.
fn fallback_project_id(seed: &str, scope: &str) -> String {
    let hex = {
        let mut hasher = Sha256::new();
        let domain = "sddk.project.fallback.v1";
        hasher.update((domain.len() as u64).to_be_bytes());
        hasher.update(domain.as_bytes());
        hasher.update((seed.len() as u64).to_be_bytes());
        hasher.update(seed.as_bytes());
        hasher.update((scope.len() as u64).to_be_bytes());
        hasher.update(scope.as_bytes());
        format!("{:x}", hasher.finalize())
    };
    format!("p-{}", &hex[..16])
}

/// Test environment holding XDG dirs and the project id.
struct WatchTestEnv {
    root: std::path::PathBuf,
    state: std::path::PathBuf,
    project_id: String,
    _dir: TempDir,
}

/// Builds a `(env, run)` pair identical in shape to `cli_approval_e2e.rs`.
fn watch_test_setup() -> (WatchTestEnv, impl Fn(&[&str]) -> std::process::Output) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let state = tmp.path().join("state");
    let data = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    let project_id = fallback_project_id("00000000-0000-0000-0000-000000000001", ".");

    // Pre-create the ledger directory so `SqliteEventStore::open` succeeds.
    let ledger_dir = state.join("sddk").join("projects").join(&project_id);
    std::fs::create_dir_all(&ledger_dir).unwrap();

    let home_c = home.clone();
    let data_c = data.clone();
    let state_c = state.clone();
    let cache_c = cache.clone();

    let run = move |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(args)
            .env("HOME", &home_c)
            .env("XDG_DATA_HOME", &data_c)
            .env("XDG_STATE_HOME", &state_c)
            .env("XDG_CACHE_HOME", &cache_c)
            .output()
            .unwrap()
    };

    let env = WatchTestEnv {
        root,
        state,
        project_id,
        _dir: tmp,
    };
    (env, run)
}

/// Opens the SQLite event store at the ledger path for the watch test env.
///
/// The CLI resolves `paths.ledger` to `<XDG_STATE_HOME>/sddk/projects/<pid>/ledger.sqlite`
/// (see `crates/sddk-engine/src/paths.rs:130`). Tests must open the same file
/// so writes via `Storage::append_event` are visible to `list_events_after`.
fn open_test_store(env: &WatchTestEnv) -> sddk_storage::Storage {
    let ledger_path = env
        .state
        .join("sddk")
        .join("projects")
        .join(&env.project_id)
        .join("ledger.sqlite");
    sddk_storage::Storage::open(&ledger_path).unwrap()
}

/// Builds a minimal `LedgerEventInput` matching the schema used in
/// `cli_approval_e2e.rs::make_event`.
///
/// `cycle_id` is intentionally `None` — `ledger_events` enforces a FK
/// `(project_id, cycle_id) REFERENCES cycles`, and we don't need to spin
/// up a cycle manifest for these streaming tests. The project_id FK is
/// satisfied by [`seed_project`].
fn make_input(
    project_id: &str,
    event_type: &str,
    payload: serde_json::Value,
) -> LedgerEventInput {
    LedgerEventInput {
        event_id: format!("e-{event_type}-{}", chrono_like_id()),
        project_id: project_id.to_string(),
        cycle_id: None,
        frame_id: "f-1".to_string(),
        command_id: "cmd-test".to_string(),
        actor: "test".to_string(),
        actor_ref: None,
        event_type: event_type.to_string(),
        occurred_at: "2026-09-11T10:00:00Z".to_string(),
        state_before: None,
        state_after: None,
        payload,
        causation_id: None,
        correlation_id: None,
    }
}

/// Inserts a project row into the ledger so the `ledger_events.project_id`
/// FK is satisfied when appending events directly.
fn seed_project(env: &WatchTestEnv) {
    let store = open_test_store(env);
    store
        .insert_project(&ProjectRecord {
            project_id: env.project_id.clone(),
            display_name: "watch-test".to_string(),
            remote_url: None,
            scope: ".".to_string(),
            created_at: "2026-09-11T10:00:00Z".to_string(),
        })
        .unwrap();
}

/// Returns a monotonically increasing pseudo-unique suffix so multiple
/// `make_input` calls within a single test produce distinct event_ids
/// without colliding on the storage primary key.
fn chrono_like_id() -> String {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    format!("{n:06x}")
}

/// Helper to invoke `sddk ledger watch` with the same XDG-isolation
/// settings used by the run closure.
fn invoke_watch(
    run: &impl Fn(&[&str]) -> std::process::Output,
    root: &std::path::Path,
    extra: &[&str],
) -> std::process::Output {
    let mut args = vec![
        "ledger",
        "watch",
        "--root",
        root.to_str().unwrap(),
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ];
    args.extend_from_slice(extra);
    run(&args)
}

// ── Tests ────────────────────────────────────────────────────────────────────────

/// Empty ledger + `--idle-timeout-ms` → exits with `emitted: 0`.
#[test]
fn watch_exits_on_idle_timeout_when_ledger_empty() {
    let (env, run) = watch_test_setup();

    let out = invoke_watch(
        &run,
        &env.root,
        &[
            "--idle-timeout-ms",
            "100",
            "--interval-ms",
            "20",
            "--max-events",
            "0",
            "--format",
            "json",
        ],
    );

    assert_eq!(
        out.status.code(),
        Some(0),
        "watch should exit cleanly: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(r#""__watch_complete":true"#),
        "expected watch completion marker, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""emitted":0"#),
        "expected zero events emitted, got: {stdout}"
    );
}

/// Pre-existing events with `--from-tail` → not re-emitted; idle timeout
/// exits with `emitted: 0` even though the ledger has historical events.
#[test]
fn watch_from_tail_ignores_pre_existing_events() {
    let (env, run) = watch_test_setup();

    // Insert project + two historical events directly via Storage.
    seed_project(&env);
    {
        let mut store = open_test_store(&env);
        store
            .append_event(&make_input(
                &env.project_id,
                "historical.event.1",
                json!({"k": "v1"}),
            ))
            .unwrap();
        store
            .append_event(&make_input(
                &env.project_id,
                "historical.event.2",
                json!({"k": "v2"}),
            ))
            .unwrap();
    }

    let out = invoke_watch(
        &run,
        &env.root,
        &[
            "--from-tail",
            "--idle-timeout-ms",
            "100",
            "--interval-ms",
            "20",
            "--max-events",
            "0",
            "--format",
            "json",
        ],
    );

    assert_eq!(
        out.status.code(),
        Some(0),
        "watch should exit cleanly: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(r#""emitted":0"#),
        "expected 0 events emitted (--from-tail), got: {stdout}"
    );
    assert!(
        !stdout.contains("historical.event.1"),
        "historical events should not be re-emitted with --from-tail, got: {stdout}"
    );
}

/// Append events while watch is running → new events show up in NDJSON
/// stream. Test runs watch in background, appends after a short delay,
/// and lets `--max-events` trigger a clean exit.
#[test]
fn watch_emits_newly_appended_events() {
    let (env, run) = watch_test_setup();

    // Use a short poll interval and an explicit `--max-events` cap so the
    // process exits as soon as the events arrive (no need for kill).
    let root_for_spawn = env.root.clone();
    let home_c = env._dir.path().join("home");
    let data_c = env._dir.path().join("data");
    let state_c = env.state.clone();
    let cache_c = env._dir.path().join("cache");

    // Pre-warm the SQLite schema BEFORE spawning the CLI. Otherwise both
    // processes race on `run_migrations` (each checks `user_version < N`
    // outside its transaction) and both try to CREATE TABLE gate_receipts.
    seed_project(&env);

    let mut child = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "ledger",
            "watch",
            "--root",
            root_for_spawn.to_str().unwrap(),
            "--scope",
            ".",
            "--fallback-seed",
            "00000000-0000-0000-0000-000000000001",
            "--interval-ms",
            "20",
            "--max-events",
            "2",
            "--format",
            "json",
        ])
        .env("HOME", &home_c)
        .env("XDG_DATA_HOME", &data_c)
        .env("XDG_STATE_HOME", &state_c)
        .env("XDG_CACHE_HOME", &cache_c)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn watch");

    // Small delay so the watch loop starts polling before we append.
    std::thread::sleep(Duration::from_millis(50));

    // Append two live events while watch is polling. The project row
    // was already inserted above to satisfy the FK.
    {
        let mut store = open_test_store(&env);
        store
            .append_event(&make_input(
                &env.project_id,
                "live.event.1",
                json!({"k": "v1"}),
            ))
            .unwrap();
        store
            .append_event(&make_input(
                &env.project_id,
                "live.event.2",
                json!({"k": "v2"}),
            ))
            .unwrap();
    }

    let out = child.wait_with_output().expect("watch output");
    assert_eq!(
        out.status.code(),
        Some(0),
        "watch should exit cleanly: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("live.event.1"),
        "expected live.event.1 in NDJSON, got: {stdout}"
    );
    assert!(
        stdout.contains("live.event.2"),
        "expected live.event.2 in NDJSON, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""emitted":2"#),
        "expected emitted=2 footer, got: {stdout}"
    );
}

/// `--max-events 1` exits after the first event, even if more exist.
#[test]
fn watch_max_events_caps_emission() {
    let (env, run) = watch_test_setup();

    // Pre-append 5 events. Watch with `--max-events 1` should emit only
    // the first and exit.
    seed_project(&env);
    {
        let mut store = open_test_store(&env);
        for i in 0..5 {
            store
                .append_event(&make_input(
                    &env.project_id,
                    &format!("cap.event.{i}"),
                    json!({"i": i}),
                ))
                .unwrap();
        }
    }

    let out = invoke_watch(
        &run,
        &env.root,
        &[
            "--max-events",
            "1",
            "--interval-ms",
            "20",
            "--format",
            "json",
        ],
    );

    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("cap.event.0"),
        "expected first event, got: {stdout}"
    );
    assert!(
        !stdout.contains("cap.event.1"),
        "should not have emitted cap.event.1, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""emitted":1"#),
        "expected emitted=1 footer, got: {stdout}"
    );
}
