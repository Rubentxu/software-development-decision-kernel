//! JSON agent-map registration core shared by opencode and zcode (ADR-0019).
//! Both editors store agents in `<dir>/<editor>.json` → `agent` map with the
//! same entry schema; the core is parameterized by `IdeKey`.

use super::reconcile::{
    EditorCapabilities, ExistingEntry, FieldDiff, ReconcileAdapter, ReconcileContext,
    ReconcileReport, resolve_alias_for,
};
use super::{AdapterReport, RegistrationContext, is_sddk_owned, resolve_for_models};
use crate::dev::agent_models::IdeKey;
use crate::dev::common::atomic_write;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// OpenCode registration: `opencode.json` agent map upsert + bounded prune.
pub struct OpenCodeAdapter {
    pub dir: PathBuf,
}

/// ZCode registration: `zcode.json` — mirrors the opencode schema.
pub struct ZCodeAdapter {
    pub dir: PathBuf,
}

impl super::EditorAdapter for OpenCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "opencode"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        upsert_json_agents(
            &self.dir.join("opencode.json"),
            IdeKey::Opencode,
            &super::PRIMARY_AGENTS,
            ctx,
        )
    }
}

impl super::EditorAdapter for ZCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "zcode"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        upsert_json_agents(
            &self.dir.join("zcode.json"),
            IdeKey::Zcode,
            &super::PRIMARY_AGENTS,
            ctx,
        )
    }
}

// ── ReconcileAdapter implementations ────────────────────────────────────────────

impl ReconcileAdapter for OpenCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "opencode"
    }

    fn capabilities(&self) -> EditorCapabilities {
        EditorCapabilities::for_ide(IdeKey::Opencode)
    }

    fn read_existing(&self, name: &str) -> Option<ExistingEntry> {
        read_json_existing(&self.dir.join("opencode.json"), name)
    }

    fn reconcile(&self, ctx: &ReconcileContext<'_>, apply: bool) -> ReconcileReport {
        reconcile_json(self.dir.join("opencode.json"), IdeKey::Opencode, ctx, apply)
    }
}

impl ReconcileAdapter for ZCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "zcode"
    }

    fn capabilities(&self) -> EditorCapabilities {
        EditorCapabilities::for_ide(IdeKey::Zcode)
    }

    fn read_existing(&self, name: &str) -> Option<ExistingEntry> {
        read_json_existing(&self.dir.join("zcode.json"), name)
    }

    fn reconcile(&self, ctx: &ReconcileContext<'_>, apply: bool) -> ReconcileReport {
        reconcile_json(self.dir.join("zcode.json"), IdeKey::Zcode, ctx, apply)
    }
}

// ── JSON reconcile helpers ────────────────────────────────────────────────────

fn read_json_existing(config_path: &Path, name: &str) -> Option<ExistingEntry> {
    let raw = std::fs::read_to_string(config_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;

    let agents = value.get("agent")?.as_object()?;
    let entry = agents.get(name)?.as_object()?;

    // Known sddk-managed keys
    let known = &["description", "model", "mode", "hidden", "prompt", "tools"];

    // Collect extras (keys not in known set)
    let mut extras = BTreeMap::new();
    for (k, v) in entry.iter() {
        if !known.contains(&k.as_str()) {
            extras.insert(k.clone(), v.clone());
        }
    }

    Some(ExistingEntry {
        name: name.to_owned(),
        description: entry
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
        model: entry
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from),
        mode: entry.get("mode").and_then(|v| v.as_str()).map(String::from),
        hidden: entry.get("hidden").and_then(|v| v.as_bool()),
        prompt: entry
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(String::from),
        tools: entry
            .get("tools")
            .and_then(|v| v.as_str())
            .map(String::from),
        extras,
    })
}

/// Apply a name rename to a JSON agent map.
///
/// Extracts `old_name → new_name` from `diff` and moves the map entry.
/// Reports collision errors via `errors` when `new_name` already exists.
pub(crate) fn apply_rename_in_agents_map(
    agents: &mut serde_json::Map<String, serde_json::Value>,
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

    if agents.contains_key(new_name) {
        errors.push(format!(
            "rename collision: target key '{new_name}' already exists; skipping rename from '{old_name}'"
        ));
    } else if let Some(mut value) = agents.remove(old_name) {
        // Update the entry's internal name field to match the new map key (INC-DEBT-011)
        if let Some(obj) = value.as_object_mut() {
            obj.insert("name".to_owned(), serde_json::json!(new_name));
        }
        agents.insert(new_name.to_owned(), value);
    }
}

