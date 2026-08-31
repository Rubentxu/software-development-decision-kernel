//! CLI Compatibility Snapshot Tests (SDDK2-001)
//!
//! Pins the current CLI command surface as a golden contract. Before any 2.0
//! refactoring touches `crates/*/src/`, these tests prove HEAD behavior is
//! preserved. If Phase 1 breaks a command, a compatibility test fails
#![allow(unused_variables, clippy::collapsible_if)]
//! immediately.
//!
//! ## Regeneration (opt-in, write-and-fail-safe)
//!
//! ```text
//! UPDATE_SNAPSHOTS=1 cargo test --test cli_compatibility
//! ```
//!
//! Writes new bytes to `CARGO_MANIFEST_DIR/tests/fixtures/cli/<name>.txt`.
//! The test then **fails** with `FIXTURE UPDATED — review required` and exits 101.
//! A regression MUST NEVER become silently green.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

use tempfile::TempDir;

// Golden Fixtures
const HELP_TOP_LEVEL: &str = include_str!("fixtures/cli/help-top-level.txt");
const HELP_UAT: &str = include_str!("fixtures/cli/help-uat.txt");

// Normalization
fn normalize(output: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(output)
        .replace("\r\n", "\n")
        .replace("\r", "\n");
    mask_dynamic_content(&strip_ansi(&s)).into_bytes()
}

fn strip_ansi(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    let mut cs = s.chars().peekable();
    while let Some(c) = cs.next() {
        if c == '\x1b' {
            while let Some(&n) = cs.peek() {
                cs.next();
                if n == 'm' {
                    break;
                }
            }
        } else {
            r.push(c);
        }
    }
    r
}

static VERSION_RE: LazyLock<regex::Regex, fn() -> regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"v\d+(?:\.\d+)+").unwrap());

fn mask_dynamic_content(s: &str) -> String {
    let mut r = s.replace(env!("CARGO_BIN_EXE_sddk"), "<BIN>");
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            r = r.replace(&home, "<HOME>");
        }
    }
    r = VERSION_RE.replace_all(&r, "<VERSION>").to_string();
    r = r.replace(&std::process::id().to_string(), "<PID>");
    // Generic timestamp patterns (RFC 3339, ISO 8601, Unix epoch, dates)
    r = regex::Regex::new(
        r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?",
    )
    .unwrap()
    .replace_all(&r, "<TS>")
    .to_string();
    r = regex::Regex::new(r"\d{2}/\d{2}/\d{4} \d{2}:\d{2}:\d{2}")
        .unwrap()
        .replace_all(&r, "<TS>")
        .to_string();
    r
}

// UPDATE_SNAPSHOTS
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cli")
        .join(name)
        .with_extension("txt")
}

fn check_update_snapshots(name: &str, content: &str) {
    if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") {
        let path = fixture_path(name);
        match fs::write(&path, content) {
            Ok(_) => {
                eprintln!(
                    "FIXTURE UPDATED — review required; re-run without UPDATE_SNAPSHOTS to verify"
                );
                std::process::exit(101);
            }
            Err(e) => panic!("cannot write fixture: {} ({})", path.display(), e),
        }
    }
}

// Snapshot Tests
fn assert_help_snapshot(name: &str, actual: Vec<u8>, expected: &[u8]) {
    let exp_norm = normalize(expected);
    let actual_str = String::from_utf8_lossy(&actual).to_string();
    // Compare FIRST, then conditionally update
    if actual != exp_norm {
        check_update_snapshots(name, &actual_str);
        assert_eq!(actual, exp_norm, "{name} snapshot mismatch");
    }
}

#[test]
fn top_level_help_matches_snapshot() {
    let out = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .arg("--help")
        .output()
        .expect("sddk --help");
    assert_help_snapshot(
        "help-top-level",
        normalize(&out.stderr),
        HELP_TOP_LEVEL.as_bytes(),
    );
}

#[test]
fn uat_help_matches_snapshot() {
    let out = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["uat", "--help"])
        .output()
        .expect("sddk uat --help");
    assert_help_snapshot("help-uat", normalize(&out.stderr), HELP_UAT.as_bytes());
}

// CliFixture for isolated flow tests
struct CliFixture {
    _dir: TempDir,
    root: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
    home: PathBuf,
}

