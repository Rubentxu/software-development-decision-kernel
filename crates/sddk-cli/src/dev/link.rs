//! `dev link` — symlink framework assets into an editor.

use super::LinkEditor;
use crate::dev::common::walk_dir;
use crate::dev::editor_adapters::LinkProfile;
use crate::dev::framework_check::{LinkReport, link_report_text, sync_assets};
use crate::dev::paths::resolve_active_framework_root;
use crate::dev::registry::write_skill_registry;
use crate::{CliEnvironment, CommandOutput, render_result};
use std::path::{Path, PathBuf};

pub(crate) fn link_file(
    source: &Path,
    target: &Path,
    stale_replaced: &mut usize,
) -> std::io::Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(target) {
        if metadata.file_type().is_symlink() {
            if let Ok(current) = std::fs::read_link(target) {
                let source_abs =
                    std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
                let current_abs = if current.is_absolute() {
                    current
                } else {
                    target
                        .parent()
                        .unwrap_or_else(|| Path::new("/"))
                        .join(current)
                };
                let current_abs = std::fs::canonicalize(&current_abs).unwrap_or(current_abs);
                if current_abs == source_abs {
                    return Ok(());
                }
            }
            std::fs::remove_file(target)?;
        } else {
            let backup = target.with_extension("sddk-stale");
            if !backup.exists() {
                std::fs::rename(target, &backup)?;
            } else {
                std::fs::remove_file(target)?;
            }
            *stale_replaced += 1;
        }
    }
    std::fs::create_dir_all(target.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "target has no parent")
    })?)?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)?;
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(source, target)?;
    }
    Ok(())
}

/// Prune editor entries that the framework no longer ships.
pub(crate) fn prune_editor(root: &Path, editor_dir: &Path) -> usize {
    let framework_namespace = |name: &std::ffi::OsStr| {
        let name = name.to_string_lossy();
        name.starts_with("sddk-")
            || name.starts_with("sdd-")
            || name.starts_with("gentle-")
            || name == "orchestrator.md"
    };
    let mut pruned = 0usize;
    let mut prune_entry = |path: &Path, source: &Path, name: &std::ffi::OsStr| -> bool {
        let is_framework_entry = framework_namespace(name);
        let exists_in_source = source.join(name).exists();
        let is_broken_link = path
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink() && !path.exists())
            .unwrap_or(false);
        let is_stale_backup = name.to_string_lossy().ends_with(".sddk-stale");
        let should_remove =
            is_broken_link || is_stale_backup || (is_framework_entry && !exists_in_source);
        if should_remove {
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_dir_all(path);
            pruned += 1;
        }
        should_remove
    };

    let source_agents = root.join("agents");
    let target_agents = editor_dir.join("agents");
    if let Ok(entries) = std::fs::read_dir(&target_agents) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let is_md = entry.path().extension().and_then(|e| e.to_str()) == Some("md");
            if is_md {
                prune_entry(&entry.path(), &source_agents, &name);
            }
        }
    }
    let source_skills = root.join("skills");
    let target_skills = editor_dir.join("skills");
    if let Ok(entries) = std::fs::read_dir(&target_skills) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let path = entry.path();
            let is_skill = entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                || path.extension().and_then(|e| e.to_str()) == Some("md");
            if is_skill {
                prune_entry(&path, &source_skills, &name);
            }
        }
    }
    let source_prompts = root.join("prompts/sddk");
    let target_prompts = editor_dir.join("prompts/sddk");
    if let Ok(entries) = std::fs::read_dir(&target_prompts) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let is_md = entry.path().extension().and_then(|e| e.to_str()) == Some("md");
            if is_md {
                prune_entry(&entry.path(), &source_prompts, &name);
            }
        }
    }
    let source_workflows = root.join("prompts/sddk/workflows");
    let target_workflows = editor_dir.join("workflows");
    if let Ok(entries) = std::fs::read_dir(&target_workflows) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_owned);
            let is_workflow_entry = matches!(ext.as_deref(), Some("yaml") | Some("yml"));
            let is_stale_backup = name.to_string_lossy().ends_with(".sddk-stale");
            if is_workflow_entry || is_stale_backup {
                prune_entry(&path, &source_workflows, &name);
            }
        }
    }
    pruned
}

