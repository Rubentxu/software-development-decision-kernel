//! Integration tests for phase9 ratchets: channels, ratchet, golden docs.

use std::process::Command;
use tempfile::TempDir;

struct RatchetTestEnv {
    _dir: TempDir,
    #[allow(dead_code)]
    root: std::path::PathBuf,
}

fn ratchet_test_setup() -> (
    RatchetTestEnv,
    impl Fn(&[&str], bool) -> std::process::Output,
) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let state = tmp.path().join("state");
    let data = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    let home = tmp.path().join("home");
    for dir in [&root, &state, &data, &cache, &home] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .output();
    // A baseline that the ratchet reads from the repo.
    std::fs::create_dir_all(root.join("fixtures")).unwrap();
    std::fs::write(
        root.join("fixtures").join("baseline-arch.json"),
        r#"{"score": 0, "measured_at": "2026-08-18T00:00:00Z"}"#,
    )
    .unwrap();

    let home_c = home.clone();
    let data_c = data.clone();
    let state_c = state.clone();
    let cache_c = cache.clone();
    let root_c = root.clone();

    let run = move |args: &[&str], with_runtime: bool| {
        let mut full: Vec<&str> = args.to_vec();
        if with_runtime {
            full.extend_from_slice(&[
                "--root",
                ".",
                "--scope",
                ".",
                "--fallback-seed",
                "00000000-0000-0000-0000-000000000001",
            ]);
        }
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(&full)
            .env("HOME", &home_c)
            .env("XDG_DATA_HOME", &data_c)
            .env("XDG_STATE_HOME", &state_c)
            .env("XDG_CACHE_HOME", &cache_c)
            .current_dir(&root_c)
            .output()
            .unwrap()
    };

    (RatchetTestEnv { _dir: tmp, root }, run)
}

#[test]
fn channel_promote_allowed_with_gates() {
    let (_env, run) = ratchet_test_setup();
    let out = run(
        &[
            "release",
            "channel",
            "--from",
            "edge",
            "--to",
            "candidate",
            "--gates-ok",
        ],
        false,
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("allowed: true"), "got: {stdout}");
}

#[test]
fn channel_promote_blocked_without_gates() {
    let (_env, run) = ratchet_test_setup();
    let out = run(
        &["release", "channel", "--from", "edge", "--to", "candidate"],
        false,
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_ne!(out.status.code(), Some(0), "must fail without gates");
    assert!(stdout.contains("allowed: false"), "got: {stdout}");
}

#[test]
fn channel_promote_non_adjacent_rejected() {
    let (_env, run) = ratchet_test_setup();
    let out = run(
        &[
            "release",
            "channel",
            "--from",
            "dev",
            "--to",
            "stable",
            "--gates-ok",
        ],
        false,
    );
    assert_ne!(out.status.code(), Some(0), "non-adjacent must fail");
}

#[test]
fn rules_ratchet_passes_with_clean_baseline() {
    let (_env, run) = ratchet_test_setup();
    let out = run(&["rules", "check", "--ratchet"], true);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "ratchet must pass, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("passed"), "got: {stdout}");
}
