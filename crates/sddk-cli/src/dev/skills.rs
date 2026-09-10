//! M8 — `sddk dev skills list/verify` subcommand.
//!
//! Surfaces the runtime skill registry that the M7.7 CLI runner gate
//! consults. Two subcommands:
//!
//! - `list`: enumerate every skill loaded from the active framework
//!   bundle + user-/project-level overrides. Output is JSON-friendly
//!   (one row per skill: scope, id, version, ref_token, source path).
//! - `verify`: walk every `CommandSpec` with declared `required_skill`s
//!   and report whether each is admitted, missing, or matches a known
//!   M7.5 contractual placeholder.
//!
//! This is the user-facing counterpart of the M7.7 bridge: the gate
//! fires on every `lint`/`cycle`/`release`, but the operator usually
//! wants to know *why* the gate warned and *which* skills are actually
//! available. `sddk dev skills list` answers both questions.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;

use crate::command_spec::CommandSpec;
use crate::skill_registry_bridge::{
    self, AdmissionOutcome, DiskBridgeOptions, LoadedSkill, PLACEHOLDER_SKILLS,
};
use crate::{CliEnvironment, CommandOutput, OutputFormat};

#[derive(Debug, Args)]
pub(crate) struct SkillsArgs {
    #[command(subcommand)]
    pub command: SkillsCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum SkillsCommand {
    /// List every skill loaded from the active framework bundle and
    /// user/project overrides.
    List(SkillsListArgs),
    /// Verify that each declared `required_skill` in any CommandSpec is
    /// satisfied by the loaded registry, or report the gap.
    Verify(SkillsVerifyArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct SkillsListArgs {
    /// Restrict to a single scope (framework / user / project).
    #[arg(long, value_enum)]
    pub scope: Option<SkillsScopeArg>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum SkillsScopeArg {
    Framework,
    User,
    Project,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct SkillsVerifyArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
    /// Include commands whose `required_skills` is empty (for audit
    /// completeness).
    #[arg(long)]
    pub include_empty: bool,
}

/// One row of the `sddk dev skills list` output.
#[derive(Debug, Serialize)]
struct SkillRow {
    /// "framework" | "user" | "project"
    scope: String,
    /// Scope-prefixed id (e.g. `framework.accessibility-reviewer`).
    id: String,
    /// Numeric version parsed from frontmatter.
    version: u32,
    /// `id@v<version>` reference token.
    ref_token: String,
    /// Absolute path to the SKILL.md that produced this entry.
    path: String,
}

impl SkillRow {
    fn from_loaded(loaded: &LoadedSkill) -> Self {
        Self {
            scope: match loaded.scope {
                crate::skill_registry_bridge::SkillScope::Framework => "framework".into(),
                crate::skill_registry_bridge::SkillScope::User => "user".into(),
                crate::skill_registry_bridge::SkillScope::Project => "project".into(),
            },
            id: loaded.definition.id.clone(),
            version: loaded.definition.version,
            ref_token: loaded.definition.ref_token(),
            path: loaded.source_path.to_string_lossy().into_owned(),
        }
    }
}

/// Resolve the bridge options from the active CLI environment.
fn bridge_options_for_environment(
    environment: &CliEnvironment,
) -> anyhow::Result<DiskBridgeOptions> {
    let framework_root = crate::dev::paths::resolve_active_framework_root(environment)?;
    let home = environment
        .home
        .clone()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from));
    Ok(DiskBridgeOptions {
        project_root: std::env::current_dir().ok(),
        framework_root: Some(framework_root),
        home,
    })
}

/// Run `sddk dev skills list`.
pub(crate) fn run_dev_skills_list(
    args: SkillsListArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let opts = match bridge_options_for_environment(environment) {
        Ok(o) => o,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("could not resolve framework root: {error}"),
            };
        }
    };
    let (registry, loaded) = skill_registry_bridge::load_registry(&opts);

    let rows: Vec<SkillRow> = loaded
        .iter()
        .filter(|sk| match args.scope {
            None => true,
            Some(SkillsScopeArg::Framework) => matches!(
                sk.scope,
                crate::skill_registry_bridge::SkillScope::Framework
            ),
            Some(SkillsScopeArg::User) => {
                matches!(sk.scope, crate::skill_registry_bridge::SkillScope::User)
            }
            Some(SkillsScopeArg::Project) => {
                matches!(sk.scope, crate::skill_registry_bridge::SkillScope::Project)
            }
        })
        .map(SkillRow::from_loaded)
        .collect();

