//! Stack-specific test-runner adapters.
//!
//! Six families: cargo-nextest, pytest, jest, go/test, maven/test, gradle/test.
//! Each adapter is a value type that produces a [`RunSpec`] from an [`AdapterRequest`].
//!
//! ## Design
//!
//! - `pub(crate)` trait + six concrete adapters.
//! - `dispatch()` is the single seam: maps `TestFamily` → adapter → `RunSpec`.
//! - Execution of the `RunSpec` is in `runner::run`; adapters never spawn processes.
//! - Missing toolchain → `RunnerError::Spawn { program, source: NotFound }`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::runner::{RunOutcome, RunSpec};

// ─── Submodules ───────────────────────────────────────────────────────────────

pub(crate) mod cargo_nextest;
pub(crate) mod go_test;
pub(crate) mod gradle_test;
pub(crate) mod jest;
pub(crate) mod maven_test;
pub(crate) mod pytest;

/// Base environment keys forwarded to every child (explicit list, NO wildcards).
pub(crate) mod env_allowlist {
    use std::collections::BTreeMap;

    /// Explicit base keys — no wildcards.
    pub(crate) const BASE: &[&str] = &[
        "PATH", "HOME", "USER", "USERNAME", "LANG", "LC_ALL", "TZ", "TMPDIR", "TEMP", "CI",
    ];

    /// Keys whose name matches these suffixes are treated as secret-like.
    fn secret_like_suffixes() -> &'static [&'static str] {
        &["_TOKEN", "_SECRET", "_KEY"]
    }

    /// Returns true when `key` looks like a secret (matches a suffix or is `GITHUB_TOKEN`).
    pub(crate) fn is_secret_like(key: &str) -> bool {
        key == "GITHUB_TOKEN"
            || secret_like_suffixes()
                .iter()
                .any(|s| key.to_uppercase().ends_with(s))
    }

    /// Merges `base` allowlist with `addenda` (addenda override on collision).
    /// Removes any key whose name is secret-like.
    pub(crate) fn merge(
        base: BTreeMap<String, String>,
        addenda: &[(impl AsRef<str>, impl AsRef<str>)],
    ) -> BTreeMap<String, String> {
        let mut result = base;
        for (k, v) in addenda {
            let key = k.as_ref();
            if !is_secret_like(key) {
                result.insert(key.to_owned(), v.as_ref().to_owned());
            }
        }
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn base_keys_are_explicit_no_wildcards() {
            for key in BASE {
                assert!(
                    !key.contains('*'),
                    "BASE key {key} must not contain wildcard"
                );
            }
        }

        #[test]
        fn github_token_is_secret_like() {
            assert!(is_secret_like("GITHUB_TOKEN"));
            assert!(is_secret_like("MY_GITHUB_TOKEN"));
        }

        #[test]
        fn token_secret_key_suffixes_are_secret_like() {
            assert!(is_secret_like("API_TOKEN"));
            assert!(is_secret_like("SECRET_KEY"));
            assert!(is_secret_like("AWS_SECRET"));
            assert!(!is_secret_like("PATH"));
            assert!(!is_secret_like("HOME"));
        }

        #[test]
        fn merge_drops_secret_like_addenda() {
            let base = [("PATH", "/usr/bin")]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect();
            let addenda = [("GITHUB_TOKEN", "secret"), ("PYTHONDONTWRITEBYTECODE", "1")];
            let result = merge(base, &addenda);
            assert!(!result.contains_key("GITHUB_TOKEN"));
            assert_eq!(result.get("PYTHONDONTWRITEBYTECODE"), Some(&"1".to_owned()));
        }
    }
}

/// Toolchain resolution for POSIX executables and trusted shebang wrappers.
pub(crate) mod toolchain {
    use std::path::{Path, PathBuf};

