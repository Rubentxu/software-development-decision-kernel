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
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tracked_output = Command::new("git")
        .args([
            "ls-files",
            "--",
            "prompts/sddk",
            "skills",
            "agents",
            "assets",
        ])
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
