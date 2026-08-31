//! `dev check` — run repository quality gates (fmt, clippy, tests, comments).

use crate::{CommandOutput, OutputFormat};
use sddk_gateway::{RunSpec, run};

pub(super) fn run_dev_check(args: super::CheckArgs) -> CommandOutput {
    let steps = [
        ("fmt", vec!["fmt", "--all", "--", "--check"]),
        (
            "clippy",
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("test", vec!["test", "--workspace", "--locked"]),
    ];
    let mut text = String::new();
    let mut failed = false;
    for (name, args) in steps {
        let spec = RunSpec {
            program: "cargo".into(),
            args: args.into_iter().map(str::to_owned).collect(),
            timeout_ms: 600_000,
            output_max_bytes: 1_048_576,
            env: Default::default(),
        };
        let outcome = match run(&spec) {
            Ok(outcome) => outcome,
            Err(error) => {
                failed = true;
                text.push_str(&format!("{name}: FAILED ({error})\n"));
                continue;
            }
        };
        let passed = outcome.exit_status == Some(0) && !outcome.timed_out;
        if !passed {
            failed = true;
        }
        text.push_str(&format!(
            "{name}: {}\n",
            if passed { "PASS" } else { "FAIL" }
        ));
    }

    // Comments gate: scan production code files for forbidden comment patterns
    // (apply.md L591-608 + verify.md §3.b). Read-only; never modifies files.
    //
    // Rules are loaded from the highest-priority source available:
    //   1. SDDK_COMMENTS_RULES env var (path to a YAML contract)
    //   2. --rules CLI flag (path to a YAML contract)
    //   3. Compile-time default (prompts/sddk/contracts/comments-rules.yaml)
    let rules = resolve_rules(&args, &mut text, &mut failed);

    let scope_label;
    let comments_result = if let Some(ref git_ref) = args.since {
        scope_label = format!("comments (added since {git_ref}): ");
        match super::comments_check::added_lines_since(&args.root, &rules, git_ref) {
            Ok(None) => {
                failed = true;
                text.push_str(&format!(
                    "{scope_label}FAILED (git diff returned no output for ref `{git_ref}`)\n"
                ));
                Err(anyhow::anyhow!("git diff empty"))
            }
            Ok(Some(added)) if added.ranges.is_empty() => {
                text.push_str(&format!(
                    "{scope_label}PASS (no added lines since {git_ref})\n"
                ));
                Ok(Vec::new())
            }
            Ok(Some(added)) => {
                let file_count = added.file_predicate().len();
                text.push_str(&format!(
                    "{scope_label}scanning {file_count} changed file(s)...\n"
                ));
                super::comments_check::scan_added_lines(&args.root, &rules, &added)
            }
            Err(e) => {
                failed = true;
                text.push_str(&format!("{scope_label}FAILED ({e})\n"));
                Err(e)
            }
        }
    } else {
        scope_label = String::from("comments: ");
        super::comments_check::scan(&args.root, &rules)
    };

    match comments_result {
        Ok(violations) if violations.is_empty() => {
            text.push_str(&format!("{scope_label}PASS\n"));
        }
        Ok(violations) => {
            failed = true;
            text.push_str(&format!("{scope_label}FAIL ({} hits)\n", violations.len()));
            for v in violations.iter().take(20) {
                let rel = v
                    .file
                    .strip_prefix(&args.root)
                    .unwrap_or(&v.file)
                    .to_string_lossy()
                    .replace('\\', "/");
                text.push_str(&format!(
                    "  {}:{} [{}] {}\n",
                    rel,
                    v.line,
                    v.rule,
                    v.snippet.chars().take(120).collect::<String>(),
                ));
            }
            if violations.len() > 20 {
                text.push_str(&format!("  ... and {} more\n", violations.len() - 20));
            }
        }
        Err(e) => {
            failed = true;
            text.push_str(&format!("comments: FAILED ({e})\n"));
        }
    }

    let mut output = CommandOutput {
        status: i32::from(failed),
        stdout: text,
        stderr: String::new(),
    };
    if let OutputFormat::Json = args.format {
        output.stdout = format!("{}\n", serde_json::json!({"passed": !failed}));
    }
    output
}

/// Resolve the comments-rules contract. Priority:
/// 1. `SDDK_COMMENTS_RULES` env var.
/// 2. `--rules` CLI flag.
/// 3. Compile-time default.
///
/// On hard failure (invalid path / malformed YAML), we still emit a
/// fallback empty contract so the rest of the gates can run, but mark
/// `failed = true`.
fn resolve_rules(
    args: &super::CheckArgs,
    text: &mut String,
    failed: &mut bool,
) -> super::comments_check::RulesContract {
    let from_env = std::env::var("SDDK_COMMENTS_RULES").ok();
    let chosen = from_env.as_deref().map(Path::new).or(args.rules.as_deref());
    match chosen {
        Some(path) => match super::comments_check::load_rules_from_path(path) {
            Ok(rules) => {
                text.push_str(&format!(
                    "comments: using custom rules from {}\n",
                    path.display()
                ));
                rules
            }
            Err(e) => {
                *failed = true;
                text.push_str(&format!(
                    "comments: failed to load rules from {}: {e}\n",
                    path.display()
                ));
                // Fallback: empty contract = no patterns means no hits,
                // which lets the rest of the gates still run.
                super::comments_check::RulesContract {
                    languages: Vec::new(),
                    patterns: Vec::new(),
                    exclude_paths: Vec::new(),
                    skip_dirs: Vec::new(),
                }
            }
        },
        None => super::comments_check::load_rules(),
    }
}

use std::path::Path;