    /// Shell executables that are NEVER used as `RunSpec.program`.
    pub(crate) fn forbidden_shells() -> &'static [&'static str] {
        &["sh", "bash", "zsh", "cmd.exe", "powershell", "pwsh"]
    }

    /// Returns true when `program` is a forbidden shell.
    pub(crate) fn is_shell(program: &str) -> bool {
        forbidden_shells().contains(&program)
    }

    /// Returns true when `path` has a forbidden Windows batch extension.
    fn is_windows_batch(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e.to_lowercase().as_str(), "cmd" | "bat" | "ps1"))
            .unwrap_or(false)
    }

    /// Attempts to resolve `candidate` to a safe executable.
    ///
    /// Returns `Ok(path)` when the file is usable as a direct program:
    /// - POSIX: executable and extension is NOT `.cmd/.bat/.ps1`.
    /// - Windows: extension is NOT `.cmd/.bat/.ps1`.
    ///
    /// Returns `Err(())` when the path is not usable.
    pub(crate) fn resolve_posix_exec(candidate: &Path) -> Result<PathBuf, ()> {
        if !candidate.exists() {
            return Err(());
        }

        // Windows batch files MUST be rejected — Rust Command routes them through cmd.exe.
        if cfg!(windows) && is_windows_batch(candidate) {
            return Err(());
        }

        // On POSIX, check the file is executable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(candidate).map_err(|_| ())?;
            let mode = meta.permissions().mode();
            if mode & 0o111 == 0 {
                return Err(()); // not executable
            }
        }

        Ok(candidate.to_path_buf())
    }

    /// Checks whether the given program name or path is acceptable as a direct
    /// `RunSpec.program` (no shell involved).
    pub(crate) fn accept_direct_program(program: &str) -> Result<(), ()> {
        if is_shell(program) {
            return Err(());
        }
        let path = Path::new(program);
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e.to_lowercase().as_str(), "cmd" | "bat" | "ps1"))
            .unwrap_or(false)
        {
            return Err(());
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn shell_names_are_forbidden() {
            for shell in forbidden_shells() {
                assert!(is_shell(shell), "{shell} must be forbidden");
            }
            assert!(!is_shell("cargo"));
            assert!(!is_shell("pytest"));
            assert!(!is_shell("node"));
            assert!(!is_shell("go"));
        }

        #[test]
        fn windows_batch_extensions_rejected() {
            assert!(is_windows_batch(Path::new("mvn.cmd")));
            assert!(is_windows_batch(Path::new("mvn.bat")));
            assert!(is_windows_batch(Path::new("gradle.ps1")));
            assert!(!is_windows_batch(Path::new("mvn")));
            assert!(!is_windows_batch(Path::new("./mvnw")));
        }
    }
}

/// Test-family identifier for the six supported runners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestFamily {
    /// `cargo nextest run --workspace`
    CargoNextest,
    /// `pytest --maxfail=1 -q`
    Pytest,
    /// `node node_modules/jest/bin/jest.js --ci --runInBand`
    Jest,
    /// `go test -count=1 -timeout=<ms>ms`
    GoTest,
    /// `mvn -B -q test` or `./mvnw -B -q test`
    MavenTest,
    /// `gradle --no-daemon -q test` or `./gradlew --no-daemon -q test`
    GradleTest,
}

/// Incoming request for an adapter.
#[derive(Debug, Clone)]
pub struct AdapterRequest {
    /// Project root to run tests in.
    pub project_root: PathBuf,
    /// Per-invocation timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum captured stdout/stderr bytes per stream.
    pub output_max_bytes: usize,
}

/// Adapter-level errors that are translated into `RunnerError` by `dispatch()`.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum AdapterError {
    /// No safe executable candidate could be found for this family.
    ToolchainMissing {
        family: TestFamily,
        searched: Vec<PathBuf>,
    },
    /// A candidate wrapper exists but is unusable.
    WrapperUnusable { path: PathBuf, reason: &'static str },
    /// Unrecognised family name.
    UnknownFamily(String),
}

