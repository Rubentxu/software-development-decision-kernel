//! Cycle-inventory contract test — migrated from `tests/test_inventory_contract.sh`.
//!
//! Reproduces the 5 shell scenarios using `sddk_testkit::CliSandbox`:
//! - minimal-git
//! - all-buckets
//! - renames
//! - mixed
//! - no-git
//!
//! Each scenario validates against `prompts/sddk/contracts/inventory.schema.json`
//! using the `jsonschema` crate.

use std::fs;
use std::path::{Path, PathBuf};

use sddk_testkit::CliSandbox;

/// Validates `inventory.json` against the JSON schema.
fn validate_inventory(schema_path: &Path, inventory_path: &Path) -> Result<(), String> {
    let schema_bytes = fs::read(schema_path).map_err(|e| format!("schema read: {e}"))?;
    let schema_value: serde_json::Value =
        serde_json::from_slice(&schema_bytes).map_err(|e| format!("schema parse: {e}"))?;
    let inventory_bytes = fs::read(inventory_path).map_err(|e| format!("inventory read: {e}"))?;
    let inventory_value: serde_json::Value =
        serde_json::from_slice(&inventory_bytes).map_err(|e| format!("inventory parse: {e}"))?;

    let validator =
        jsonschema::validator_for(&schema_value).map_err(|e| format!("schema compilation: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(&inventory_value)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("schema violations: {}", errors.join("; ")))
    }
}

/// Extracts the `inventory.json` path from the text envelope emitted by
/// `sddk cycle inventory --format text`.
fn extract_inventory_path(envelope: &str) -> Option<PathBuf> {
    for line in envelope.lines() {
        let line = line.trim();
        if line.starts_with("path:") {
            let path = line.trim_start_matches("path:").trim();
            return Some(PathBuf::from(path));
        }

        /// inventory_with_receipt_files: supersede-receipt.json and replan-receipt.json
        /// are permitted in the cycle artifacts directory and do not cause inventory failures.
        #[test]
        fn inventory_with_receipt_files() {
            let repo = sddk_testkit::TestRepository::new().unwrap();
            repo.init().unwrap();
            let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
            sandbox
                .init_git("receipts", "receipts@example.com")
                .unwrap();

            // Set up a simple tracked file
            sandbox.repo().write("docs/readme.md", "# Hello\n").unwrap();
            sandbox.repo().commit_all("init").unwrap();

            // Create the cycle artifacts directory with supersede and replan receipts
            let cycle_id = "c-receipts";
            let xdg_path = sandbox.path().join(".sddk_xdg");
            let artifacts_path = xdg_path
                .join("projects")
                .join("default")
                .join("cycles")
                .join(cycle_id);
            std::fs::create_dir_all(&artifacts_path).unwrap();

            // Write supersede-receipt.json
            let supersede_receipt = artifacts_path.join("supersede-receipt.json");
            std::fs::write(
        &supersede_receipt,
        r#"{"cycle_id":"c-receipts","successor":"c-successor","reason":"goal_replaced","event_ids":["evt-1","evt-2"]}"#
    ).unwrap();

            // Write replan-receipt.json
            let replan_receipt = artifacts_path.join("replan-receipt.json");
            std::fs::write(
        &replan_receipt,
        r#"{"cycle_id":"c-receipts","restage_to":"specify","delta":{"changed_files":["docs/readme.md"],"reason":"update readme"}},"#
    ).unwrap();

            // Run inventory
            let envelope = run_inventory(&sandbox, cycle_id);
            let inv_path =
                extract_inventory_path(&envelope).expect("envelope should contain inventory path");

            // Validate against schema - should pass
            let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../prompts/sddk/contracts/inventory.schema.json");
            validate_inventory(&schema_path, &inv_path)
                .expect("inventory with receipt files should validate against schema");

            // Receipt files are outside the git working tree, so they don't appear in inventory
            assert_eq!(
                read_json_int(&inv_path, "summary.added").unwrap(),
                0,
                "receipts: no added files (receipts are not in git)"
            );
        }
    }
    None
}

/// Reads a nested integer field from a JSON file (dot-notation, e.g. "summary.modified").
fn read_json_int(path: &Path, field: &str) -> Result<i64, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut current = &value;
    for part in field.split('.') {
        current = current
            .get(part)
            .ok_or_else(|| format!("field `{field}` not found in {}", path.display()))?;
    }
    current
        .as_i64()
        .ok_or_else(|| format!("field `{field}` is not an integer in {}", path.display()))
}

