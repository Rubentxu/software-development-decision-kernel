//! ZCode adapter: native `.md` subagent files + primary-agent command files
//! (ADR-0081). Owns `<zcode_dir>/agents` — no symlinks there.
//!
//! ZCode discovers user subagents as markdown files under
//! `<zcode_dir>/agents/<name>.md` whose YAML frontmatter MUST carry `name` and
//! `description` (files missing either are silently ignored by the runtime);
//! keys are camelCase and the body is the inline system prompt. There is no
//! `{file:...}` prompt-reference mechanism and no `mode: primary` concept:
//! the main agent is fixed and subagents cannot spawn further subagents.
//!
//! Bundle agents marked primary (`PRIMARY_AGENTS`) are therefore registered as
//! slash commands under `<zcode_dir>/commands/<name>.md` instead: invoking the
//! command injects the agent prompt into the main agent, which CAN dispatch
//! the sddk-* sub-agents via the Agent tool — restoring OpenCode primary-mode
//! semantics on ZCode.

use super::reconcile::{
    EditorCapabilities, ExistingEntry, FieldDiff, ReconcileAdapter, ReconcileContext,
    ReconcileReport, resolve_alias_for,
};
use super::{AdapterReport, RegistrationContext, is_sddk_owned, resolve_for_models};
use crate::dev::agent_models::IdeKey;
use crate::dev::common::atomic_write;
use crate::dev::editor_adapters::AgentSource;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Frontmatter marker written into every generated command file. ZCode's flat
/// command parser ignores unknown keys, and prune/uninstall only remove marker
/// files so hand-written commands sharing a framework namespace survive.
const COMMAND_MARKER: &str = "source: sddk";

/// Preamble prepended to primary-agent command bodies: the prompt was written
/// for an OpenCode primary agent, so re-state the dispatch contract for the
/// ZCode main agent.
const COMMAND_PREAMBLE: &str = "> Registered by SDDK. You are running as the ZCode main agent: \
dispatch sddk-* sub-agents with the Agent tool and never do worker work inline.\n\n";

/// ZCode model vocabulary: full `provider/model` ids as configured in the app
/// (e.g. `deepseek/deepseek-chat`). ZCode has no inherit keyword — omitting
/// the `model` field inherits the primary agent's model.
pub(super) fn zcode_model_valid(model: &str) -> bool {
    model.contains('/')
}

/// ZCode registration: `agents/<name>.md` per bundle sub-agent, plus
/// `commands/<name>.md` for primary agents.
pub struct ZCodeAdapter {
    pub dir: PathBuf,
}

impl ZCodeAdapter {
    fn agents_dir(&self) -> PathBuf {
        self.dir.join("agents")
    }

    fn commands_dir(&self) -> PathBuf {
        self.dir.join("commands")
    }
}

impl super::EditorAdapter for ZCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "zcode"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        let mut report = AdapterReport {
            editor: "zcode".to_owned(),
            ..AdapterReport::default()
        };
        let agents_dir = self.agents_dir();
        let commands_dir = self.commands_dir();
        for agent in ctx.agents {
            let resolved = match resolve_for_models(ctx.models, &agent.name, IdeKey::Zcode) {
                Ok(model) => model,
                Err(()) => {
                    report.skipped_unresolved += 1;
                    continue;
                }
            };
            if let Some(model) = &resolved
                && !zcode_model_valid(model)
            {
                report.errors.push(format!(
                    "agent {}: model '{model}' not in zcode vocabulary \
                     (expected a full provider/model id)",
                    agent.name
                ));
                report.skipped_unresolved += 1;
                continue;
            }
            if super::PRIMARY_AGENTS.contains(&agent.name.as_str()) {
                register_primary(
                    &agents_dir,
                    &commands_dir,
                    agent,
                    resolved.as_ref(),
                    &mut report,
                );
            } else {
                register_subagent(&agents_dir, agent, resolved.as_ref(), &mut report);
            }
        }
        prune_stale_artifacts(&agents_dir, &commands_dir, ctx, &mut report);
        report
    }
}

/// Registers one primary agent as a command file. Legacy artifacts from the
/// agent-map era (symlinks or `name`-less md files under `agents/`) are
/// removed: the agent is now reachable only through the command.
fn register_primary(
    agents_dir: &Path,
    commands_dir: &Path,
    agent: &AgentSource,
    model: Option<&String>,
    report: &mut AdapterReport,
) {
    let legacy = agents_dir.join(format!("{}.md", agent.name));
    if is_managed_legacy_artifact(&legacy, &agent.name)
        && let Err(error) = std::fs::remove_file(&legacy)
    {
        report
            .errors
            .push(format!("{}: cannot remove: {error}", legacy.display()));
    }
    let target = commands_dir.join(format!("{}.md", agent.name));
    if target.exists() {
        report.skipped_existing += 1;
        return;
    }
    let content = render_command(agent, model);
    match atomic_write(&target, content.as_bytes(), None) {
        Ok(()) => report.registered += 1,
        Err(error) => report.errors.push(format!("{}: {error}", target.display())),
    }
}