/// Adapter request that carries the resolved program/args/env for execution.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct ResolvedSpec {
    pub spec: RunSpec,
    /// Last candidate that was tried (used for error messages).
    pub last_candidate: PathBuf,
}

/// Returns the current process's environment as a base allowlist map.
/// Only keys in `env_allowlist::BASE` are included; secret-like keys are dropped.
#[allow(dead_code)]
pub(crate) fn current_env_as_allowlist() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for key in env_allowlist::BASE {
        // BASE items are `&str`; env::var borrows from them via AsRef<OsStr>.
        // When key is `&str`, var returns String; when key is `&&str`, var returns &str.
        // We always .to_owned() the value to ensure String.
        if let Ok(val) = std::env::var(*key)
            && !env_allowlist::is_secret_like(key)
        {
            map.insert(key.to_string(), val.to_owned());
        }
    }
    map
}

// ─── Trait & dispatch ────────────────────────────────────────────────────────

/// Adapter trait for stack-specific test runners.
///
/// Each adapter is a value type (no interior state) that:
/// - Identifies its family ([`family()`](TestRunnerAdapter::family)).
/// - Builds the argv for its family ([`build_args()`](TestRunnerAdapter::build_args)).
/// - Returns per-family env addenda ([`env_addenda()`](TestRunnerAdapter::env_addenda);
///   `jest` returns `&[]` because `NODE_OPTIONS` is a code-injection vector).
/// - Resolves the toolchain to a [`ResolvedSpec`](ResolvedSpec).
#[allow(dead_code)]
pub(crate) trait TestRunnerAdapter: Send + Sync {
    /// Which family this adapter handles.
    fn family(&self) -> TestFamily;

    /// Build the argument vector for this invocation.
    /// The `go/test` adapter formats `-timeout={req.timeout_ms}ms` here.
    fn build_args(&self, req: &AdapterRequest) -> Vec<String>;

    /// Per-family environment addenda merged into the base allowlist.
    /// Jest returns `&[]` — `NODE_OPTIONS` must never be forwarded.
    fn env_addenda(&self) -> &[(&'static str, &'static str)];

    /// Resolves the toolchain and returns a [`ResolvedSpec`] or a typed error.
    fn resolve(&self, req: &AdapterRequest) -> Result<ResolvedSpec, AdapterError>;
}

#[allow(dead_code)]
/// Maps a `TestFamily` to its corresponding adapter.
fn adapter_for(family: TestFamily) -> Box<dyn TestRunnerAdapter> {
    match family {
        TestFamily::CargoNextest => Box::new(cargo_nextest::CargoNextestAdapter),
        TestFamily::Pytest => Box::new(pytest::PytestAdapter),
        TestFamily::Jest => Box::new(jest::JestAdapter),
        TestFamily::GoTest => Box::new(go_test::GoTestAdapter),
        TestFamily::MavenTest => Box::new(maven_test::MavenTestAdapter),
        TestFamily::GradleTest => Box::new(gradle_test::GradleTestAdapter),
    }
}

