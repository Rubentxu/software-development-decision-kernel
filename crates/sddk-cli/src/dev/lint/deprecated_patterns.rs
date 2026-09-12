//! `dev lint deprecated-patterns` — execute the M0 D6 registry of deprecated
//! patterns against the live workspace baseline.
//!
//! Reads `docs/architecture/lints/deprecated_patterns.toml` from the workspace
//! root (or a path supplied via `--registry`), parses each lint's machine-
//! readable `pattern` (regex) and `paths` (glob list), walks the matching
//! Rust source files, and reports every hit with `file:line:col` precision.
//!
//! Output formats: `--format text` (default, tabular) or `--format json`
//! (machine-readable, suitable for CI consumption).
//!
//! Enforcement modes (exit code semantics):
//! - advisory (default): exit 0 regardless of hits, the user inspects text output.
//! - `--enforce`: exit non-zero if any lint with `default: deny` reports
//!   hits. At v1.168.34 four lints are `default: deny` and reported with
//!   0 hits against the live workspace: `agent_result_used` (M9.1, v1.168.12)
//!   and the three AX-S5 agent-asset hygiene lints (v1.168.29). The five
//!   `default: allow` advisory lints split by blocker kind: 1 is construction-
//!   blocked (`evidence_kind_v1`, gated on ADR-0100 + cycle-5 evidence-
//!   relations) and 4 are audit-blocked (`transition_outcome_used` description
//!   rewrite, `orchestration_synthesis_no_dissent` runner extension,
//!   `execution_outcome_as_synthesis` corpus expansion, `asset_unregistered_cli_example`
//!   regex unsafe-by-design). Per-lint rationale lives in each [[lints]].explanation
//!   block of `deprecated_patterns.toml`. See header table (ARCH-LINT-M9.2).
//!   `--enforce` is meaningful and passes on a clean workspace today; the
//!   flag stays for forward-compatibility with future promotion cycles.
//!
//! Implementation notes:
//! - Glob expansion uses `glob::glob_with` for cross-platform behaviour.
//! - Regex compilation uses `regex::Regex` with default options.
//! - The runner does NOT spawn `ripgrep`; pure Rust keeps the binary
//!   dependency-free and the test surface small.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::CommandOutput;
use crate::OutputFormat;

#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    #[serde(default = "default_schema_version")]
    #[allow(dead_code)]
    schema_version: String,
    #[serde(default)]
    #[allow(dead_code)]
    generated_at: String,
    #[serde(default)]
    #[allow(dead_code)]
    enforcement_status: String,
    lints: Vec<LintSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LintSpec {
    pub id: String,
    pub severity: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    description: String,
    #[serde(default)]
    #[allow(dead_code)]
    detection: String,
    pub pattern: String,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub replacement: Vec<String>,
}

fn default_schema_version() -> String {
    "v1".to_string()
}

/// One hit found during lint execution: which lint, which file/line/col, the
/// offending snippet.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, serde::Deserialize)]
pub struct LintHit {
    pub lint_id: String,
    pub severity: String,
    pub file: String,
    pub line: u64,
    pub column: u64,
    pub snippet: String,
}

/// Outcome of a single lint run.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, serde::Deserialize)]
pub struct LintOutcome {
    pub lint_id: String,
    pub severity: String,
    pub default_mode: String,
    pub hits: usize,
    pub replacement: Vec<String>,
}

/// Combined report returned by `run_lints`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, serde::Deserialize)]
pub struct LintReport {
    pub schema_version: String,
    pub evaluated_at: String,
    pub registry_path: String,
    pub lints: Vec<LintOutcome>,
    pub hits: Vec<LintHit>,
    pub exit_status: i32,
}