/// Registers one sub-agent as a native `agents/<name>.md` file, replacing
/// agent-map era symlinks and `name`-less stale writes in place.
fn register_subagent(
    agents_dir: &Path,
    agent: &AgentSource,
    model: Option<&String>,
    report: &mut AdapterReport,
) {
    let target = agents_dir.join(format!("{}.md", agent.name));
    if let Ok(meta) = std::fs::symlink_metadata(&target) {
        // Native files carry the REQUIRED `name` frontmatter and are left
        // untouched (first-time-only). Symlinks are always ours to replace
        // (users never symlink into the framework's namespace), while
        // `name`-less stale regular files count as ours only for sddk-owned
        // names — most bundle agents lack the sddk- prefix (debt-*, uat-*,
        // studio-*, jd-*…), so ownership cannot hinge on the prefix alone.
        let native = meta.is_file()
            && std::fs::read_to_string(&target)
                .ok()
                .and_then(|content| frontmatter(&content))
                .is_some_and(|fm| fm.contains_key("name"));
        let ours = meta.is_symlink() || is_sddk_owned(&agent.name);
        if native || !ours {
            report.skipped_existing += 1;
            return;
        }
        report.updated_stale += 1;
    } else {
        report.registered += 1;
    }
    let content = render_subagent(agent, model);
    if let Err(error) = atomic_write(&target, content.as_bytes(), None) {
        report.errors.push(format!("{}: {error}", target.display()));
    }
}