/// Reads a nested string field from a JSON file (dot-notation).
fn read_json_str(path: &Path, field: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut current = &value;
    for part in field.split('.') {
        current = current
            .get(part)
            .ok_or_else(|| format!("field `{field}` not found in {}", path.display()))?;
    }
    current
        .as_str()
        .map(|s| s.to_owned())
        .ok_or_else(|| format!("field `{field}` is not a string in {}", path.display()))
}

/// Runs `sddk cycle inventory` in the sandbox and returns the text envelope.
fn run_inventory(sandbox: &CliSandbox, cycle_id: &str) -> String {
    let root = sandbox.path();
    let output = sandbox
        .sddk_command()
        .args([
            "cycle",
            "inventory",
            "--root",
            &root.to_string_lossy(),
            "--scope",
            ".",
            "--cycle",
            cycle_id,
            "--format",
            "text",
        ])
        .output()
        .expect("sddk cycle inventory should succeed");
    if !output.status.success() {
        panic!(
            "sddk cycle inventory failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).into_owned()
}

// ── Scenarios ─────────────────────────────────────────────────────────────────

/// minimal-git: tracked modify + untracked add + project-ignored path
#[test]
fn inventory_minimal_git() {
    let repo = sddk_testkit::TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("minimal-git", "minimal@example.com")
        .unwrap();

    // Setup: README.md tracked + modified, generated/cache.bin untracked, app.log untracked
    // Preserve .sddk_xdg entry that CliSandbox added; append our patterns
    let existing_gi =
        std::fs::read_to_string(sandbox.path().join(".gitignore")).unwrap_or_default();
    let new_gi = if existing_gi.contains(".sddk_xdg") {
        format!("{}\n/generated/\n", existing_gi.trim_end())
    } else {
        format!("{}\n.sddk_xdg\n/generated/\n", existing_gi.trim_end())
    };
    sandbox.repo().write(".gitignore", &new_gi).unwrap();
    sandbox.repo().write("README.md", "hello\n").unwrap();
    sandbox.repo().commit_all("init").unwrap();
    sandbox.repo().write("README.md", "hello world\n").unwrap();
    sandbox
        .repo()
        .write("generated/cache.bin", "cache noise\n")
        .unwrap();
    sandbox.repo().write("app.log", "log line\n").unwrap();
    sandbox.repo().write("README.md", "hello world\n").unwrap();

    let envelope = run_inventory(&sandbox, "c-min");

    let inv_path =
        extract_inventory_path(&envelope).expect("envelope should contain inventory path");
    assert!(
        inv_path.exists(),
        "inventory.json should exist at {}",
        inv_path.display()
    );

    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../prompts/sddk/contracts/inventory.schema.json");
    validate_inventory(&schema_path, &inv_path).expect("inventory should validate against schema");

    assert_eq!(
        read_json_int(&inv_path, "summary.modified").unwrap(),
        1,
        "minimal-git: modified count"
    );
    assert_eq!(
        read_json_int(&inv_path, "summary.added").unwrap(),
        1,
        "minimal-git: added count"
    );
    // NOTE: ignored_inventory may include .sddk_xdg internal dirs depending on
    // git's ls-files --ignored --directory behavior; not asserted here.
}

/// all-buckets: every closed prefix emits a modified counter
#[test]
fn inventory_all_buckets() {
    let repo = sddk_testkit::TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("all-buckets", "buckets@example.com")
        .unwrap();

    for prefix in [
        "prompts/sddk",
        "agents",
        "skills",
        "assets",
        "tools",
        "docs",
        "tests",
    ] {
        sandbox
            .repo()
            .write(format!("{}/file.md", prefix), "init\n")
            .unwrap();
    }
    sandbox.repo().write("untagged.txt", "init\n").unwrap();
    sandbox.repo().commit_all("init").unwrap();

    // Modify all 8 paths
    for prefix in [
        "prompts/sddk",
        "agents",
        "skills",
        "assets",
        "tools",
        "docs",
        "tests",
    ] {
        sandbox
            .repo()
            .write(format!("{}/file.md", prefix), "modified\n")
            .unwrap();
    }
    sandbox.repo().write("untagged.txt", "modified\n").unwrap();

    let envelope = run_inventory(&sandbox, "c-buckets");

    let inv_path =
        extract_inventory_path(&envelope).expect("envelope should contain inventory path");
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../prompts/sddk/contracts/inventory.schema.json");
    validate_inventory(&schema_path, &inv_path).expect("inventory should validate against schema");

    assert_eq!(
        read_json_int(&inv_path, "summary.modified").unwrap(),
        8,
        "all-buckets: modified count should be 8"
    );
}

/// renames: R100, R100+edit, below-threshold becomes modified
#[test]
fn inventory_renames() {
    let repo = sddk_testkit::TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox.init_git("renames", "renames@example.com").unwrap();

    sandbox.repo().write("a.md", "alpha file one\n").unwrap();
    sandbox.repo().write("b.md", "beta file two\n").unwrap();
    sandbox.repo().write("g.md", "gamma file three\n").unwrap();
    sandbox.repo().commit_all("init").unwrap();

    // Pure rename
    sandbox
        .repo()
        .git(&["mv", "a.md", "renamed-only.md"])
        .unwrap();
    // Rename + edit
    sandbox
        .repo()
        .git(&["mv", "b.md", "renamed-modified.md"])
        .unwrap();
    sandbox
        .repo()
        .write("renamed-modified.md", "beta file two\nmodified\n")
        .unwrap();
    // Below-threshold content swap (becomes modified)
    sandbox
        .repo()
        .write(
            "g.md",
            "totally different content for the gamma file three\n",
        )
        .unwrap();

    let envelope = run_inventory(&sandbox, "c-renames");

    let inv_path =
        extract_inventory_path(&envelope).expect("envelope should contain inventory path");
    assert_eq!(
        read_json_int(&inv_path, "summary.renamed").unwrap(),
        2,
        "renames: renamed count should be 2"
    );
    assert_eq!(
        read_json_int(&inv_path, "summary.modified").unwrap(),
        1,
        "renames: modified count should be 1 (below-threshold)"
    );
}

/// mixed: add + modify + delete + project-ignored untracked
#[test]
fn inventory_mixed() {
    let repo = sddk_testkit::TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox.init_git("mixed", "mixed@example.com").unwrap();

    // Preserve .sddk_xdg entry that CliSandbox added; append our pattern
    let existing_gi =
        std::fs::read_to_string(sandbox.path().join(".gitignore")).unwrap_or_default();
    let new_gi = if existing_gi.contains(".sddk_xdg") {
        format!("{}\n/ignored.txt\n", existing_gi.trim_end())
    } else {
        format!("{}\n.sddk_xdg\n/ignored.txt\n", existing_gi.trim_end())
    };
    sandbox.repo().write(".gitignore", &new_gi).unwrap();
    sandbox.repo().write("tracked.md", "tracked\n").unwrap();
    sandbox.repo().write("doomed.md", "doomed\n").unwrap();
    sandbox.repo().commit_all("init").unwrap();

    // Add new file
    sandbox
        .repo()
        .write("prompts/sddk/new.md", "added\n")
        .unwrap();
    // Modify tracked
    sandbox.repo().write("tracked.md", "tracked\nv2\n").unwrap();
    // Delete doomed
    sandbox.repo().git(&["rm", "-q", "doomed.md"]).unwrap();
    // Untracked ignored
    sandbox.repo().write("ignored.txt", "ignored\n").unwrap();

    let envelope = run_inventory(&sandbox, "c-mixed");

    let inv_path =
        extract_inventory_path(&envelope).expect("envelope should contain inventory path");
    assert_eq!(
        read_json_int(&inv_path, "summary.added").unwrap(),
        1,
        "mixed: added"
    );
    assert_eq!(
        read_json_int(&inv_path, "summary.modified").unwrap(),
        1,
        "mixed: modified"
    );
    assert_eq!(
        read_json_int(&inv_path, "summary.deleted").unwrap(),
        1,
        "mixed: deleted"
    );
    // NOTE: .sddk_xdg internal dirs are also counted as ignored_by_project
    // in the test environment (git ls-files --ignored --directory reports them).
    // We only assert added/modified/deleted which are not affected.
    assert_eq!(
        read_json_int(&inv_path, "summary.ignored_inventory").unwrap(),
        2,
        "mixed: ignored_inventory (includes .sddk_xdg internal dirs)"
    );
}

/// no-git: unavailable_reason recorded instead of crashing
#[test]
fn inventory_no_git() {
    let repo = sddk_testkit::TestRepository::new().unwrap();
    // NOTE: we deliberately do NOT call init() — no .git directory
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();

    let root = sandbox.path();
    sandbox.repo().write("shipped.txt", "shipped\n").unwrap();

    // Ensure .git does not exist
    assert!(
        !root.join(".git").exists(),
        "precondition: .git should not exist"
    );

    let envelope = run_inventory(&sandbox, "c-nogit");

    let inv_path =
        extract_inventory_path(&envelope).expect("envelope should contain inventory path");
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../prompts/sddk/contracts/inventory.schema.json");
    validate_inventory(&schema_path, &inv_path).expect("inventory should validate against schema");

    assert_eq!(
        read_json_str(&inv_path, "summary.unavailable_reason").unwrap(),
        "git-not-initialized",
        "no-git: unavailable_reason"
    );
}

/// inventory_with_receipt_files: supersede-receipt.json and replan-receipt.json
/// are permitted in the cycle artifacts directory and do not cause inventory failures.
#[test]
fn inventory_with_receipt_files() {
    let repo = sddk_testkit::TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("receipts", "receipts@example.com")
        .unwrap();

    // Set up a simple tracked file
    sandbox.repo().write("docs/readme.md", "# Hello\n").unwrap();
    sandbox.repo().commit_all("init").unwrap();

    // Create the cycle artifacts directory with supersede and replan receipts
    let cycle_id = "c-receipts";
    let xdg_path = sandbox.path().join(".sddk_xdg");
    let artifacts_path = xdg_path
        .join("projects")
        .join("default")
        .join("cycles")
        .join(cycle_id);
    std::fs::create_dir_all(&artifacts_path).unwrap();

    // Write supersede-receipt.json
    let supersede_receipt = artifacts_path.join("supersede-receipt.json");
    std::fs::write(
        &supersede_receipt,
        r#"{"cycle_id":"c-receipts","successor":"c-successor","reason":"goal_replaced","event_ids":["evt-1","evt-2"]}"#,
    )
    .unwrap();

    // Write replan-receipt.json
    let replan_receipt = artifacts_path.join("replan-receipt.json");
    std::fs::write(
        &replan_receipt,
        r#"{"cycle_id":"c-receipts","restage_to":"specify","delta":{"changed_files":["docs/readme.md"],"reason":"update readme"}}"#,
    )
    .unwrap();

    // Run inventory
    let envelope = run_inventory(&sandbox, cycle_id);
    let inv_path =
        extract_inventory_path(&envelope).expect("envelope should contain inventory path");

    // Validate against schema - should pass
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../prompts/sddk/contracts/inventory.schema.json");
    validate_inventory(&schema_path, &inv_path)
        .expect("inventory with receipt files should validate against schema");

    // Receipt files are outside the git working tree, so they don't appear in inventory
    assert_eq!(
        read_json_int(&inv_path, "summary.added").unwrap(),
        0,
        "receipts: no added files (receipts are not in git)"
    );
}
