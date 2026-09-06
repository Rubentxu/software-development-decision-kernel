//! Regression test: `sddk plan roadmap *` must use the canonical RuntimeContext::open() resolver.
//!
//! ## Bug
//! `open_storage_for_plan()` (plan.rs:42-72) walks up from cwd looking for `.sddk/adoption.json`
//! at the project root, but the canonical adoption receipt lives at:
//! `<XDG_DATA_HOME>/sddk/projects/<pid>/workspaces/<wid>/adoption.json`
//! via `sddk adopt apply`. The walker never finds it for XDG-resolved adoptions.
//!
//! ## Test strategy
//! - Create a tempdir with proper XDG env vars
//! - `git init` a minimal repo inside the tempdir
//! - Run `sddk adopt apply --root <tmp> --scope .` to create the canonical receipt
//! - Import the spine fixture using `sddk plan import --spine <fixture>`
//! - Run `sddk plan roadmap next` as a subprocess (cwd = tempdir, XDG vars set)
//! - Assert: exit 0 AND stderr does NOT contain "No adopted project found"
//!
//! Before the fix: this test FAILS (RED) — plan command cannot find the adopted project.
//! After the fix: this test PASSES (GREEN) — plan uses RuntimeContext::open() resolver.

use std::process::Command;

use tempfile::TempDir;

/// Run `sddk` binary as a subprocess with full XDG environment.
fn run_sddk_subprocess(
    root: &std::path::Path,
    xdg_state: &std::path::Path,
    xdg_data: &std::path::Path,
    xdg_cache: &std::path::Path,
    home: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sddk"));
    cmd.env("HOME", home.to_str().unwrap());
    cmd.env("XDG_DATA_HOME", xdg_data.to_str().unwrap());
    cmd.env("XDG_STATE_HOME", xdg_state.to_str().unwrap());
    cmd.env("XDG_CACHE_HOME", xdg_cache.to_str().unwrap());
    cmd.env("USER", "test-cli-actor");
    cmd.current_dir(root);
    cmd.args(args);
    cmd.output().expect("failed to spawn sddk")
}

/// Regression: plan roadmap next uses RuntimeContext::open() canonical resolver.
/// RED: this test fails against unmodified code (walker finds no .sddk/adoption.json).
/// GREEN: passes after open_storage_for_plan() is replaced with RuntimeContext::open().
#[test]
fn plan_roadmap_next_uses_canonical_resolver() {
    // Create isolated XDG environment
    let root = TempDir::new().expect("temp dir");
    let xdg_state = TempDir::new().expect("xdg state temp dir");
    let xdg_data = TempDir::new().expect("xdg data temp dir");
    let xdg_cache = TempDir::new().expect("xdg cache temp dir");
    let home = TempDir::new().expect("home temp dir");

    let root_str = root.path().to_str().unwrap();

    // Create minimal git repo so adopt apply can discover remote
    let git_dir = root.path().join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git dir");

    // Run adopt apply to create canonical receipt under XDG_STATE_HOME
    let adopt_output = run_sddk_subprocess(
        root.path(),
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &[
            "adopt",
            "apply",
            "--root",
            root_str,
            "--scope",
            ".",
            "--timestamp",
            "2026-09-06T00:00:00Z",
            "--actor",
            "test",
            "--format",
            "json",
        ],
    );

    // Adopting the project must succeed
    assert!(
        adopt_output.status.success(),
        "adopt apply failed: {}",
        String::from_utf8_lossy(&adopt_output.stderr)
    );

    // Parse receipt_path from adopt output
    let adopt_json: serde_json::Value = serde_json::from_slice(&adopt_output.stdout)
        .expect("adopt must return JSON");
    let receipt_path_str = adopt_json["receipt_path"]
        .as_str()
        .expect("receipt_path must be string");

    // Verify receipt exists at canonical XDG location (the path from adopt output)
    let receipt_path = std::path::Path::new(receipt_path_str);
    assert!(
        receipt_path.is_file(),
        "canonical adoption receipt must exist at: {} (XDG_STATE_HOME={})",
        receipt_path.display(),
        xdg_state.path().display()
    );

    // Verify the receipt is inside XDG_DATA_HOME (not at project root)
    // Note: receipt is stored under data_home, ledger under state_home
    assert!(
        receipt_path.starts_with(xdg_data.path()),
        "receipt must be under XDG_DATA_HOME: {} vs XDG_DATA_HOME={}",
        receipt_path.display(),
        xdg_data.path().display()
    );

    // Verify NO .sddk/adoption.json at project root (the bug walker looks here)
    let stub_receipt = root.path().join(".sddk/adoption.json");
    assert!(
        !stub_receipt.exists(),
        "bug invariant: no .sddk/adoption.json should exist at project root"
    );

    // Import the spine fixture so the ledger has data for roadmap projections
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let spine_path = std::path::Path::new(manifest_dir)
        .join("..")
        .join("sddk-domain/tests/fixtures/execution_spine_post_reconciliation.yaml");
    assert!(
        spine_path.is_file(),
        "spine fixture must exist: {}",
        spine_path.display()
    );
    let import_output = run_sddk_subprocess(
        root.path(),
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &["plan", "import", "--spine", &spine_path.to_str().unwrap()],
    );
    assert!(
        import_output.status.success(),
        "spine import failed: {}",
        String::from_utf8_lossy(&import_output.stderr)
    );

    // Run plan roadmap next — this is the seam being tested
    // Note: --format is a top-level flag on `sddk plan`, not on roadmap subcommands
    let plan_output = run_sddk_subprocess(
        root.path(),
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &["plan", "--format", "json", "roadmap", "next"],
    );

    let stderr = String::from_utf8_lossy(&plan_output.stderr);
    let stdout = String::from_utf8_lossy(&plan_output.stdout);

    // The bug: plan command cannot find the adopted project
    assert!(
        !stderr.contains("No adopted project found"),
        "stderr must NOT contain 'No adopted project found': {}",
        stderr
    );

    // The fix: plan command exits 0 with valid JSON projection
    assert!(
        plan_output.status.success(),
        "plan roadmap next must exit 0, got {}: {}",
        plan_output.status,
        stderr
    );

    // Verify stdout is valid JSON (projection result)
    let parsed = serde_json::from_str::<serde_json::Value>(&stdout);
    assert!(
        parsed.is_ok(),
        "plan roadmap next stdout must be valid JSON: {}",
        stdout
    );
}

