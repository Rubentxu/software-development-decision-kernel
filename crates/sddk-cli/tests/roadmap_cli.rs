//! CLI tests for `sddk plan roadmap` subcommands (AC-PLN4-12).
//!
//! Tests the five roadmap projection subcommands:
//! - `sddk plan roadmap status`
//! - `sddk plan roadmap next`
//! - `sddk plan roadmap blocked`
//! - `sddk plan roadmap show --id <id>`
//! - `sddk plan roadmap graph [--format json|dot|mermaid]`
//!
//! Functional tests use the canonical XDG adoption flow: `sddk adopt apply`
//! creates the receipt under XDG_DATA_HOME, and `sddk plan import --spine`
//! populates the ledger.

use std::process::Command;

use tempfile::TempDir;

// ── Test fixtures ──────────────────────────────────────────────────────────────

/// Builds an adopted storage tree using the canonical XDG adoption flow.
/// Returns (root_tempdir, sddk_binary_path).
/// The canonical layout (with shared tempdir for all XDG vars):
///   <tmp>/.sddk/                    ← git repo root
///   <tmp>/sddk/projects/<pid>/    ← XDG data + state dir
///   <tmp>/sddk/projects/<pid>/workspaces/<wid>/adoption.json
///   <tmp>/sddk/projects/<pid>/ledger.sqlite
fn build_adopted_storage() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("temp dir");
    let tmp_str = tmp.path().to_str().unwrap();

    // Create minimal git repo
    std::fs::create_dir_all(tmp.path().join(".git")).expect("create .git dir");

    // Run sddk adopt apply — using the same dir for root and all XDG vars
    let adopt = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .env("HOME", tmp_str)
        .env("XDG_DATA_HOME", tmp_str)
        .env("XDG_STATE_HOME", tmp_str)
        .env("XDG_CACHE_HOME", tmp_str)
        .env("USER", "test-cli-actor")
        .current_dir(tmp.path())
        .args([
            "adopt",
            "apply",
            "--root",
            tmp_str,
            "--scope",
            ".",
            "--timestamp",
            "2026-09-06T00:00:00Z",
            "--actor",
            "test",
            "--format",
            "json",
        ])
        .output()
        .expect("adopt apply");
    assert!(
        adopt.status.success(),
        "adopt apply failed: {}",
        String::from_utf8_lossy(&adopt.stderr)
    );

    // Import the spine fixture
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let spine_path = std::path::Path::new(manifest_dir)
        .join("..")
        .join("sddk-domain/tests/fixtures/execution_spine_post_reconciliation.yaml");
    let import = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .env("HOME", tmp_str)
        .env("XDG_DATA_HOME", tmp_str)
        .env("XDG_STATE_HOME", tmp_str)
        .env("XDG_CACHE_HOME", tmp_str)
        .env("USER", "test-cli-actor")
        .current_dir(tmp.path())
        .args(["plan", "import", "--spine", spine_path.to_str().unwrap()])
        .output()
        .expect("plan import");
    assert!(
        import.status.success(),
        "spine import failed: {}",
        String::from_utf8_lossy(&import.stderr)
    );

    (tmp, env!("CARGO_BIN_EXE_sddk").into())
}

/// Run `sddk` binary as a subprocess with all XDG vars pointing to tmp_path.
///
/// The binary needs XDG_DATA_HOME to locate the adoption receipt created by
/// `build_adopted_storage()`.  The old broken `open_storage_for_plan` walked
/// the filesystem from cwd (ignoring XDG), which accidentally worked in the
/// test because cwd happened to contain the receipt.  The canonical resolver
/// uses XDG paths, so all vars must be set consistently.
fn run_sddk(
    tmp_path: &std::path::Path,
    sddk_bin: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    let tmp_str = tmp_path.to_str().unwrap();
    Command::new(sddk_bin)
        .args(args)
        .current_dir(tmp_path)
        .env("HOME", tmp_str)
        .env("XDG_DATA_HOME", tmp_str)
        .env("XDG_STATE_HOME", tmp_str)
        .env("XDG_CACHE_HOME", tmp_str)
        .env("USER", "test-cli-actor")
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
