//! Regression test: MANIFEST.sha256 contains only git-tracked files.
//!
//! Per REQ-Bundle-Coverage, the manifest must be generated from tracked files only
//! (via `git ls-files`), not from filesystem walks that could include ignored or
//! untracked entries.

use std::process::Command;

#[test]
fn manifest_contains_only_tracked_files() {
    // Read the MANIFEST.sha256 file from the workspace root
    // CARGO_MANIFEST_DIR for a test binary points to the crate root
    let manifest_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../MANIFEST.sha256");
    let manifest_content =
        std::fs::read_to_string(&manifest_path).expect("failed to read MANIFEST.sha256");

    // Get list of all files tracked by git in the current worktree
    // Run git from the repo root to ensure consistent results
    //
    // NO pathspec. The property under test is "every entry in MANIFEST.sha256 is
    // git-tracked"; filtering `ls-files` by surface was an optimisation that
    // turned into a fifth hand-written copy of MANIFEST_SURFACES. It went stale
    // at 2bc0511c, which added `specs` as a sixth surface: the four pathspecs
    // still said agents/skills/prompts/assets, so all 14 `specs/` files and both
    // `docs/impeccable-reference/` files — every one of them tracked, as
    // `git ls-files` and `git status` confirm — were reported as untracked. A
    // guard that reports a defect that does not exist is worse than no guard.
    //
    // Dropping the pathspec makes the test ask git directly, so there is no list
    // here to keep in sync. The reverse direction (tracked but missing from the
    // manifest) is a different property and is covered by
    // `tests/test_bundle_surface_coverage.py`.
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tracked_output = Command::new("git")
        .args(["ls-files", "--full-name"])
        .current_dir(&repo_root)
        .output()
        .expect("failed to execute git ls-files");

    let tracked_files: std::collections::HashSet<String> =
        std::str::from_utf8(&tracked_output.stdout)
            .expect("git output not utf-8")
            .lines()
            .map(|s| s.to_string())
            .collect();

    // For each file in the manifest, verify it's tracked by git
    let mut failures = Vec::new();
    for line in manifest_content.lines() {
        let parts: Vec<&str> = line.splitn(2, "  ").collect();
        if parts.len() != 2 {
            continue; // skip malformed lines
        }
        let file_path = parts[1].trim();

        if !tracked_files.contains(file_path) {
            failures.push(file_path.to_string());
        }
    }

    if !failures.is_empty() {
        panic!(
            "MANIFEST.sha256 contains {} untracked files: {:?}",
            failures.len(),
            &failures[..failures.len().min(10)]
        );
    }
}
