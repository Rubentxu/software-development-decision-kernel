//! Codex adapter: native TOML agent files in `<codex_dir>/agents/`
//! (ADR-0019). Body translation: markdown body → `developer_instructions`.
//! Fields the framework does not model (e.g. `model_reasoning_effort`,
//! `model_reasoning_summary`) are deliberately not written — documented in
//! docs/adr/ADR-0019 and the apply notes.

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

/// Codex registration: one `agents/<name>.toml` per bundle agent.
pub struct CodexAdapter {
    pub dir: PathBuf,
}

impl CodexAdapter {
    fn to_toml(agent: &super::AgentSource, model: Option<String>) -> anyhow::Result<String> {
        let mut table = toml::map::Map::new();
        table.insert("name".to_owned(), toml::Value::String(agent.name.clone()));
        table.insert(
            "description".to_owned(),
            toml::Value::String(agent.description.clone()),
        );
        table.insert(
            "developer_instructions".to_owned(),
            toml::Value::String(agent.body.clone()),
        );
        if let Some(model) = model {
            table.insert("model".to_owned(), toml::Value::String(model));
        }
        Ok(toml::to_string_pretty(&toml::Value::Table(table))?)
    }
}

impl super::EditorAdapter for CodexAdapter {
    fn editor_name(&self) -> &'static str {
        "codex"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        let mut report = AdapterReport {
            editor: "codex".to_owned(),
            ..AdapterReport::default()
        };
        let agents_dir = self.dir.join("agents");
        for agent in ctx.agents {
            let target = agents_dir.join(format!("{}.toml", agent.name));
            if target.exists() {
                report.skipped_existing += 1;
                continue;
            }
            match resolve_for_models(ctx.models, &agent.name, IdeKey::Codex) {
                Ok(model) => match Self::to_toml(agent, model) {
                    Ok(serialized) => match atomic_write(&target, serialized.as_bytes(), None) {
                        Ok(()) => report.registered += 1,
                        Err(error) => report.errors.push(format!("{}: {error}", target.display())),
                    },
                    Err(error) => report
                        .errors
                        .push(format!("{}: serialization failed: {error}", agent.name)),
                },
                Err(()) => report.skipped_unresolved += 1,
            }
        }
        let bundle_names: HashSet<&str> =
            ctx.agents.iter().map(|agent| agent.name.as_str()).collect();
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
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

impl ReconcileAdapter for CodexAdapter {
    fn editor_name(&self) -> &'static str {
        "codex"
    }

    fn capabilities(&self) -> EditorCapabilities {
        EditorCapabilities::for_ide(IdeKey::Codex)
    }

    fn read_existing(&self, name: &str) -> Option<ExistingEntry> {
        read_codex_existing(&self.dir.join("agents"), name)
    }

    fn reconcile(&self, ctx: &ReconcileContext<'_>, apply: bool) -> ReconcileReport {
        reconcile_codex(self.dir.join("agents"), ctx, apply)
    }
}

// ── Codex reconcile helpers ───────────────────────────────────────────────────

fn read_codex_existing(agents_dir: &Path, name: &str) -> Option<ExistingEntry> {
    let path = agents_dir.join(format!("{}.toml", name));
    let content = std::fs::read_to_string(&path).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;

    let table = value.as_table()?;

    // Known sddk-managed keys
    let known = &["name", "description", "developer_instructions", "model"];

    let mut extras = BTreeMap::new();
    for (k, v) in table.iter() {
        if !known.contains(&k.as_str()) {
            extras.insert(
                k.clone(),
                serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
            );
        }
    }

    Some(ExistingEntry {
        name: name.to_owned(),
        description: table
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
        model: table
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from),
        mode: None,
        hidden: None,
        prompt: None,
        tools: None,
        extras,
    })
}

/// Apply a name rename to a Codex .toml file.
///
/// Writes the new agent file and removes the old file.
/// Reports errors via `errors` when serialization or atomic_write fails.
pub(crate) fn apply_rename_codex_file(
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

    let mut table = toml::map::Map::new();
    table.insert("name".to_owned(), toml::Value::String(agent.name.clone()));
    table.insert(
        "description".to_owned(),
        toml::Value::String(agent.description.clone()),
    );
    table.insert(
        "developer_instructions".to_owned(),
        toml::Value::String(agent.body.clone()),
    );
    if let Some(ref model) = target.model {
        let model_str = model.clone();
        table.insert("model".to_owned(), toml::Value::String(model_str));
    }

    if let Some(ex) = existing {
        for (k, v) in &ex.extras {
            if let Ok(tv) = serde_json::from_value(v.clone()) {
                table.insert(k.clone(), tv);
            }
        }
    }

    let serialized = match toml::to_string_pretty(&toml::Value::Table(table)) {
        Ok(s) => s,
        Err(error) => {
            errors.push(format!("{}: serialization failed: {error}", agent.name));
            return;
        }
    };

    let target_path = agents_dir.join(format!("{}.toml", agent.name));
    let old_path = agents_dir.join(format!("{}.toml", old_name));

    if let Err(error) = atomic_write(&target_path, serialized.as_bytes(), None) {
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

fn reconcile_codex(
    agents_dir: PathBuf,
    ctx: &ReconcileContext<'_>,
    apply: bool,
) -> ReconcileReport {
    use std::collections::HashSet;

    let mut report = ReconcileReport {
        editor: "codex".to_owned(),
        ..Default::default()
    };

    let capabilities = EditorCapabilities::for_ide(IdeKey::Codex);
    let bundle_names: HashSet<&str> = ctx.agents.iter().map(|a| a.name.as_str()).collect();

    // Process each bundle agent
    for agent in ctx.agents {
        let Some(target) = ctx.build_target(agent, IdeKey::Codex, &capabilities) else {
            report.mark_skipped();
            continue;
        };

        // Try canonical name first, then alias (INC-DEBT-011: alias-driven diff detection)
        let existing = resolve_alias_for(ctx.renames, &agent.name, |n| {
            read_codex_existing(&agents_dir, n)
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

            let target_path = agents_dir.join(format!("{}.toml", agent.name));

            if let Some(nd) = name_diff {
                // Rename path (INC-DEBT-010, cycle-36)
                apply_rename_codex_file(
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
            let mut table = toml::map::Map::new();
            table.insert("name".to_owned(), toml::Value::String(agent.name.clone()));
            table.insert(
                "description".to_owned(),
                toml::Value::String(agent.description.clone()),
            );
            table.insert(
                "developer_instructions".to_owned(),
                toml::Value::String(agent.body.clone()),
            );
            if let Some(ref model) = target.model {
                table.insert("model".to_owned(), toml::Value::String(model.clone()));
            }

            // Preserve extras
            if let Some(ref ex) = existing {
                for (k, v) in &ex.extras {
                    if let Ok(tv) = serde_json::from_value(v.clone()) {
                        table.insert(k.clone(), tv);
                    }
                }
            }

            let serialized = match toml::to_string_pretty(&toml::Value::Table(table.clone())) {
                Ok(s) => s,
                Err(error) => {
                    report
                        .errors
                        .push(format!("{}: serialization failed: {error}", agent.name));
                    continue;
                }
            };

            if let Err(error) = atomic_write(&target_path, serialized.as_bytes(), None) {
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
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
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
#[path = "../tests/codex_adapter_tests.rs"]
mod codex_adapter_tests;