/// Arguments for `dev lint deprecated-patterns`.
#[derive(Debug, Clone, Args)]
pub struct DeprecatedPatternsArgs {
    /// Workspace root (default: current dir).
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Path to the registry TOML (default: `<root>/docs/architecture/lints/deprecated_patterns.toml`).
    #[arg(long)]
    pub registry: Option<PathBuf>,
    /// If set, exit non-zero when any lint with `default: deny` finds hits.
    /// Since v1.168.12 (ARCH-LINT-M9.1) `agent_result_used` is `default:
    /// deny` with zero production hits, so `--enforce` is meaningful and
    /// passes on a clean workspace. Further promotions land as their
    /// replacement ADRs (ADR-0100/0101) are accepted and built.
    #[arg(long)]
    pub enforce: bool,
    /// Output format (`text` or `json`).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

/// Loads and validates the registry TOML.
pub fn load_registry(path: &Path) -> Result<Registry, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("failed to read registry {}: {e}", path.display()))?;
    toml::from_str::<Registry>(&text)
        .map_err(|e| format!("invalid TOML in {}: {e}", path.display()))
}

/// Walks the given glob patterns and applies the regex, returning every hit.
/// Excluded patterns are filtered out before the walk.
pub fn collect_hits(root: &Path, lint: &LintSpec) -> Result<Vec<LintHit>, String> {
    let regex = Regex::new(&lint.pattern)
        .map_err(|e| format!("invalid regex `{}` for lint {}: {e}", lint.pattern, lint.id))?;

    let exclude_patterns = lint
        .exclude_paths
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect::<Vec<_>>();

    let mut hits = Vec::new();

    for path_glob in &lint.paths {
        let full_pattern = root.join(path_glob);
        let pattern_str = full_pattern.to_string_lossy();
        let entries = glob::glob_with(
            &pattern_str,
            glob::MatchOptions {
                case_sensitive: true,
                require_literal_separator: false,
                require_literal_leading_dot: false,
            },
        )
        .map_err(|e| format!("invalid glob `{pattern_str}`: {e}"))?;

        for entry in entries {
            let path = entry.map_err(|e| format!("glob walk error: {e}"))?;
            if !path.is_file() {
                continue;
            }

            // Apply excludes relative to root (so absolute excludes like
            // `/abs/path/file.rs` would still work, but the convention is
            // to write paths relative to the workspace root).
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_str = rel.to_string_lossy();
            let excluded = exclude_patterns
                .iter()
                .any(|p| p.matches_path(Path::new(&*rel_str)));
            if excluded {
                continue;
            }

            let content = fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

            for (idx, line) in content.lines().enumerate() {
                if let Some(m) = regex.find(line) {
                    hits.push(LintHit {
                        lint_id: lint.id.clone(),
                        severity: lint.severity.clone(),
                        file: rel_str.to_string(),
                        line: (idx + 1) as u64,
                        column: (m.start() + 1) as u64,
                        snippet: line.trim().to_string(),
                    });
                }
            }
        }
    }

    Ok(hits)
}

/// Runs every lint in the registry and produces the combined report.
/// `enforce = true` flips the exit_status when any `default: deny` lint
/// has hits. Lints with `default: allow` are advisory regardless.
pub fn run_lints(registry_path: &Path, root: &Path, enforce: bool) -> Result<LintReport, String> {
    let registry = load_registry(registry_path)?;

    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    let mut outcomes = Vec::new();
    let mut all_hits = Vec::new();
    let mut deny_failures: BTreeMap<String, usize> = BTreeMap::new();

    for lint in &registry.lints {
        let hits = collect_hits(root, lint)?;
        let mode = lint.default.clone().unwrap_or_else(|| "allow".to_string());
        if mode == "deny" && !hits.is_empty() {
            deny_failures.insert(lint.id.clone(), hits.len());
        }
        all_hits.extend(hits.iter().cloned());
        outcomes.push(LintOutcome {
            lint_id: lint.id.clone(),
            severity: lint.severity.clone(),
            default_mode: mode,
            hits: hits.len(),
            replacement: lint.replacement.clone(),
        });
    }

    let exit_status = if enforce && !deny_failures.is_empty() {
        1
    } else {
        0
    };

    Ok(LintReport {
        schema_version: "v1".to_string(),
        evaluated_at: now,
        registry_path: registry_path.to_string_lossy().to_string(),
        lints: outcomes,
        hits: all_hits,
        exit_status,
    })
}