fn link_editor(root: &Path, editor_dir: &Path, profile: LinkProfile) -> LinkReport {
    let mut report = LinkReport {
        editor: editor_dir.to_string_lossy().into_owned(),
        agents_linked: 0,
        skills_linked: 0,
        prompts_linked: 0,
        workflows_linked: 0,
        stale_replaced: 0,
        pruned: 0,
        agents_registered: 0,
        agents_updated_stale: 0,
        agents_skipped_existing: 0,
        agents_skipped_unresolved: 0,
        errors: Vec::new(),
    };
    let mut stale = 0usize;

    if profile.agents {
        let agents_source = root.join("agents");
        if let Ok(entries) = std::fs::read_dir(&agents_source) {
            let agents_target = editor_dir.join("agents");
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
                    let name = entry.file_name();
                    if let Err(error) =
                        link_file(&entry.path(), &agents_target.join(&name), &mut stale)
                    {
                        report.errors.push(format!("agents/{name:?}: {error}"));
                    } else {
                        report.agents_linked += 1;
                    }
                }
            }
        }
    }

    if profile.skills {
        let skills_source = root.join("skills");
        if let Ok(entries) = std::fs::read_dir(&skills_source) {
            let skills_target = editor_dir.join("skills");
            for entry in entries.flatten() {
                let path = entry.path();
                let is_skill_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let is_markdown = path.extension().and_then(|e| e.to_str()) == Some("md");
                if is_skill_dir || is_markdown {
                    let name = entry.file_name();
                    if let Err(error) = link_file(&path, &skills_target.join(&name), &mut stale) {
                        report.errors.push(format!("skills/{name:?}: {error}"));
                    } else {
                        report.skills_linked += 1;
                    }
                }
            }
        }
    }

    if profile.prompts {
        let prompts_source = root.join("prompts/sddk");
        let prompts_target = editor_dir.join("prompts/sddk");
        if prompts_source.is_dir() {
            for entry in walk_dir(&prompts_source) {
                if entry.is_file() {
                    let relative = entry
                        .strip_prefix(&prompts_source)
                        .unwrap_or(entry.as_path());
                    let target = prompts_target.join(relative);
                    if let Err(error) = link_file(&entry, &target, &mut stale) {
                        report.errors.push(format!("prompts/{relative:?}: {error}"));
                    } else {
                        report.prompts_linked += 1;
                    }
                }
            }
        }
    }

    if profile.workflows {
        let workflows_source = root.join("prompts/sddk/workflows");
        let workflows_target = editor_dir.join("workflows");
        if workflows_source.is_dir() {
            for entry in walk_dir(&workflows_source) {
                if entry.is_file()
                    && let Some(name) = entry
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_owned)
                {
                    if let Err(error) = link_file(&entry, &workflows_target.join(&name), &mut stale)
                    {
                        report.errors.push(format!("workflows/{name:?}: {error}"));
                    } else {
                        report.workflows_linked += 1;
                    }
                }
            }
        }
    }

    report.stale_replaced = stale;
    report.pruned = prune_editor(root, editor_dir);
    report
}

// ── Public subcommand ──────────────────────────────────────────────────────────