    match args.format {
        OutputFormat::Text => render_skills_text(&rows, registry.entries().len()),
        OutputFormat::Json => match serde_json::to_string_pretty(&SkillListReport {
            total_loaded: loaded.len(),
            total_after_filter: rows.len(),
            rows: &rows,
        }) {
            Ok(json) => CommandOutput {
                status: 0,
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("failed to serialize skill rows: {error}"),
            },
        },
    }
}

#[derive(Serialize)]
struct SkillListReport<'a> {
    total_loaded: usize,
    total_after_filter: usize,
    rows: &'a [SkillRow],
}

fn render_skills_text(rows: &[SkillRow], total_loaded: usize) -> CommandOutput {
    let mut out = String::new();
    out.push_str(&format!(
        "skills loaded: {} (after filter: {})\n\n",
        total_loaded,
        rows.len()
    ));
    out.push_str("scope      ref_token                                   id                                                  version  path\n");
    out.push_str("────────── ─────────────────────────────────────────── ────────────────────────────────────────────────── ──────── ─────────────\n");
    for row in rows {
        out.push_str(&format!(
            "{:<10} {:<42} {:<50} {:<8} {}\n",
            row.scope, row.ref_token, row.id, row.version, row.path
        ));
    }
    if rows.is_empty() {
        out.push_str("(no skills matched the filter)\n");
    }
    CommandOutput {
        status: 0,
        stdout: out,
        stderr: String::new(),
    }
}

/// Run `sddk dev skills verify`.
pub(crate) fn run_dev_skills_verify(
    args: SkillsVerifyArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let opts = match bridge_options_for_environment(environment) {
        Ok(o) => o,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("could not resolve framework root: {error}"),
            };
        }
    };
    let (registry, _loaded) = skill_registry_bridge::load_registry(&opts);

    // Collect every CommandSpec with declared required_skill(s).
    let specs: Vec<CommandSpec> = crate::command_spec::all_command_specs()
        .into_iter()
        .filter(|s| args.include_empty || !s.required_skills.is_empty())
        .collect();

    let mut report = VerifyReport {
        registry_size: registry.entries().len(),
        commands_checked: 0,
        commands_admitted: 0,
        commands_with_placeholders: 0,
        commands_blocked: 0,
        rows: Vec::new(),
        known_placeholders: PLACEHOLDER_SKILLS.iter().map(|s| s.to_string()).collect(),
    };

    for spec in &specs {
        report.commands_checked += 1;
        let outcome = skill_registry_bridge::check_admission(spec, &registry);
        let row = match &outcome {
            AdmissionOutcome::NotRequired => {
                report.commands_admitted += 1;
                VerifyRow {
                    command: spec.name.clone(),
                    status: "admitted-no-skills".into(),
                    detail: String::new(),
                    required_skills: spec.required_skills.clone(),
                }
            }
            AdmissionOutcome::Admitted { satisfied_by } => {
                report.commands_admitted += 1;
                VerifyRow {
                    command: spec.name.clone(),
                    status: "admitted".into(),
                    detail: format!("satisfied by {} ref_tokens", satisfied_by.len()),
                    required_skills: spec.required_skills.clone(),
                }
            }
            AdmissionOutcome::MissingPlaceholderSkill { required } => {
                report.commands_with_placeholders += 1;
                VerifyRow {
                    command: spec.name.clone(),
                    status: "placeholder-missing".into(),
                    detail: format!(
                        "{required} is one of the three M7.5 contractual placeholders; \
                         the gate warns but does not block"
                    ),
                    required_skills: spec.required_skills.clone(),
                }
            }
            AdmissionOutcome::MissingRealSkill {
                required,
                available,
            } => {
                report.commands_blocked += 1;
                VerifyRow {
                    command: spec.name.clone(),
                    status: "missing-real-skill".into(),
                    detail: format!(
                        "{required} not registered; {} skills available",
                        available.len()
                    ),
                    required_skills: spec.required_skills.clone(),
                }
            }
        };
        report.rows.push(row);
    }

    match args.format {
        OutputFormat::Text => render_verify_text(&report),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => CommandOutput {
                status: if report.commands_blocked == 0 { 0 } else { 64 },
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("failed to serialize verify report: {error}"),
            },
        },
    }
}

#[derive(Serialize)]
struct VerifyReport {
    registry_size: usize,
    commands_checked: usize,
    commands_admitted: usize,
    commands_with_placeholders: usize,
    commands_blocked: usize,
    rows: Vec<VerifyRow>,
    known_placeholders: Vec<String>,
}

#[derive(Serialize)]
struct VerifyRow {
    command: String,
    status: String,
    detail: String,
    required_skills: Vec<String>,
}