/// Removes sddk-owned leftovers: bundle-orphan agent files under `agents/`
/// (the agents dir namespace is bounded by ADR-0018) and marker commands no
/// longer backed by a primary bundle agent. Non-marker command files are
/// never touched.
fn prune_stale_artifacts(
    agents_dir: &Path,
    commands_dir: &Path,
    ctx: &RegistrationContext<'_>,
    report: &mut AdapterReport,
) {
    let bundle_names: HashSet<&str> = ctx.agents.iter().map(|a| a.name.as_str()).collect();
    let root_canon = std::fs::canonicalize(ctx.root).unwrap_or_else(|_| ctx.root.to_path_buf());
    // A symlink under agents/ that resolves into the framework root is ours
    // regardless of the agent name (most bundle agents lack the sddk- prefix).
    let links_into_framework = |path: &Path| -> bool {
        path.is_symlink()
            && std::fs::read_link(path)
                .ok()
                .map(|target| {
                    let absolute = if target.is_absolute() {
                        target
                    } else {
                        path.parent()
                            .map(|parent| parent.join(&target))
                            .unwrap_or(target)
                    };
                    std::fs::canonicalize(absolute)
                        .map(|resolved| resolved.starts_with(&root_canon))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
    };
    if let Ok(entries) = std::fs::read_dir(agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let ours = is_sddk_owned(stem) || links_into_framework(&path);
            if ours && !bundle_names.contains(stem) {
                match std::fs::remove_file(&path) {
                    Ok(()) => report.pruned += 1,
                    Err(error) => report
                        .errors
                        .push(format!("{}: cannot prune: {error}", path.display())),
                }
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(commands_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let is_marker_command = is_sddk_command(&path);
            let still_primary = is_sddk_owned(stem)
                && super::PRIMARY_AGENTS.contains(&stem)
                && bundle_names.contains(stem);
            if is_marker_command && !still_primary {
                match std::fs::remove_file(&path) {
                    Ok(()) => report.pruned += 1,
                    Err(error) => report
                        .errors
                        .push(format!("{}: cannot prune: {error}", path.display())),
                }
            }
        }
    }
}

/// True when `path` is an artifact of the agent-map era: a symlink into the
/// framework bundle, or an sddk-owned md file missing the required `name:`
/// frontmatter (both are always ours to replace).
fn is_managed_legacy_artifact(path: &Path, name: &str) -> bool {
    if path.is_symlink() {
        return true;
    }
    if !path.is_file() || !is_sddk_owned(name) {
        return false;
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| frontmatter(&content))
        .is_none_or(|fm| !fm.contains_key("name"))
}

/// Renders a native ZCode subagent file: `name`/`description` are REQUIRED by
/// the runtime; `tools` passes through bundle frontmatter when declared; the
/// body is the inline system prompt.
fn render_subagent(agent: &AgentSource, model: Option<&String>) -> String {
    let mut content = format!(
        "---\nname: {}\ndescription: {}\n",
        agent.name, agent.description
    );
    if let Some(tools) = &agent.tools {
        content.push_str(&format!("tools: {tools}\n"));
    }
    if let Some(model) = model {
        content.push_str(&format!("model: {model}\n"));
    }
    content.push_str("---\n");
    content.push_str(&agent.body);
    content
}

/// Renders a primary-agent command file. The `source: sddk` marker bounds
/// prune/uninstall; `model` (optional) switches the main agent's model while
/// running the command.
fn render_command(agent: &AgentSource, model: Option<&String>) -> String {
    let mut content = format!(
        "---\ndescription: {}\nargument-hint: <goal>\n{COMMAND_MARKER}\n",
        agent.description
    );
    if let Some(model) = model {
        content.push_str(&format!("model: {model}\n"));
    }
    content.push_str("---\n");
    content.push_str(COMMAND_PREAMBLE);
    content.push_str(&agent.body);
    content
}

/// True when `path` is a command file generated by this adapter (identified
/// by the `source: sddk` frontmatter marker; hand-written commands are never
/// matched). Shared with `uninstall.rs`.
pub(crate) fn is_sddk_command(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| frontmatter(&content))
        .is_some_and(|fm| fm.get("source").map(|v| v == "sddk").unwrap_or(false))
}

/// Parses a flat frontmatter block into ordered key → value pairs (single-line
/// top-level keys only, mirroring ZCode's own parser).
fn frontmatter(content: &str) -> Option<BTreeMap<String, String>> {
    let rest = content.strip_prefix("---")?;
    let (block, _body) = rest.split_once("---")?;
    let mut frontmatter = BTreeMap::new();
    for line in block.lines() {
        if let Some((key, val)) = line.split_once(':') {
            frontmatter.insert(key.trim().to_owned(), val.trim().to_owned());
        }
    }
    Some(frontmatter)
}

// ── ReconcileAdapter implementation ───────────────────────────────────────────────

impl ReconcileAdapter for ZCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "zcode"
    }

    fn capabilities(&self) -> EditorCapabilities {
        EditorCapabilities::for_ide(IdeKey::Zcode)
    }

    fn read_existing(&self, name: &str) -> Option<ExistingEntry> {
        read_zcode_existing(&self.agents_dir(), &self.commands_dir(), name)
    }

    fn reconcile(&self, ctx: &ReconcileContext<'_>, apply: bool) -> ReconcileReport {
        reconcile_zcode(self.agents_dir(), self.commands_dir(), ctx, apply)
    }
}

/// Reads the existing state of one agent: native md under `agents/` for
/// sub-agents, command file under `commands/` for primary agents.
fn read_zcode_existing(
    agents_dir: &Path,
    commands_dir: &Path,
    name: &str,
) -> Option<ExistingEntry> {
    let primary = super::PRIMARY_AGENTS.contains(&name);
    let path = if primary {
        commands_dir.join(format!("{name}.md"))
    } else {
        agents_dir.join(format!("{name}.md"))
    };
    let content = std::fs::read_to_string(&path).ok()?;
    let frontmatter = frontmatter(&content)?;

    let mut extras = BTreeMap::new();
    let mut description = None;
    let mut model = None;
    let mut tools = None;

    for (key, val) in &frontmatter {
        match key.as_str() {
            "description" => description = Some(val.trim().trim_matches('"').to_owned()),
            "model" => model = Some(val.trim().trim_matches('"').to_owned()),
            "tools" => tools = Some(val.trim().trim_matches('"').to_owned()),
            _ => {
                if key != "name"
                    && key != COMMAND_MARKER
                    && key != "source"
                    && key != "argument-hint"
                {
                    extras.insert(
                        key.clone(),
                        serde_json::Value::String(val.trim().to_owned()),
                    );
                }
            }
        }
    }

    Some(ExistingEntry {
        name: name.to_owned(),
        description,
        model,
        mode: None,
        hidden: None,
        prompt: None,
        tools,
        extras,
    })
}