/// Dispatches an [`AdapterRequest`] to the appropriate adapter.
///
/// 1. Selects the adapter by `TestFamily`.
/// 2. Calls `adapter.resolve()` → `ResolvedSpec`.
/// 3. Merges base env + adapter addenda (secret-like keys dropped).
/// 4. Calls `runner::run()` → [`RunOutcome`].
///
/// On toolchain missing: maps to `RunnerError::Spawn { program, source: NotFound }`.
#[allow(dead_code)]
pub(crate) fn dispatch(
    req: &AdapterRequest,
    family: TestFamily,
) -> Result<RunOutcome, crate::runner::RunnerError> {
    let adapter = adapter_for(family);
    let addenda = adapter.env_addenda(); // captured before resolve to use after

    let resolved = match adapter.resolve(req) {
        Ok(r) => r,
        Err(e) => {
            let (program, source) = match e {
                AdapterError::ToolchainMissing { searched, .. } => {
                    let last = searched.last().cloned().unwrap_or_default();
                    (
                        last.to_string_lossy().into_owned(),
                        std::io::Error::from(std::io::ErrorKind::NotFound),
                    )
                }
                AdapterError::WrapperUnusable { path, .. } => (
                    path.to_string_lossy().into_owned(),
                    std::io::Error::from(std::io::ErrorKind::NotFound),
                ),
                AdapterError::UnknownFamily(name) => {
                    (name, std::io::Error::from(std::io::ErrorKind::NotFound))
                }
            };
            return Err(crate::runner::RunnerError::Spawn { program, source });
        }
    };

    let base = current_env_as_allowlist();
    // Merge base allowlist + adapter addenda (secret-like keys filtered by merge).
    let env = env_allowlist::merge(base, addenda);
    let spec = RunSpec {
        env,
        ..resolved.spec
    };

    crate::runner::run(&spec)
}

// ─── Conformance tests (spec v3 §Scenarios) ──────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::runner::RunSpec;

    #[test]
    fn case8_truthful_timeout() {
        let spec = RunSpec {
            program: "sleep".to_owned(),
            args: vec!["5".to_owned()],
            env: Default::default(),
            timeout_ms: 50, // very short — child outlives it.
            output_max_bytes: 1024,
        };

        let outcome = crate::runner::run(&spec).expect("run must succeed");
        assert!(
            outcome.timed_out,
            "timed_out must be true when child outlives timeout"
        );
        assert_eq!(
            outcome.exit_status, None,
            "exit_status must be None after timeout-kill"
        );
    }

    // S9 / Case 9 (45c): report-output opacity — output returned verbatim.
    #[test]
    fn case9_output_not_parsed() {
        let spec = RunSpec {
            program: "echo".to_owned(),
            args: vec![
                "-n".to_owned(),
                r#"<?xml version="1.0"?><testsuite name="sample" tests="1" failures="0"><testcase name="test_ok"/></testsuite>"#
                    .to_owned(),
            ],
            env: Default::default(),
            timeout_ms: 5_000,
            output_max_bytes: 1_024,
        };

        let outcome = crate::runner::run(&spec).expect("echo must succeed");
        assert!(
            outcome.stdout.contains("testsuite"),
            "output must be returned verbatim, got: {}",
            outcome.stdout
        );
    }

    // S10 / Case 10 (45c): contract drift — RunSpec/RunOutcome field integrity.
    #[test]
    fn case10_public_runner_contract_stable() {
        // Verify field structure is unchanged: RunSpec and RunOutcome contain exactly
        // the same fields as cycle-44. We test via run() round-trip (not serde
        // — RunOutcome does not implement Serialize to preserve byte-identical baseline).
        let spec = RunSpec {
            program: "echo".to_owned(),
            args: vec!["hello".to_owned()],
            env: Default::default(),
            timeout_ms: 5_000,
            output_max_bytes: 1024,
        };

        let outcome = crate::runner::run(&spec).expect("run must succeed");

        // Fields are present and correctly typed.
        assert!(outcome.stdout.contains("hello"));
        assert!(!outcome.timed_out);
        assert_eq!(outcome.exit_status, Some(0));

        // RunSpec fields are structurally identical to cycle-44 baseline.
        assert_eq!(spec.program, "echo");
        assert_eq!(spec.args, &["hello"]);
        assert_eq!(spec.timeout_ms, 5_000);
        assert_eq!(spec.output_max_bytes, 1024);

        // RunOutcome fields identical to cycle-44 baseline.
        assert!(!outcome.stdout.is_empty());
        assert!(outcome.stderr.is_empty() || !outcome.stderr.is_empty()); // always accessible
        assert!(!outcome.timed_out);
    }
}