impl CliFixture {
    fn new(name: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(name);
        std::fs::create_dir_all(&root).unwrap();
        Self {
            root,
            data: dir.path().join("data"),
            state: dir.path().join("state"),
            cache: dir.path().join("cache"),
            home: dir.path().join("home"),
            _dir: dir,
        }
    }
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(args)
            .env("HOME", &self.home)
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_STATE_HOME", &self.state)
            .env("XDG_CACHE_HOME", &self.cache)
            .output()
            .expect("sddk command")
    }
    // Create minimal adopted repo + running cycle, return cycle_id
    fn with_cycle(&self) -> String {
        let root_arg = self.root.to_str().unwrap();
        let adopt = self.run(&[
            "adopt",
            "apply",
            "--root",
            root_arg,
            "--scope",
            ".",
            "--timestamp",
            "2026-08-12T00:00:00Z",
            "--actor",
            "test",
            "--format",
            "json",
        ]);
        assert!(
            adopt.status.success(),
            "adopt apply failed: {}",
            String::from_utf8_lossy(&adopt.stderr)
        );
        let proj_id: serde_json::Value = serde_json::from_slice(&adopt.stdout).unwrap();
        let _pid = proj_id["project_id"].as_str().unwrap();
        let start = self.run(&[
            "cycle",
            "start",
            "--root",
            root_arg,
            "--scope",
            ".",
            "--name",
            "test",
            "--timestamp",
            "2026-08-12T00:00:00Z",
            "--actor",
            "test",
            "--format",
            "json",
        ]);
        assert!(
            start.status.success(),
            "cycle start failed: {}",
            String::from_utf8_lossy(&start.stderr)
        );
        let started: serde_json::Value = serde_json::from_slice(&start.stdout).unwrap();
        started["cycle_id"].as_str().unwrap().to_string()
    }
}

#[test]
fn cycle_status_exits_zero_with_canonical_json() {
    let fx = CliFixture::new("cycle-status");
    let cycle_id = fx.with_cycle();
    let root = fx.root.to_str().unwrap();
    let out = fx.run(&[
        "cycle", "status", "--root", root, "--scope", ".", "--cycle", &cycle_id, "--format", "json",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty(), "stderr should be empty");
    let p: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        p.get("cycle_id").is_some()
            && p.get("status").is_some()
            && p.get("phase").is_some()
            && p.get("path").is_some()
            && p.get("lease").is_some(),
        "missing required keys"
    );
}

#[test]
fn ledger_verify_exits_zero_with_clean_json() {
    let fx = CliFixture::new("ledger-verify");
    let root = fx.root.to_str().unwrap();
    let out = fx.run(&[
        "ledger",
        "verify",
        "--root",
        root,
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000000",
        "--format",
        "json",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let p: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let has_err = p
        .as_object()
        .map(|o| o.contains_key("error"))
        .unwrap_or(false);
    assert!(
        !has_err || p["error"].is_null(),
        "should not have error key"
    );
}

#[test]
fn capability_status_exits_zero_with_empty_array() {
    let fx = CliFixture::new("capability-status");
    let root = fx.root.to_str().unwrap();
    let out = fx.run(&[
        "capability",
        "status",
        "--root",
        root,
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000000",
        "--format",
        "json",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let arr = serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap();
    let arr = arr.as_array().unwrap();
    assert!(arr.is_empty(), "should be empty array");
}

// Unit test for normalization
#[test]
fn normalize_handles_crlf_ansi_sentinels() {
    // CRLF
    assert_eq!(normalize(b"line1\r\nline2\r\n"), b"line1\nline2\n");
    // ANSI
    assert_eq!(normalize(b"\x1b[32mgreen\x1b[0m"), b"green");
    // HOME
    let home = std::env::var("HOME").unwrap();
    let with_home = format!("{}/.config/sddk", home);
    let out = normalize(with_home.as_bytes());
    assert!(!String::from_utf8_lossy(&out).contains(&home));
    // PID
    let with_pid = format!("process {}\n", std::process::id());
    let out = normalize(with_pid.as_bytes());
    assert!(!String::from_utf8_lossy(&out).contains(&std::process::id().to_string()));
    // TS (RFC 3339)
    let ts = "2026-08-12T15:30:00Z";
    let out = normalize(ts.as_bytes());
    assert!(!String::from_utf8_lossy(&out).contains("2026-08-12"));
    // VERSION
    let out = normalize(b"v1.9.1 and v2.0.0-alpha");
    assert!(!String::from_utf8_lossy(&out).contains("v1.9.1"));
}
