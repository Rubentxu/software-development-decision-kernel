//! Claude Code adapter: native `.md` agent files with YAML frontmatter
//! (ADR-0019). Owns `<claude_dir>/agents` — no symlinks there.

use super::reconcile::{
    EditorCapabilities, ExistingEntry, FieldDiff, ReconcileAdapter, ReconcileContext,
    ReconcileReport, ReconcileTarget, resolve_alias_for,
};
use super::{AdapterReport, RegistrationContext, is_sddk_owned, resolve_for_models};
use crate::dev::agent_models::IdeKey;
use crate::dev::common::atomic_write;
use crate::dev::editor_adapters::AgentSource;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Claude Code model vocabulary: short aliases or full provider/model IDs.
pub(super) fn claude_model_valid(model: &str) -> bool {
    matches!(model, "sonnet" | "opus" | "haiku" | "inherit") || model.contains('/')
}

/// Claude registration: one `agents/<name>.md` per bundle agent.
pub struct ClaudeAdapter {
    pub dir: PathBuf,
}

impl super::EditorAdapter for ClaudeAdapter {
    fn editor_name(&self) -> &'static str {
        "claude"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        let mut report = AdapterReport {
            editor: "claude".to_owned(),
            ..AdapterReport::default()
        };
        let agents_dir = self.dir.join("agents");
        for agent in ctx.agents {
            let target = agents_dir.join(format!("{}.md", agent.name));
            if target.exists() {
                report.skipped_existing += 1;
                continue;
            }
            match resolve_for_models(ctx.models, &agent.name, IdeKey::Claude) {
                Ok(model) => {
                    if let Some(model) = &model
                        && !claude_model_valid(model)
                    {
                        report.errors.push(format!(
                            "agent {}: model '{model}' not in claude vocabulary \
                             (sonnet|opus|haiku|inherit or a full provider/model id)",
                            agent.name
                        ));
                        report.skipped_unresolved += 1;
                        continue;
                    }
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
                    match atomic_write(&target, content.as_bytes(), None) {
                        Ok(()) => report.registered += 1,
                        Err(error) => report.errors.push(format!("{}: {error}", target.display())),
                    }
                }
                Err(()) => report.skipped_unresolved += 1,
            }
        }
        let bundle_names: HashSet<&str> =
            ctx.agents.iter().map(|agent| agent.name.as_str()).collect();
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                if is_sddk_owned(stem) && !bundle_names.contains(stem) {
                    match std::fs::remove_file(&path) {
                        Ok(()) => report.pruned += 1,
                        Err(error) => report
                            .errors
                            .push(format!("{}: cannot prune: {error}", path.display())),
                    }
                }
            }
        }
        report
    }
}

// ── ReconcileAdapter implementation ───────────────────────────────────────────────

impl ReconcileAdapter for ClaudeAdapter {
    fn editor_name(&self) -> &'static str {
        "claude"
    }

    fn capabilities(&self) -> EditorCapabilities {
        EditorCapabilities::for_ide(IdeKey::Claude)
    }

    fn read_existing(&self, name: &str) -> Option<ExistingEntry> {
        read_claude_existing(&self.dir.join("agents"), name)
    }

    fn reconcile(&self, ctx: &ReconcileContext<'_>, apply: bool) -> ReconcileReport {
        reconcile_claude(self.dir.join("agents"), ctx, apply)
    }
}

// ── Claude reconcile helpers ───────────────────────────────────────────────────