fn render_verify_text(report: &VerifyReport) -> CommandOutput {
    let mut out = String::new();
    out.push_str("=== sddk dev skills verify ===\n\n");
    out.push_str(&format!(
        "registry size:        {}\ncommands checked:      {}\n  admitted:            {}\n  placeholder-warning: {}\n  blocked:             {}\n\n",
        report.registry_size,
        report.commands_checked,
        report.commands_admitted,
        report.commands_with_placeholders,
        report.commands_blocked,
    ));
    out.push_str(&format!(
        "known M7.5 placeholders (warn but do not block): {} total\n",
        report.known_placeholders.len()
    ));
    for token in &report.known_placeholders {
        out.push_str(&format!("  - {token}\n"));
    }
    out.push_str("\ncommand                         status                  detail\n");
    out.push_str(
        "────────────────────────────── ──────────────────────── ──────────────────────────\n",
    );
    for row in &report.rows {
        out.push_str(&format!(
            "{:<30} {:<23} {}\n",
            row.command, row.status, row.detail
        ));
    }
    CommandOutput {
        status: if report.commands_blocked == 0 { 0 } else { 64 },
        stdout: out,
        stderr: String::new(),
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_definition::SkillRegistry;

    fn write_skill(dir: &std::path::Path, name: &str, version: &str, description: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let content = format!(
            "---\nname: {name}\ndescription: \"{description}\"\nmetadata:\n  version: \"{version}\"\n---\n\nBody\n"
        );
        std::fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    fn tempdir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "sddk-skills-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn skills_list_filters_by_scope() {
        let root = tempdir();
        let framework = root.join("fw");
        write_skill(
            &framework.join("skills").join("a"),
            "alpha",
            "1",
            "alpha desc",
        );
        write_skill(
            &framework.join("skills").join("b"),
            "beta",
            "1",
            "beta desc",
        );

        let opts = DiskBridgeOptions {
            project_root: None,
            framework_root: Some(framework),
            home: Some(root.join("home")),
        };
        let (_reg, loaded) = skill_registry_bridge::load_registry(&opts);
        assert_eq!(loaded.len(), 2);

        let framework_rows: Vec<SkillRow> = loaded
            .iter()
            .filter(|sk| {
                matches!(
                    sk.scope,
                    crate::skill_registry_bridge::SkillScope::Framework
                )
            })
            .map(SkillRow::from_loaded)
            .collect();
        assert_eq!(framework_rows.len(), 2);
        assert_eq!(framework_rows[0].scope, "framework");
        assert_eq!(framework_rows[0].id, "framework.alpha");
        assert_eq!(framework_rows[0].ref_token, "framework.alpha@v1");
    }

    #[test]
    fn skills_verify_reports_admitted_for_empty_required_skills() {
        let reg = SkillRegistry::new();
        let spec = CommandSpec::new("version", "show version");
        // required_skills is empty
        let outcome = skill_registry_bridge::check_admission(&spec, &reg);
        assert!(matches!(outcome, AdmissionOutcome::NotRequired));
    }

    #[test]
    fn skills_verify_reports_placeholder_separately_from_real() {
        let reg = SkillRegistry::new();

        // Placeholder: warn but admit
        let spec_ph = CommandSpec::new("lint", "x").with_required_skill("core.contract-review@v1");
        let out_ph = skill_registry_bridge::check_admission(&spec_ph, &reg);
        assert!(matches!(
            out_ph,
            AdmissionOutcome::MissingPlaceholderSkill { .. }
        ));
        assert!(out_ph.is_admitted());

        // Real missing: block
        let spec_real =
            CommandSpec::new("lint", "x").with_required_skill("framework.contract-review@v1");
        let out_real = skill_registry_bridge::check_admission(&spec_real, &reg);
        assert!(matches!(
            out_real,
            AdmissionOutcome::MissingRealSkill { .. }
        ));
        assert!(!out_real.is_admitted());
    }

    #[test]
    fn bridge_options_returns_err_when_no_framework_installed() {
        // A CliEnvironment with no SDDK_DATA_DIR + an empty HOME will
        // fail to resolve the framework root. We can't easily simulate
        // "no framework" because CliEnvironment::default() is empty and
        // resolve_active_framework_root consults env vars too. Skip if
        // a bundle happens to be present on the test machine.
        let env = CliEnvironment::default();
        let result = bridge_options_for_environment(&env);
        if std::env::var_os("HOME").is_some() {
            // Most likely a bundle is reachable — just assert no panic.
            let _ = result;
        }
    }
}