pub(super) fn run_dev_link(args: super::LinkArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let mut warnings: Vec<String> = Vec::new();
    let result = (|| -> anyhow::Result<Vec<LinkReport>> {
        let root = if args.root.as_os_str() == "." {
            resolve_active_framework_root(environment)?
        } else {
            std::fs::canonicalize(&args.root)?
        };
        // Dogfooding: sync assets and regenerate manifest.
        if args.root.as_os_str() != "."
            && let Ok(framework_root) = resolve_active_framework_root(environment)
            && root.join("assets").is_dir()
        {
            let _ = sync_assets(&root.join("assets"), &framework_root.join("assets"));
            let _ = super::manifest::write_manifest(&framework_root);
        }
        // Agent→model config (Option — absence is not an error).
        let models_path = root.join("assets").join("agent-models.yaml");
        let models = match crate::dev::agent_models::AgentModelsConfig::from_file(&models_path) {
            Ok(models) => models,
            Err(error) => {
                warnings.push(format!("warning: {error}"));
                None
            }
        };
        if models.is_none() {
            warnings.push(format!(
                "warning: agent-models.yaml not found at {}; agents will be registered without a model",
                models_path.display()
            ));
        }
        // One agent-source pass shared by all adapters.
        let agents = crate::dev::editor_adapters::load_agent_sources(&root);
        let ctx = crate::dev::editor_adapters::RegistrationContext {
            root: &root,
            agents: &agents,
            models: models.as_ref(),
        };
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let opencode_dir = args
            .opencode_dir
            .clone()
            .unwrap_or_else(|| home.join(".config/opencode"));
        let zcode_dir = args
            .zcode_dir
            .clone()
            .unwrap_or_else(|| home.join(".zcode"));
        let claude_dir = args
            .claude_dir
            .clone()
            .unwrap_or_else(|| home.join(".claude"));
        let codex_dir = args
            .codex_dir
            .clone()
            .unwrap_or_else(|| home.join(".codex"));
        let dirs = crate::dev::editor_adapters::EditorDirs {
            opencode: opencode_dir.clone(),
            zcode: zcode_dir.clone(),
            claude: claude_dir.clone(),
            codex: codex_dir.clone(),
        };
        let mut reports = Vec::new();
        for editor in [
            LinkEditor::OpenCode,
            LinkEditor::ZCode,
            LinkEditor::Claude,
            LinkEditor::Codex,
        ] {
            if !matches!(args.editor, LinkEditor::All) && args.editor != editor {
                continue;
            }
            let editor_dir = match editor {
                LinkEditor::OpenCode => &opencode_dir,
                LinkEditor::ZCode => &zcode_dir,
                LinkEditor::Claude => &claude_dir,
                LinkEditor::Codex => &codex_dir,
                LinkEditor::All => unreachable!("single-editor loop"),
            };
            let profile = crate::dev::editor_adapters::LinkProfile::for_editor(editor);
            let mut report = link_editor(&root, editor_dir, profile);
            register_into_report(editor, &dirs, &ctx, &mut report, &mut warnings);
            reports.push(report);
        }

        // Write idempotent skill registry.
        let project_root_for_registry =
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if args.write_registry {
            match write_skill_registry(environment, &project_root_for_registry, &root) {
                Ok((path, count)) => {
                    eprintln!(
                        "skill registry: {} entries written to {}",
                        count,
                        path.display()
                    )
                }
                Err(error) => {
                    eprintln!("warning: skill registry write failed: {error}");
                }
            }
        }

        Ok(reports)
    })();
    let has_errors = result
        .as_ref()
        .map(|reports: &Vec<LinkReport>| reports.iter().any(|report| !report.errors.is_empty()))
        .unwrap_or(true);
    let mut output = render_result(result, format, |reports: &Vec<LinkReport>| {
        let mut text = String::new();
        for report in reports {
            text.push_str(&link_report_text(report));
        }
        text
    });
    // PerEditorReporting: any editor failure yields a non-zero exit while all
    // editors still report their outcome.
    if has_errors {
        output.status = 1;
    }
    if !warnings.is_empty() {
        output.stderr = format!("{}\n", warnings.join("\n"));
    }
    output
}

/// Run the selected editor's adapters and merge their reports into the
/// editor's `LinkReport` (PerIdeErrorIsolation: per-editor errors are
/// captured and reported without aborting the other editors).
fn register_into_report(
    editor: super::LinkEditor,
    dirs: &crate::dev::editor_adapters::EditorDirs,
    ctx: &crate::dev::editor_adapters::RegistrationContext<'_>,
    report: &mut LinkReport,
    warnings: &mut Vec<String>,
) {
    use crate::dev::agent_models::IdeKey;
    use crate::dev::editor_adapters::resolve_for_models;
    let ide = match editor {
        super::LinkEditor::OpenCode => IdeKey::Opencode,
        super::LinkEditor::ZCode => IdeKey::Zcode,
        super::LinkEditor::Claude => IdeKey::Claude,
        super::LinkEditor::Codex => IdeKey::Codex,
        super::LinkEditor::All => unreachable!("single-editor dispatch"),
    };
    for adapter in crate::dev::editor_adapters::adapters_for(editor, dirs) {
        let adapter_report = adapter.register(ctx);
        report.agents_registered = adapter_report.registered;
        report.agents_updated_stale = adapter_report.updated_stale;
        report.agents_skipped_existing = adapter_report.skipped_existing;
        report.agents_skipped_unresolved = adapter_report.skipped_unresolved;
        report.errors.extend(adapter_report.errors);
        if adapter_report.registered > 0 {
            warnings.push(format!(
                "{}: registered {} framework agents",
                adapter.editor_name(),
                adapter_report.registered
            ));
        }
        if adapter_report.updated_stale > 0 {
            warnings.push(format!(
                "{}: refreshed {} stale agent paths (framework root changed)",
                adapter.editor_name(),
                adapter_report.updated_stale
            ));
        }
        if report.agents_skipped_unresolved > 0 {
            let names: Vec<String> = ctx
                .agents
                .iter()
                .filter(|agent| resolve_for_models(ctx.models, &agent.name, ide).is_err())
                .map(|agent| agent.name.clone())
                .collect();
            warnings.push(format!(
                "warning: {}: {} agents skipped (no model configured): {}",
                adapter_report.editor,
                report.agents_skipped_unresolved,
                names.join(", ")
            ));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/reconciliation_tests.rs"]
mod reconciliation_tests;

#[cfg(test)]
#[path = "tests/link_e2e_tests.rs"]
mod link_e2e_tests;
