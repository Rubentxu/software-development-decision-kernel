//! CLI tests for `sddk plan roadmap` subcommands (AC-PLN4-12).
//!
//! Tests the five roadmap projection subcommands:
//! - `sddk plan roadmap status`
//! - `sddk plan roadmap next`
//! - `sddk plan roadmap blocked`
//! - `sddk plan roadmap show --id <id>`
//! - `sddk plan roadmap graph [--format json|dot|mermaid]`
//!
//! Functional tests spawn a subprocess with a custom CWD inside a temp dir
//! that also contains `.sddk/adoption.json`, so the CLI's walk-up
//! adoption lookup finds it immediately.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

// ── Test fixtures ──────────────────────────────────────────────────────────────

/// Path to the pinned spine fixture.
const FIXTURE_PATH: &str = "../sddk-domain/tests/fixtures/execution_spine_post_reconciliation.yaml";

/// Load the pinned spine fixture as bytes.
fn load_fixture() -> Vec<u8> {
    fs::read(FIXTURE_PATH).expect("failed to read spine fixture")
}

/// Build an adopted storage tree and return (TempDir, sddk binary path).
/// The tree looks like:
///   $tmp/.sddk/adoption.json        ← found by walk-up from cwd
///   $tmp/sddk/projects/p-test/ledger.sqlite  ← found via XDG_STATE_HOME
fn build_adopted_storage() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("temp dir");
    let tmp_path = tmp.path();

    // Create adoption receipt at tmp root (CLI walks up from cwd to find this)
    let sddk_dir = tmp_path.join(".sddk");
    fs::create_dir_all(&sddk_dir).expect("create .sddk dir");
    fs::write(
        sddk_dir.join("adoption.json"),
        serde_json::json!({
            "project_id": "p-test",
            "workspace_id": "ws-test"
        })
        .to_string(),
    )
    .expect("write adoption.json");

    // Create ledger directory and import spine
    let ledger_dir = tmp_path.join("sddk/projects/p-test/");
    fs::create_dir_all(&ledger_dir).expect("create ledger dir");
    let db_path = ledger_dir.join("ledger.sqlite");
    let mut storage = sddk_storage::Storage::open(&db_path).expect("open storage");
    let spine_bytes = load_fixture();
    sddk_storage::spine_import::import_spine(&spine_bytes, &mut storage)
        .expect("import spine fixture");

    // Find the sddk binary
    let bin_path = env!("CARGO_BIN_EXE_sddk");

    (tmp, bin_path.into())
}

/// Run `sddk` binary as a subprocess with cwd=tmp_path and XDG_STATE_HOME=tmp_path.
fn run_sddk(
    tmp_path: &std::path::Path,
    sddk_bin: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(sddk_bin)
        .args(args)
        .current_dir(tmp_path)
        .env("XDG_STATE_HOME", tmp_path.to_str().unwrap())
        .output()
        .expect("failed to spawn sddk")
}

// ── AC-PLN4-12: CLI dispatch tests ─────────────────────────────────────────────

/// Dispatch: `sddk plan roadmap --help` exits 0.
#[test]
fn roadmap_help_exits_zero() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "--help"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Dispatch: `sddk plan roadmap status --help` exits 0.
#[test]
fn roadmap_status_help_exits_zero() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "status", "--help"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Dispatch: `sddk plan roadmap next --help` exits 0.
#[test]
fn roadmap_next_help_exits_zero() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "next", "--help"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Dispatch: `sddk plan roadmap blocked --help` exits 0.
#[test]
fn roadmap_blocked_help_exits_zero() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "blocked", "--help"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Dispatch: `sddk plan roadmap show --help` exits 0.
#[test]
fn roadmap_show_help_exits_zero() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "show", "--help"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Dispatch: `sddk plan roadmap graph --help` exits 0.
#[test]
fn roadmap_graph_help_exits_zero() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "graph", "--help"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── AC-PLN4-12: functional tests ───────────────────────────────────────────────

/// Functional: `sddk plan roadmap status` exits 0 and emits JSON to stdout.
#[test]
fn roadmap_status_functional() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "status"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "roadmap status must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("per_horizon") || stdout.contains("active_item"),
        "stdout must be JSON with projection fields: {}",
        stdout
    );
}

/// Functional: `sddk plan roadmap next` exits 0 and emits JSON to stdout.
#[test]
fn roadmap_next_functional() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "next"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "roadmap next must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_ok(),
        "stdout must be valid JSON: {}",
        stdout
    );
}

/// Functional: `sddk plan roadmap graph` exits 0 and emits JSON graph.
#[test]
fn roadmap_graph_default_functional() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(tmp.path(), &bin, &["plan", "roadmap", "graph"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "roadmap graph must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_ok(),
        "stdout must be valid JSON graph: {}",
        stdout
    );
}

/// Functional: `sddk plan roadmap show --id` with a known item exits 0.
#[test]
fn roadmap_show_functional() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(
        tmp.path(),
        &bin,
        &["plan", "roadmap", "show", "--id", "TEST-LEDGER-A"],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "roadmap show --id must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_ok(),
        "stdout must be valid JSON: {}",
        stdout
    );
}

/// Functional: `sddk plan roadmap show --id` with unknown item fails gracefully.
#[test]
fn roadmap_show_unknown_id_fails() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(
        tmp.path(),
        &bin,
        &["plan", "roadmap", "show", "--id", "DOES-NOT-EXIST"],
    );

    assert_ne!(
        output.status.code(),
        Some(0),
        "show with unknown id must fail"
    );
}

/// Functional: `sddk plan roadmap graph --format dot` exits 0.
#[test]
fn roadmap_graph_dot_format_functional() {
    let (tmp, bin) = build_adopted_storage();
    let output = run_sddk(
        tmp.path(),
        &bin,
        &["plan", "roadmap", "graph", "--format", "dot"],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "roadmap graph --format dot must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