/// Text rendering — the human-facing summary table.
fn render_text(report: &LintReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();

    let _ = writeln!(
        out,
        "deprecated-patterns registry: {} ({} lints, evaluated at {})",
        report.registry_path,
        report.lints.len(),
        report.evaluated_at
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:<32} {:<8} {:<8} {:>5}",
        "LINT_ID", "SEVERITY", "DEFAULT", "HITS"
    );
    let _ = writeln!(out, "{}", "-".repeat(60));
    for o in &report.lints {
        let _ = writeln!(
            out,
            "{:<32} {:<8} {:<8} {:>5}",
            o.lint_id, o.severity, o.default_mode, o.hits
        );
    }

    let total_hits = report.hits.len();
    if total_hits == 0 {
        let _ = writeln!(out);
        let _ = writeln!(out, "no hits.");
        return out;
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "hits ({}):", total_hits);
    for h in &report.hits {
        let _ = writeln!(
            out,
            "  {}:{}:{}  [{}]  {}",
            h.file, h.line, h.column, h.lint_id, h.snippet
        );
    }
    out
}

/// Public entry point invoked by `dev/mod.rs::run_dev`.
pub fn run_deprecated_patterns_lint(args: DeprecatedPatternsArgs) -> CommandOutput {
    let root = args.root.unwrap_or_else(|| std::path::PathBuf::from("."));
    let registry_path = args
        .registry
        .unwrap_or_else(|| root.join("docs/architecture/lints/deprecated_patterns.toml"));

    match run_lints(&registry_path, &root, args.enforce) {
        Ok(report) => {
            let stdout = match args.format {
                OutputFormat::Text => render_text(&report),
                OutputFormat::Json => {
                    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
                }
            };
            CommandOutput {
                status: report.exit_status,
                stdout,
                stderr: String::new(),
            }
        }
        Err(e) => CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("error: {e}\n"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_registry(dir: &Path, body: &str) -> PathBuf {
        let p = dir.join("deprecated_patterns.toml");
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    fn write_crate_source(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, body).unwrap();
    }

    const MINIMAL_REGISTRY: &str = r#"
schema_version = "v1"

[[lints]]
id = "foo_used"
severity = "warn"
default = "allow"
description = "foo is legacy"
detection = "Find references to foo"
pattern = '\bfoo\b'
paths = ["crates/**/*.rs"]
exclude_paths = ["crates/sddk-cli/src/excluded.rs"]
replacement = ["Use bar instead"]

[[lints]]
id = "bar_used_deny"
severity = "deny"
default = "deny"
description = "bar is denied"
detection = "Find references to bar"
pattern = '\bbar\b'
paths = ["crates/**/*.rs"]
exclude_paths = []
replacement = ["Use baz instead"]
"#;

    fn setup_workspace() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        write_crate_source(
            tmp.path(),
            "crates/sddk-cli/src/lib.rs",
            "pub fn f() {\n    let foo = 1;\n    let bar = 2;\n}\n",
        );
        write_crate_source(
            tmp.path(),
            "crates/sddk-cli/src/excluded.rs",
            "pub fn excluded_foo() { let foo = 0; }\n",
        );
        tmp
    }

    #[test]
    fn loads_valid_registry() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write_registry(tmp.path(), MINIMAL_REGISTRY);
        let r = load_registry(&p).unwrap();
        assert_eq!(r.lints.len(), 2);
        assert_eq!(r.lints[0].id, "foo_used");
        assert_eq!(r.lints[1].id, "bar_used_deny");
        assert_eq!(r.lints[1].default.as_deref(), Some("deny"));
    }

    #[test]
    fn rejects_invalid_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write_registry(tmp.path(), "not valid = = toml");
        let err = load_registry(&p).unwrap_err();
        assert!(err.contains("invalid TOML"), "got: {err}");
    }

    #[test]
    fn collects_hits_in_included_paths() {
        let tmp = setup_workspace();
        let p = write_registry(tmp.path(), MINIMAL_REGISTRY);
        let r = load_registry(&p).unwrap();
        let foo_lint = &r.lints[0];
        let hits = collect_hits(tmp.path(), foo_lint).unwrap();
        // foo appears twice: once in lib.rs, once in excluded.rs (filtered).
        assert_eq!(
            hits.len(),
            1,
            "expected 1 hit after exclude filter, got {hits:?}"
        );
        assert!(hits[0].file.ends_with("lib.rs"));
        assert_eq!(hits[0].line, 2);
    }

    #[test]
    fn enforce_flag_exits_nonzero_on_deny_lint() {
        let tmp = setup_workspace();
        let p = write_registry(tmp.path(), MINIMAL_REGISTRY);
        let r = run_lints(&p, tmp.path(), true).unwrap();
        assert_eq!(
            r.exit_status, 1,
            "bar_used_deny has hits, enforce must fail"
        );
        // The deny lint (bar) has one hit; the allow lint (foo) has one too.
        let deny = r
            .lints
            .iter()
            .find(|o| o.lint_id == "bar_used_deny")
            .unwrap();
        assert_eq!(deny.hits, 1);
        let allow = r.lints.iter().find(|o| o.lint_id == "foo_used").unwrap();
        assert_eq!(allow.hits, 1);
    }

    #[test]
    fn advisory_mode_exits_zero_even_with_hits() {
        let tmp = setup_workspace();
        let p = write_registry(tmp.path(), MINIMAL_REGISTRY);
        let r = run_lints(&p, tmp.path(), false).unwrap();
        assert_eq!(r.exit_status, 0, "no enforce, even deny lints are advisory");
    }

    #[test]
    fn output_format_text_includes_summary_and_hits() {
        let tmp = setup_workspace();
        let p = write_registry(tmp.path(), MINIMAL_REGISTRY);
        let r = run_lints(&p, tmp.path(), false).unwrap();
        let text = render_text(&r);
        assert!(text.contains("LINT_ID"), "missing header");
        assert!(text.contains("foo_used"));
        assert!(text.contains("bar_used_deny"));
        // One of the hits:
        assert!(
            text.contains("let foo = 1") || text.contains("let bar = 2"),
            "snippet missing in: {text}"
        );
    }

    #[test]
    fn output_format_json_round_trips() {
        let tmp = setup_workspace();
        let p = write_registry(tmp.path(), MINIMAL_REGISTRY);
        let r = run_lints(&p, tmp.path(), false).unwrap();
        let json = serde_json::to_string_pretty(&r).unwrap();
        let back: LintReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.lints.len(), r.lints.len());
        assert_eq!(back.hits.len(), r.hits.len());
        assert_eq!(back.exit_status, r.exit_status);
    }

    #[test]
    fn no_hits_when_pattern_absent() {
        let tmp = tempfile::tempdir().unwrap();
        write_crate_source(
            tmp.path(),
            "crates/sddk-cli/src/lib.rs",
            "pub fn f() {\n    let baz = 1;\n}\n",
        );
        let p = write_registry(tmp.path(), MINIMAL_REGISTRY);
        let r = run_lints(&p, tmp.path(), false).unwrap();
        assert_eq!(r.hits.len(), 0);
        assert_eq!(r.exit_status, 0);
    }

    #[test]
    fn invalid_regex_surfaces_as_error() {
        let tmp = tempfile::tempdir().unwrap();
        // Unbalanced bracket in pattern.
        let bad = r#"
[[lints]]
id = "broken"
severity = "warn"
pattern = '[unclosed'
paths = ["crates/**/*.rs"]
"#;
        let p = write_registry(tmp.path(), bad);
        let r = load_registry(&p).unwrap();
        let err = collect_hits(tmp.path(), &r.lints[0]).unwrap_err();
        assert!(err.contains("invalid regex"), "got: {err}");
    }

    #[test]
    fn live_registry_asset_lints_are_promoted_or_advisory_and_clean() {
        // ARCH-LINT-AX-S5 promotion guard (v1.168.29): three of the four
        // AX-S5 agent-asset hygiene lints are now `default: deny`
        // (asset_deprecated_namespace, asset_raw_store_reference,
        // asset_authority_language) — they pass all three acceptance
        // criteria from [acceptance_for_m9_blocking_enforcement]:
        // validation_per_lint (0 hits, no legacy paths), migration
        // completion (convention only, no replacement code), and
        // user_signoff (continuous auto-mode authorization).
        //
        // asset_unregistered_cli_example remains `default: allow` because
        // its first-letter-sieve regex is unsafe against legitimate
        // commands (memory/metrics/status/ship/fork/stale/explore).
        // Full-surface validation stays the responsibility of the AX-S1
        // `live_registry_command_specs_are_in_sync` pin.
        //
        // Every lint must produce ZERO hits against the live corpus
        // (agents/ + prompts/); a hit under deny is a CI blocker, a hit
        // under allow is a regression.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest
            .ancestors()
            .nth(2)
            .expect("crate lives two levels under workspace root")
            .to_path_buf();
        let registry_path = workspace_root.join("docs/architecture/lints/deprecated_patterns.toml");
        let registry = load_registry(&registry_path).expect("live registry must parse");

        // Three promoted lints: deny + zero hits.
        for id in [
            "asset_deprecated_namespace",
            "asset_raw_store_reference",
            "asset_authority_language",
        ] {
            let lint = registry
                .lints
                .iter()
                .find(|l| l.id == id)
                .unwrap_or_else(|| panic!("{id} must exist"));
            assert_eq!(
                lint.default.as_deref(),
                Some("deny"),
                "{id} promoted to deny in v1.168.29"
            );
            let hits = collect_hits(&workspace_root, lint).unwrap();
            assert!(
                hits.is_empty(),
                "{id} deny but found hits: {hits:?} — corpus regressed"
            );
        }

        // One advisory lint: allow + zero hits (regression guard).
        let advisory_id = "asset_unregistered_cli_example";
        let advisory = registry
            .lints
            .iter()
            .find(|l| l.id == advisory_id)
            .unwrap_or_else(|| panic!("{advisory_id} must exist"));
        assert_eq!(
            advisory.default.as_deref(),
            Some("allow"),
            "{advisory_id} stays advisory per ARCH-LINT-AX-S5 (unsafe regex)"
        );
        let advisory_hits = collect_hits(&workspace_root, advisory).unwrap();
        assert!(
            advisory_hits.is_empty(),
            "{advisory_id} advisory but found hits: {advisory_hits:?}"
        );
    }

    #[test]
    fn live_registry_agent_result_used_is_deny_and_clean() {
        // ARCH-LINT-M9.1 promotion guard: `agent_result_used` is the one
        // promoted lint. It must stay `default: deny` AND produce zero
        // hits against the live workspace (the spike corpus and converter
        // surface are excluded). If this test fails with hits, someone
        // reintroduced the legacy aggregate into production code.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest
            .ancestors()
            .nth(2)
            .expect("crate lives two levels under workspace root")
            .to_path_buf();
        let registry_path = workspace_root.join("docs/architecture/lints/deprecated_patterns.toml");
        let registry = load_registry(&registry_path).expect("live registry must parse");
        let lint = registry
            .lints
            .iter()
            .find(|l| l.id == "agent_result_used")
            .expect("agent_result_used lint must exist");
        assert_eq!(lint.default.as_deref(), Some("deny"));
        let hits = collect_hits(&workspace_root, lint).unwrap();
        assert!(
            hits.is_empty(),
            "agent_result_used is deny but found hits: {hits:?}"
        );
    }
}
