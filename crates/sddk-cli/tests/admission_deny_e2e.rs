//! E2E: authority-engine Deny produces zero side effects (WU-C4-2, R-4-005).
//!
//! Injectable deny case: `cycle evaluate-gate --actor user:*`. The
//! `gate_receipts` surface is High band and `cli.execute` is System-only
//! there (legacy C3 matrix preserved in `derive_capabilities_for`), so a
//! Human actor is denied by the engine BEFORE any ledger write. The test
//! asserts: non-zero exit, ADMISSION message with decision_id, and an
//! empty project ledger afterwards (zero external side effects).
//!
//! Follows the fixture pattern of `tests/cli.rs` (XDG temp dirs + real
//! binary via CARGO_BIN_EXE_sddk).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct DenyFixture {
    _directory: TempDir,
    root: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
    home: PathBuf,
}

impl DenyFixture {
    fn new(name: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join(name);
        fs::create_dir_all(&root).unwrap();
        // Canonicalize like tests/cli.rs: TMPDIR may be a symlinked path
        // (e.g. /var/home -> /home) and sddk canonicalizes on emission.
        let canon = |p: PathBuf| fs::canonicalize(&p).unwrap_or(p);
        Self {
            home: canon(directory.path().join("home")),
            root: canon(root),
            data: canon(directory.path().join("data")),
            state: canon(directory.path().join("state")),
            cache: canon(directory.path().join("cache")),
            _directory: directory,
        }
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sddk"));
        command
            .args(args)
            .env("HOME", &self.home)
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_STATE_HOME", &self.state)
            .env("XDG_CACHE_HOME", &self.cache)
            .env("USER", "user:deny-e2e");
        command.output().unwrap()
    }
}

/// Collect every file under the XDG state dir (the durable side-effect
/// surface of the CLI). Empty vec = zero side effects.
fn state_tree(state: &Path) -> Vec<String> {
    let mut out = vec![];
    let mut stack = vec![state.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(
                    path.strip_prefix(state)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    out
}

/// Zero-governed-mutation check (R-4-005): the ledger may be bootstrapped
/// by `RuntimeContext::open` (schema + project row, storage-local), but a
/// deny must leave ZERO events in it. No event = no durable domain effect.
fn ledger_event_count(state: &Path, project_dir: &str) -> usize {
    let ledger = state
        .join("sddk")
        .join("projects")
        .join(project_dir)
        .join("ledger.sqlite");
    if !ledger.exists() {
        return 0;
    }
    let store = sddk_storage::Storage::open_read_only(&ledger)
        .expect("ledger must be openable if it exists");
    store.list_events().expect("list_events").len()
}

#[test]
fn human_evaluate_gate_denied_with_zero_side_effects() {
    let fixture = DenyFixture::new("admission-deny-e2e");

    // Baseline: fresh environment, no durable state yet.
    assert!(
        state_tree(&fixture.state).is_empty(),
        "fixture must start with no durable state"
    );

    // Human actor on gate_receipts: the engine denies cli.execute on High
    // band surfaces for non-System actors before any mutation.
    let denied = fixture.run(&[
        "cycle",
        "evaluate-gate",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/deny-e2e.git",
        "--cycle",
        "c-deny-e2e",
        "--transition",
        "phase.explore.complete",
        "--gate",
        "exploration-sufficient",
        "--evaluator",
        "sddk.cli",
        "--outcome",
        "passed",
        "--evidence",
        r#"{"checked": true}"#,
        "--actor",
        "user:mallory",
        "--format",
        "json",
    ]);

    // 1) Non-zero exit: admission blocks the command.
    assert!(
        !denied.status.success(),
        "evaluate-gate must fail for a Human actor on gate_receipts; stdout={} stderr={}",
        String::from_utf8_lossy(&denied.stdout),
        String::from_utf8_lossy(&denied.stderr),
    );

    // 2) Operator-facing ADMISSION message with a correlatable decision id.
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(
        stderr.contains("ADMISSION") && stderr.contains("decision_id="),
        "expected ADMISSION + decision_id in stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("no changes were made"),
        "expected zero-side-effect wording, got: {stderr}"
    );

    // 3) Zero governed mutations: `RuntimeContext::open` bootstraps the
    //    ledger file (schema + project row — storage-local, identical for
    //    every command), but the deny must leave ZERO events in it (R-4-005).
    //    Once WU-C4-3 lands, the same scenario will additionally assert a
    //    single `authority.admission.decided` event as the ONLY event.
    let project_dirs: Vec<String> = fs::read_dir(fixture.state.join("sddk").join("projects"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    for project_dir in &project_dirs {
        let events = ledger_event_count(&fixture.state, project_dir);
        assert_eq!(
            events, 0,
            "deny must append zero events to ledger of {project_dir}"
        );
    }
    // And no receipts or other durable artifacts beyond the bootstrap
    // ledger file (and its sqlite -wal/-shm siblings) itself.
    let durable: Vec<String> = state_tree(&fixture.state)
        .into_iter()
        .filter(|p| {
            !(p.ends_with("ledger.sqlite")
                || p.ends_with("ledger.sqlite-wal")
                || p.ends_with("ledger.sqlite-shm"))
        })
        .collect();
    assert!(
        durable.is_empty(),
        "deny must leave zero durable artifacts beyond the bootstrapped ledger, found: {durable:?}"
    );
}

#[test]
fn system_evaluate_gate_not_denied_by_actor_matrix() {
    // Control case: a System actor (default, no user: prefix) passes the
    // same matrix point — the denial above is authority-driven, not a
    // broken command. Failure is allowed (unknown cycle id etc.) but the
    // error must NOT be an ADMISSION denial.
    let fixture = DenyFixture::new("admission-system-e2e");
    let out = fixture.run(&[
        "cycle",
        "evaluate-gate",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/system-e2e.git",
        "--cycle",
        "c-system-e2e",
        "--transition",
        "phase.explore.complete",
        "--gate",
        "exploration-sufficient",
        "--evaluator",
        "sddk.cli",
        "--outcome",
        "passed",
        "--evidence",
        r#"{"checked": true}"#,
        "--actor",
        "sddk-verify-worker",
        "--format",
        "json",
    ]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("ADMISSION"),
        "System actor must not hit an ADMISSION denial, got: {stderr}"
    );
}