fn reconcile_zcode(
    agents_dir: PathBuf,
    commands_dir: PathBuf,
    ctx: &ReconcileContext<'_>,
    apply: bool,
) -> ReconcileReport {
    let mut report = ReconcileReport {
        editor: "zcode".to_owned(),
        ..Default::default()
    };

    for agent in ctx.agents {
        let model = match resolve_for_models(ctx.models, &agent.name, IdeKey::Zcode) {
            Ok(model) => model,
            Err(()) => {
                report.mark_skipped();
                continue;
            }
        };
        let primary = super::PRIMARY_AGENTS.contains(&agent.name.as_str());
        let target_path = if primary {
            commands_dir.join(format!("{}.md", agent.name))
        } else {
            agents_dir.join(format!("{}.md", agent.name))
        };

        // Try canonical name first, then alias (INC-DEBT-011: alias-driven
        // diff detection). Stale agent-map artifacts under `agents/` count as
        // absent so they are rewritten in place with the native format;
        // command files never carry `name:` so the check is agents-only.
        let (existing, alias_name) = resolve_alias_for(ctx.renames, &agent.name, |n| {
            read_zcode_existing(&agents_dir, &commands_dir, n)
        })
        .filter(|(_, found)| {
            if primary {
                true
            } else {
                let found_path = agents_dir.join(format!("{found}.md"));
                !is_managed_legacy_artifact(&found_path, found)
            }
        })
        .map(|(e, found)| (Some(e), Some(found)))
        .unwrap_or((None, None));

        let mut diffs = if let Some(ref ex) = existing {
            let mut diffs = Vec::new();
            if ex.description.as_ref() != Some(&agent.description) {
                diffs.push(FieldDiff {
                    field_name: "description",
                    old_value: ex
                        .description
                        .as_ref()
                        .map(|s| serde_json::Value::String(s.clone())),
                    new_value: Some(serde_json::Value::String(agent.description.clone())),
                });
            }
            if ex.model.as_ref() != model.as_ref() {
                diffs.push(FieldDiff {
                    field_name: "model",
                    old_value: ex
                        .model
                        .as_ref()
                        .map(|s| serde_json::Value::String(s.clone())),
                    new_value: model.as_ref().map(|s| serde_json::Value::String(s.clone())),
                });
            }
            if !primary && ex.tools.as_ref() != agent.tools.as_ref() {
                diffs.push(FieldDiff {
                    field_name: "tools",
                    old_value: ex
                        .tools
                        .as_ref()
                        .map(|s| serde_json::Value::String(s.clone())),
                    new_value: agent
                        .tools
                        .as_ref()
                        .map(|s| serde_json::Value::String(s.clone())),
                });
            }
            diffs
        } else {
            vec![FieldDiff {
                field_name: "description",
                old_value: None,
                new_value: Some(serde_json::Value::String(agent.description.clone())),
            }]
        };
        let renamed = alias_name
            .as_ref()
            .is_some_and(|found| *found != agent.name);
        if renamed {
            diffs.push(FieldDiff {
                field_name: "name",
                old_value: alias_name
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
                new_value: Some(serde_json::Value::String(agent.name.clone())),
            });
        }

        let changed = !diffs.is_empty();
        let result = crate::dev::editor_adapters::reconcile::AgentReconcileResult {
            name: agent.name.clone(),
            changed,
            diffs,
            errors: Vec::new(),
        };
        report.merge_agent(&result);

        if changed && apply {
            let content = if primary {
                render_command(agent, model.as_ref())
            } else {
                render_subagent(agent, model.as_ref())
            };
            if let Err(error) = atomic_write(&target_path, content.as_bytes(), None) {
                report
                    .errors
                    .push(format!("{}: {error}", target_path.display()));
            }
            // Alias-driven entry: the file under the old name is now an orphan.
            if renamed && let Some(found) = alias_name.as_deref() {
                let old_path = if primary {
                    commands_dir.join(format!("{found}.md"))
                } else {
                    agents_dir.join(format!("{found}.md"))
                };
                if let Err(error) = std::fs::remove_file(&old_path) {
                    report.errors.push(format!(
                        "orphan after rename: {}: {error}",
                        old_path.display()
                    ));
                }
            }
        }
    }

    // Prune: remove sddk-owned artifacts no longer backed by the bundle.
    if apply {
        let registration_ctx = RegistrationContext {
            root: ctx.root,
            agents: ctx.agents,
            models: ctx.models,
        };
        let mut register_report = AdapterReport {
            editor: "zcode".to_owned(),
            ..AdapterReport::default()
        };
        prune_stale_artifacts(
            &agents_dir,
            &commands_dir,
            &registration_ctx,
            &mut register_report,
        );
        report.agents_pruned += register_report.pruned;
        report.errors.extend(register_report.errors);
    }

    report
}

#[cfg(test)]
#[path = "../tests/zcode_adapter_tests.rs"]
mod zcode_adapter_tests;