fn read_claude_existing(agents_dir: &Path, name: &str) -> Option<ExistingEntry> {
    let path = agents_dir.join(format!("{}.md", name));
    let content = std::fs::read_to_string(&path).ok()?;
    let frontmatter = parse_frontmatter(&content)?;

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
                if !["name"].contains(&key.as_str()) {
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

/// Parse YAML frontmatter: returns ordered_frontmatter_map.
fn parse_frontmatter(content: &str) -> Option<BTreeMap<String, String>> {
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

/// Apply a name rename to a Claude .md file.
///
/// Writes the new agent file and removes the old file.
/// Reports errors via `errors` when atomic_write fails; best-effort remove of old file.
pub(crate) fn apply_rename_claude_file(
    agents_dir: &Path,
    agent: &AgentSource,
    target: &ReconcileTarget,
    existing: Option<&ExistingEntry>,
    diff: &FieldDiff,
    errors: &mut Vec<String>,
) {
    let old_name = diff
        .old_value
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let new_name = diff
        .new_value
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if old_name.is_empty() || new_name.is_empty() || old_name == new_name {
        return;
    }

    let mut new_content = String::from("---\n");
    new_content.push_str(&format!("name: {}\n", agent.name));
    new_content.push_str(&format!("description: {}\n", agent.description));
    if let Some(ref tools) = agent.tools {
        new_content.push_str(&format!("tools: {}\n", tools));
    }
    if let Some(ref model) = target.model {
        new_content.push_str(&format!("model: {}\n", model));
    }

    if let Some(ex) = existing {
        for (k, v) in &ex.extras {
            if k != "name" && k != "description" && k != "model" && k != "tools" {
                new_content.push_str(&format!("{}: {}\n", k, v));
            }
        }
    }

    new_content.push_str("---\n");
    new_content.push_str(&agent.body);

    let target_path = agents_dir.join(format!("{}.md", agent.name));
    let old_path = agents_dir.join(format!("{}.md", old_name));

    if let Err(error) = atomic_write(&target_path, new_content.as_bytes(), None) {
        errors.push(format!("{}: {error}", target_path.display()));
    } else if old_path != target_path
        && let Err(error) = std::fs::remove_file(&old_path)
    {
        errors.push(format!(
            "orphan after rename: {}: {error}",
            old_path.display()
        ));
    }
}

fn reconcile_claude(
    agents_dir: PathBuf,
    ctx: &ReconcileContext<'_>,
    apply: bool,
) -> ReconcileReport {
    use std::collections::HashSet;

    let mut report = ReconcileReport {
        editor: "claude".to_owned(),
        ..Default::default()
    };

    let capabilities = EditorCapabilities::for_ide(IdeKey::Claude);
    let bundle_names: HashSet<&str> = ctx.agents.iter().map(|a| a.name.as_str()).collect();

    // Process each bundle agent
    for agent in ctx.agents {
        let Some(target) = ctx.build_target(agent, IdeKey::Claude, &capabilities) else {
            report.mark_skipped();
            continue;
        };

        // Try canonical name first, then alias (INC-DEBT-011: alias-driven diff detection)
        let existing = resolve_alias_for(ctx.renames, &agent.name, |n| {
            read_claude_existing(&agents_dir, n)
        })
        .map(|(e, _)| e);
        let diffs = if let Some(ref ex) = existing {
            ReconcileContext::diff_existing_target(ex, &target, &capabilities)
        } else {
            vec![FieldDiff {
                field_name: "description",
                old_value: None,
                new_value: Some(serde_json::Value::String(target.description.clone())),
            }]
        };

        let changed = !diffs.is_empty();
        let result = crate::dev::editor_adapters::reconcile::AgentReconcileResult {
            name: agent.name.clone(),
            changed,
            diffs,
            errors: Vec::new(),
        };
        report.merge_agent(&result);

        if changed && apply {
            // Detect rename: FieldDiff { field_name: "name" } indicates an old_name -> new_name rename
            let name_diff = result
                .diffs
                .iter()
                .find(|d| d.field_name == "name")
                .cloned();

            let target_path = agents_dir.join(format!("{}.md", agent.name));

            if let Some(nd) = name_diff {
                // Rename path (INC-DEBT-010, cycle-36)
                apply_rename_claude_file(
                    &agents_dir,
                    agent,
                    &target,
                    existing.as_ref(),
                    &nd,
                    &mut report.errors,
                );
                continue; // skip the normal rewrite below
            }

            // Normal rewrite path (existing code, unchanged for non-rename case)
            let mut new_content = String::from("---\n");
            new_content.push_str(&format!("name: {}\n", agent.name));
            new_content.push_str(&format!("description: {}\n", agent.description));
            if let Some(ref tools) = target.tools {
                new_content.push_str(&format!("tools: {}\n", tools));
            }
            if let Some(ref model) = target.model {
                new_content.push_str(&format!("model: {}\n", model));
            }

            // Preserve extras
            if let Some(ref ex) = existing {
                for (k, v) in &ex.extras {
                    if k != "name" && k != "description" && k != "model" && k != "tools" {
                        new_content.push_str(&format!("{}: {}\n", k, v));
                    }
                }
            }

            new_content.push_str("---\n");
            new_content.push_str(&agent.body);

            if let Err(error) = atomic_write(&target_path, new_content.as_bytes(), None) {
                report
                    .errors
                    .push(format!("{}: {error}", target_path.display()));
            }
        }
    }

    // Prune: remove framework-namespaced agents not in bundle
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if is_sddk_owned(stem) && !bundle_names.contains(stem) {
                if apply && let Err(error) = std::fs::remove_file(&path) {
                    report
                        .errors
                        .push(format!("{}: cannot prune: {error}", path.display()));
                }
                report.mark_pruned();
            }
        }
    }

    report
}

#[cfg(test)]
#[path = "../tests/claude_adapter_tests.rs"]
mod claude_adapter_tests;