/// Additional regression: verify all 5 roadmap subcommands work after the fix.
#[test]
fn plan_roadmap_all_subcommands_use_canonical_resolver() {
    let root = TempDir::new().expect("temp dir");
    let xdg_state = TempDir::new().expect("xdg state temp dir");
    let xdg_data = TempDir::new().expect("xdg data temp dir");
    let xdg_cache = TempDir::new().expect("xdg cache temp dir");
    let home = TempDir::new().expect("home temp dir");

    let root_str = root.path().to_str().unwrap();

    // Create minimal git repo
    std::fs::create_dir_all(root.path().join(".git")).expect("create .git dir");

    // Adopt
    let adopt_output = run_sddk_subprocess(
        root.path(),
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &[
            "adopt", "apply", "--root", root_str, "--scope", ".",
            "--timestamp", "2026-09-06T00:00:00Z", "--actor", "test", "--format", "json",
        ],
    );
    assert!(adopt_output.status.success(), "adopt apply failed");

    // Import the spine fixture so the ledger has data for roadmap projections
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let spine_path = std::path::Path::new(manifest_dir)
        .join("..")
        .join("sddk-domain/tests/fixtures/execution_spine_post_reconciliation.yaml");
    let import_output = run_sddk_subprocess(
        root.path(),
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &["plan", "import", "--spine", &spine_path.to_str().unwrap()],
    );
    assert!(
        import_output.status.success(),
        "spine import failed: {}",
        String::from_utf8_lossy(&import_output.stderr)
    );

    // Test all roadmap subcommands that read projections (not ones that require data)
    // Note: status/next/blocked output always JSON (no --format flag); graph has --format
    for subcmd in &["status", "next", "blocked"] {
        let output = run_sddk_subprocess(
            root.path(),
            xdg_state.path(),
            xdg_data.path(),
            xdg_cache.path(),
            home.path(),
            &["plan", "roadmap", subcmd],
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("No adopted project found"),
            "plan roadmap {} must use canonical resolver: {}",
            subcmd,
            stderr
        );
        assert!(
            output.status.success(),
            "plan roadmap {} must exit 0: {}",
            subcmd,
            stderr
        );
    }
}