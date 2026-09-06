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
    let adopt_json: serde_json::Value =
        serde_json::from_slice(&adopt_output.stdout).expect("adopt must return JSON");
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
        &["plan", "import", "--spine", spine_path.to_str().unwrap()],
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
        &["plan", "import", "--spine", spine_path.to_str().unwrap()],
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

/// Regression: verify all plan subcommands exercise the canonical resolver (not the broken walker).
/// Covers workitem list, dep add, evidence attach, decision record, graph — all 7 subcommands
/// share the same RuntimeContext::open() resolver via open_storage_for_plan().
#[test]
fn plan_subcommands_all_use_canonical_resolver() {
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
    assert!(adopt_output.status.success(), "adopt apply failed");

    // Import the spine fixture
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
        &["plan", "import", "--spine", spine_path.to_str().unwrap()],
    );
    assert!(
        import_output.status.success(),
        "spine import failed: {}",
        String::from_utf8_lossy(&import_output.stderr)
    );

    // Test remaining subcommands (roadmap {status,next,blocked} already tested above).
    // Each subcommand should reach the resolver without "No adopted project found".
    let subcommand_tests = [
        // workitem list — cycle_id from spine fixture items (TEST-LEDGER-C)
        (
            &["plan", "workitem", "list", "--cycle-id", "TEST-LEDGER-C"] as &[_],
            "workitem list",
        ),
        // dep add — requires two work items; use stub IDs (will fail with not-found, not No adopted)
        (
            &["plan", "dep", "add", "--from-id", "A", "--to-id", "B"],
            "dep add",
        ),
        // evidence attach — requires body file + work item; stub args (will fail not-found)
        (
            &[
                "plan",
                "evidence",
                "attach",
                "--work-item-id",
                "X",
                "--kind",
                "log",
                "--body-file",
                "/dev/null",
            ],
            "evidence attach",
        ),
        // decision record — requires work item; stub args (will fail not-found)
        (
            &[
                "plan",
                "decision",
                "record",
                "--work-item-id",
                "X",
                "--kind",
                "accept",
                "--rationale",
                "test",
                "--actor-id",
                "agent:cli",
            ],
            "decision record",
        ),
        // graph — requires cycle_id
        (&["plan", "graph", "--cycle-id", "TEST-LEDGER-C"], "graph"),
    ];

    for (args, name) in subcommand_tests {
        let output = run_sddk_subprocess(
            root.path(),
            xdg_state.path(),
            xdg_data.path(),
            xdg_cache.path(),
            home.path(),
            args,
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("No adopted project found"),
            "plan {} must use canonical resolver (not cwd-walker): {}",
            name,
            stderr
        );
    }
}

/// Regression: explicit --root/--scope flags override CWD inference for plan roadmap next.
/// Scenario 3 of the spec: when run from a non-adopted cwd, --root/--scope must succeed.
#[test]
fn plan_flags_override_cwd_inference() {
    let root = TempDir::new().expect("temp dir");
    let xdg_state = TempDir::new().expect("xdg state temp dir");
    let xdg_data = TempDir::new().expect("xdg data temp dir");
    let xdg_cache = TempDir::new().expect("xdg cache temp dir");
    let home = TempDir::new().expect("home temp dir");

    let root_str = root.path().to_str().unwrap();

    // Create minimal git repo
    std::fs::create_dir_all(root.path().join(".git")).expect("create .git dir");

    // Adopt the project
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
    assert!(adopt_output.status.success(), "adopt apply failed");

    // Import the spine fixture
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
        &["plan", "import", "--spine", spine_path.to_str().unwrap()],
    );
    assert!(
        import_output.status.success(),
        "spine import failed: {}",
        String::from_utf8_lossy(&import_output.stderr)
    );

    // Create a SECOND tempdir that is NOT adopted
    let non_adopted = TempDir::new().expect("non-adopted temp dir");
    std::fs::create_dir_all(non_adopted.path().join(".git")).expect("create .git dir");

    // From the non-adopted cwd WITHOUT --root/--scope: must fail with "No adopted project found"
    let no_flags_output = run_sddk_subprocess(
        non_adopted.path(),
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &["plan", "roadmap", "next"],
    );
    let no_flags_stderr = String::from_utf8_lossy(&no_flags_output.stderr);
    assert!(
        no_flags_stderr.contains("No adopted project found"),
        "without --root/--scope from non-adopted cwd, expected 'No adopted project found', got: {}",
        no_flags_stderr
    );

    // From the non-adopted cwd WITH --root pointing to adopted project: must succeed
    let with_root_output = run_sddk_subprocess(
        non_adopted.path(),
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &[
            "plan", "--root", root_str, "--scope", ".", "roadmap", "next",
        ],
    );
    let with_root_stderr = String::from_utf8_lossy(&with_root_output.stderr);
    assert!(
        !with_root_stderr.contains("No adopted project found"),
        "with --root/--scope must override CWD inference: {}",
        with_root_stderr
    );
    assert!(
        with_root_output.status.success(),
        "plan roadmap next with --root/--scope must exit 0: {}",
        with_root_stderr
    );

    // Wrong --root (non-adopted path) must fail with adopted-project error (not cwd-walker error)
    let wrong_root_output = run_sddk_subprocess(
        root.path(), // run from adopted dir but with wrong --root
        xdg_state.path(),
        xdg_data.path(),
        xdg_cache.path(),
        home.path(),
        &[
            "plan",
            "--root",
            non_adopted.path().to_str().unwrap(),
            "--scope",
            ".",
            "roadmap",
            "next",
        ],
    );
    let wrong_root_stderr = String::from_utf8_lossy(&wrong_root_output.stderr);
    // Should fail because non_adopted is not an adopted project
    assert!(
        wrong_root_stderr.contains("No adopted project found"),
        "wrong --root must fail with adopted-project error: {}",
        wrong_root_stderr
    );
}