fn reconcile_json(
    config_path: PathBuf,
    ide: IdeKey,
    ctx: &ReconcileContext<'_>,
    apply: bool,
) -> ReconcileReport {
    use std::collections::HashSet;

    let mut report = ReconcileReport {
        editor: ide.as_str().to_owned(),
        ..Default::default()
    };

    let mut config: serde_json::Value = if config_path.exists() {
        match std::fs::read_to_string(&config_path)
            .map_err(std::io::Error::other)
            .and_then(|raw| serde_json::from_str(&raw).map_err(std::io::Error::other))
        {
            Ok(value) => value,
            Err(error) => {
                report
                    .errors
                    .push(format!("{}: invalid JSON: {error}", config_path.display()));
                return report;
            }
        }
    } else {
        serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {},
            "mcp": {}
        })
    };

    let Some(agents) = config
        .get_mut("agent")
        .and_then(|value| value.as_object_mut())
    else {
        report
            .errors
            .push(format!("{}: no agent map", config_path.display()));
        return report;
    };

    let capabilities = EditorCapabilities::for_ide(ide);
    let bundle_names: HashSet<&str> = ctx.agents.iter().map(|a| a.name.as_str()).collect();

    // Process each bundle agent
    for agent in ctx.agents {
        let Some(target) = ctx.build_target(agent, ide, &capabilities) else {
            // NoModelConfigured → skip
            report.mark_skipped();
            continue;
        };

        // Try canonical name first, then alias (INC-DEBT-011: alias-driven diff detection)
        let existing = resolve_alias_for(ctx.renames, &agent.name, |n| {
            read_json_existing(&config_path, n)
        })
        .map(|(e, _)| e);
        let diffs = if let Some(ref ex) = existing {
            ReconcileContext::diff_existing_target(ex, &target, &capabilities)
        } else {
            // New agent: all fields are "new" — emit diffs for every supported field
            let mut diffs = vec![
                FieldDiff {
                    field_name: "description",
                    old_value: None,
                    new_value: Some(serde_json::Value::String(target.description.clone())),
                },
                FieldDiff {
                    field_name: "model",
                    old_value: None,
                    new_value: target.model.clone().map(serde_json::Value::String),
                },
            ];
            if capabilities.supports_mode
                && let Some(ref m) = target.mode
            {
                diffs.push(FieldDiff {
                    field_name: "mode",
                    old_value: None,
                    new_value: Some(serde_json::Value::String(m.clone())),
                });
            }
            if capabilities.supports_hidden
                && let Some(h) = target.hidden
            {
                diffs.push(FieldDiff {
                    field_name: "hidden",
                    old_value: None,
                    new_value: Some(serde_json::Value::Bool(h)),
                });
            }
            if capabilities.supports_prompt_ref
                && let Some(ref p) = target.prompt
            {
                diffs.push(FieldDiff {
                    field_name: "prompt",
                    old_value: None,
                    new_value: Some(serde_json::Value::String(p.clone())),
                });
            }
            diffs
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
            // Detect name diff BEFORE we borrow agents via entry()
            let name_diff = result
                .diffs
                .iter()
                .find(|d| d.field_name == "name")
                .cloned();

            // Handle rename if present (INC-DEBT-010, cycle-36)
            if let Some(ref nd) = name_diff {
                apply_rename_in_agents_map(agents, nd, &mut report.errors);
            }

            // Apply changes to the in-memory config
            let entry = agents
                .entry(agent.name.clone())
                .or_insert_with(|| serde_json::json!({}));

            if let Some(obj) = entry.as_object_mut() {
                for diff in &result.diffs {
                    match diff.field_name {
                        "name" => {
                            // Already handled above; this arm is here only to consume the diff
                            // so it doesn't fall through to _ => {}
                        }
                        "description" => {
                            obj.insert(
                                "description".to_owned(),
                                serde_json::json!(target.description),
                            );
                        }
                        "model" => {
                            if let Some(ref m) = target.model {
                                obj.insert("model".to_owned(), serde_json::json!(m));
                            } else {
                                obj.remove("model");
                            }
                        }
                        "mode" => {
                            if let Some(ref m) = target.mode {
                                obj.insert("mode".to_owned(), serde_json::json!(m));
                            }
                        }
                        "hidden" => {
                            if let Some(h) = target.hidden {
                                obj.insert("hidden".to_owned(), serde_json::json!(h));
                            }
                        }
                        "prompt" => {
                            if let Some(ref p) = target.prompt {
                                obj.insert("prompt".to_owned(), serde_json::json!(p));
                            }
                        }
                        "tools" => {
                            if let Some(ref t) = target.tools {
                                obj.insert("tools".to_owned(), serde_json::json!(t));
                            }
                        }
                        _ => {}
                    }
                }

                // Preserve extras
                if let Some(ref ex) = existing {
                    for (k, v) in &ex.extras {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }

    // Prune: remove sddk-owned agents not in bundle
    let orphans: Vec<String> = agents
        .keys()
        .filter(|name| is_sddk_owned(name) && !bundle_names.contains(name.as_str()))
        .cloned()
        .collect();

    for orphan in &orphans {
        if apply {
            agents.remove(orphan);
        }
        report.mark_pruned();
    }

    if apply && (!orphans.is_empty() || report.agents_changed > 0) {
        // Write if anything changed
        match serde_json::to_string_pretty(&config) {
            Ok(serialized) => {
                if let Err(error) = atomic_write(&config_path, serialized.as_bytes(), None) {
                    report
                        .errors
                        .push(format!("{}: {error}", config_path.display()));
                }
            }
            Err(error) => {
                report.errors.push(format!(
                    "{}: serialization failed: {error}",
                    config_path.display()
                ));
            }
        }
    }

    report
}

/// Upsert bundle agents into a JSON editor config.
///
/// Invariants (ADR-0018): first-time only (existing entries are skipped
/// byte-untouched); ConfigAbsent omits the `model` key; NoModelConfigured
/// skips the agent; pruning is bounded to framework-namespaced orphans.
pub(super) fn upsert_json_agents(
    config_path: &Path,
    ide: IdeKey,
    primary_agents: &[&str],
    ctx: &RegistrationContext<'_>,
) -> AdapterReport {
    let mut report = AdapterReport {
        editor: ide.as_str().to_owned(),
        ..AdapterReport::default()
    };
    let mut config: serde_json::Value = if config_path.exists() {
        match std::fs::read_to_string(config_path)
            .map_err(std::io::Error::other)
            .and_then(|raw| serde_json::from_str(&raw).map_err(std::io::Error::other))
        {
            Ok(value) => value,
            Err(error) => {
                report
                    .errors
                    .push(format!("{}: invalid JSON: {error}", config_path.display()));
                return report;
            }
        }
    } else {
        serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {},
            "mcp": {}
        })
    };
    let Some(agents) = config
        .get_mut("agent")
        .and_then(|value| value.as_object_mut())
    else {
        report
            .errors
            .push(format!("{}: no agent map", config_path.display()));
        return report;
    };
    let mut changed = false;
    for agent in ctx.agents {
        // The expected prompt path for this agent under the new framework root.
        let new_prompt = format!(
            "{{file:{}}}",
            ctx.root
                .join("agents")
                .join(format!("{}.md", agent.name))
                .to_string_lossy()
        );
        if let Some(existing) = agents.get_mut(&agent.name).and_then(|v| v.as_object_mut()) {
            // Entry already exists. Refresh the prompt path ONLY if the existing
            // path looks like a previous sddk install (stale), preserving any
            // user customization (model, hidden, mode, custom prompt path).
            let existing_prompt = existing
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if existing_prompt == new_prompt {
                report.skipped_existing += 1;
            } else if looks_like_framework_prompt(existing_prompt, &agent.name) {
                existing.insert("prompt".to_owned(), serde_json::Value::String(new_prompt));
                report.updated_stale += 1;
                changed = true;
            } else {
                // Path is a user-customized prompt — leave it alone.
                report.skipped_existing += 1;
            }
            continue;
        }
        match resolve_for_models(ctx.models, &agent.name, ide) {
            Ok(model) => {
                let primary = primary_agents.contains(&agent.name.as_str());
                let mut entry = serde_json::json!({
                    "description": agent.description,
                    "mode": if primary { "primary" } else { "subagent" },
                    "prompt": new_prompt,
                });
                if let Some(model) = model {
                    entry["model"] = serde_json::Value::String(model);
                }
                if !primary {
                    entry["hidden"] = serde_json::Value::Bool(true);
                }
                agents.insert(agent.name.clone(), entry);
                report.registered += 1;
                changed = true;
            }
            Err(()) => report.skipped_unresolved += 1,
        }
    }
    let bundle_names: std::collections::HashSet<&str> =
        ctx.agents.iter().map(|agent| agent.name.as_str()).collect();
    let orphans: Vec<String> = agents
        .keys()
        .filter(|name| is_sddk_owned(name) && !bundle_names.contains(name.as_str()))
        .cloned()
        .collect();
    for orphan in orphans {
        agents.remove(&orphan);
        report.pruned += 1;
        changed = true;
    }
    if !changed {
        return report;
    }
    match serde_json::to_string_pretty(&config) {
        Ok(serialized) => {
            if let Err(error) = atomic_write(config_path, serialized.as_bytes(), None) {
                report
                    .errors
                    .push(format!("{}: {error}", config_path.display()));
            }
        }
        Err(error) => report.errors.push(format!(
            "{}: serialization failed: {error}",
            config_path.display()
        )),
    }
    report
}

/// Heuristic: detect whether an existing `{file:...}` prompt path looks like it
/// came from a previous sddk install (and is therefore stale-eligible) rather
/// than a user-customized path.
///
/// Recognised framework path shapes:
///   - `<somewhere>/sddk-framework/agents/<name>.md`   (dogfooding / bind-mount)
///   - `<somewhere>/.local/share/sddk/framework/<version-or-current>/agents/<name>.md`
///
/// Anything else (`/custom/path.md`, etc.) is treated as a user customization
/// and is preserved byte-untouched.
fn looks_like_framework_prompt(prompt: &str, expected_name: &str) -> bool {
    // Strip the `{file:...}` wrapper if present.
    let inner = prompt
        .strip_prefix("{file:")
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(prompt);
    // Must point to an `<name>.md` file (otherwise it's clearly not the agent
    // we registered).
    let expected_suffix = format!("/agents/{expected_name}.md");
    if !inner.ends_with(&expected_suffix) {
        return false;
    }
    inner.contains("/sddk-framework/agents/") || inner.contains("/sddk/framework/")
}

#[cfg(test)]
#[path = "../tests/json_adapter_tests.rs"]
mod json_adapter_tests;
