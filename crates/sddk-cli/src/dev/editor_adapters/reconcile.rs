//! Per-IDE reconciliation adapters and shared types.
//!
//! Reconciles drift between bundle agent sources + agent-models.yaml
//! and the per-IDE config files (opencode.json, zcode.json, agents/*.md, agents/*.toml).

use crate::dev::agent_models::{AgentModelsConfig, IdeKey};
use std::collections::BTreeMap;
use std::path::Path;

// AgentSource is pub(super) so it's accessible within editor_adapters
use super::{AgentSource, is_framework_namespaced};

// ── EditorCapabilities ────────────────────────────────────────────────────────

/// EditorCapabilities describes which optional capabilities an editor supports.
///
/// `PartialEq` and `Eq` are intentionally NOT derived because the `model_validator`
/// field is a function pointer (`Option<fn(&str) -> bool>`), and function pointers
/// have unpredictable equality semantics (different codegen units may produce
/// different addresses for the same function; identical functions may share an
/// address after merging). Cycle-33 closes this latent footgun per INC-DEBT-007.
///
/// See: docs/debt/INC-DEBT-007-preexisting-clippy-sddk-cli.md
#[derive(Debug, Clone, Copy)]
pub struct EditorCapabilities {
    /// IDE supports the `mode` field.
    pub supports_mode: bool,
    /// IDE supports the `hidden` field.
    pub supports_hidden: bool,
    /// IDE supports prompt-path references.
    pub supports_prompt_ref: bool,
    /// IDE supports the `tools` field.
    pub supports_tools: bool,
    /// Reserved for capability framework per ADR-0064 §D-4.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    pub model_validator: Option<fn(&str) -> bool>,
}

impl EditorCapabilities {
    /// Returns the capabilities for a given IDE.
    pub fn for_ide(ide: IdeKey) -> Self {
        match ide {
            IdeKey::Opencode => EditorCapabilities {
                supports_mode: true,
                supports_hidden: true,
                supports_prompt_ref: true,
                supports_tools: false,
                model_validator: None,
            },
            IdeKey::Zcode => EditorCapabilities {
                supports_mode: true,
                supports_hidden: true,
                supports_prompt_ref: true,
                supports_tools: false,
                model_validator: None,
            },
            IdeKey::Claude => EditorCapabilities {
                supports_mode: false,
                supports_hidden: false,
                supports_prompt_ref: false,
                supports_tools: true,
                model_validator: Some(claude_model_valid),
            },
            IdeKey::Codex => EditorCapabilities {
                supports_mode: false,
                supports_hidden: false,
                supports_prompt_ref: false,
                supports_tools: false,
                model_validator: None,
            },
        }
    }
}

/// Claude Code model vocabulary validation.
fn claude_model_valid(model: &str) -> bool {
    matches!(model, "sonnet" | "opus" | "haiku" | "inherit") || model.contains('/')
}

// ── ReconcileTarget ───────────────────────────────────────────────────────────

/// The desired state for one agent in one IDE config after reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileTarget {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub hidden: Option<bool>,
    pub prompt: Option<String>,
    pub tools: Option<String>,
}

// ── ExistingEntry ────────────────────────────────────────────────────────────

/// An existing entry read from an editor config, preserving unknown fields.
#[derive(Debug, Clone)]
pub struct ExistingEntry {
    /// Agent name.
    ///
    /// Captured by `read_existing()` from the lookup key, then diffed in
    /// `diff_existing_target()` per cycle-35 to close the structural design
    /// gap (INC-DEBT-009). Retained per ADR-0064 §D-5 capability-framework
    /// contract.
    pub name: String,
    /// Description (if present).
    pub description: Option<String>,
    /// Model (if present).
    pub model: Option<String>,
    /// Mode (if present) — opencode/zcode only.
    pub mode: Option<String>,
    /// Hidden (if present) — opencode/zcode only.
    pub hidden: Option<bool>,
    /// Prompt path (if present) — opencode/zcode only.
    pub prompt: Option<String>,
    /// Tools (if present) — claude only.
    pub tools: Option<String>,
    /// Extra fields not managed by sddk, preserved as raw JSON values.
    pub extras: BTreeMap<String, serde_json::Value>,
}

impl ExistingEntry {
    /// Reserved for capability framework per ADR-0064 §D-5.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    pub fn has_sddk_fields(&self) -> bool {
        self.description.is_some()
            || self.model.is_some()
            || self.mode.is_some()
            || self.hidden.is_some()
            || self.prompt.is_some()
            || self.tools.is_some()
    }
}

// ── FieldDiff ────────────────────────────────────────────────────────────────

/// A field that differs between existing and target state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FieldDiff {
    /// Name of the field that differs.
    pub field_name: &'static str,
    /// Current value in the editor config.
    pub old_value: Option<serde_json::Value>,
    /// Desired value from bundle + agent-models.yaml.
    pub new_value: Option<serde_json::Value>,
}

// ── ReconcileReport ──────────────────────────────────────────────────────────

/// Per-agent reconciliation result.
#[derive(Debug, Clone)]
pub struct AgentReconcileResult {
    /// Reserved for capability framework per ADR-0064 §D-4.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    pub name: String,
    /// Whether any field changed or would change.
    pub changed: bool,
    /// Field-level diffs.
    pub diffs: Vec<FieldDiff>,
    /// Errors encountered during reconciliation.
    pub errors: Vec<String>,
}

impl AgentReconcileResult {
    /// Reserved for capability framework per ADR-0064 §D-4.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    pub fn skipped(name: String) -> Self {
        Self {
            name,
            changed: false,
            diffs: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Reserved for capability framework per ADR-0064 §D-4.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    pub fn pruned(name: String) -> Self {
        Self {
            name,
            changed: true,
            diffs: Vec::new(),
            errors: Vec::new(),
        }
    }
}

/// Full reconciliation report for one IDE.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ReconcileReport {
    pub editor: String,
    pub agents_total: usize,
    pub agents_changed: usize,
    pub agents_pruned: usize,
    pub agents_skipped: usize,
    pub errors: Vec<String>,
}

impl ReconcileReport {
    /// Merges an per-agent result into this report.
    pub fn merge_agent(&mut self, result: &AgentReconcileResult) {
        self.agents_total += 1;
        if result.changed {
            self.agents_changed += 1;
        }
        if !result.errors.is_empty() {
            self.errors.extend(result.errors.iter().cloned());
        }
    }

    /// Marks an agent as pruned.
    pub fn mark_pruned(&mut self) {
        self.agents_pruned += 1;
    }

    /// Marks an agent as skipped (NoModelConfigured).
    pub fn mark_skipped(&mut self) {
        self.agents_skipped += 1;
    }
}

// ── Renames builder ─────────────────────────────────────────────────────────

/// Builds a BTreeMap from alias → canonical agent name.
///
/// Only sddk-owned agents (is_framework_namespaced) are included.
/// BTreeMap gives deterministic iteration order (INV-8, INV-11).
/// Collision: first-loaded alphabetical wins (INV-11).
pub(crate) fn renames_builder(agents: &[AgentSource]) -> BTreeMap<String, String> {
    let mut renames: BTreeMap<String, String> = BTreeMap::new();
    for agent in agents {
        // Only framework-namespaced agents contribute aliases (S5 scope filter)
        if !is_framework_namespaced(&agent.name) {
            continue;
        }
        if let Some(ref aliases) = agent.aliases {
            for alias in aliases {
                // First-loaded wins on collision (INV-11: alphabetical sort in load_agent_sources)
                renames
                    .entry(alias.clone())
                    .or_insert_with(|| agent.name.clone());
            }
        }
    }
    renames
}

/// Resolves an alias for a canonical name: checks if the canonical name exists
/// in the IDE config; if not, looks up any alias that maps to this canonical.
///
/// First-match-wins: if canonical exists as a key in renames, the canonical wins.
/// Returns (existing_entry, old_name) if an alias-driven entry is found.
pub(crate) fn resolve_alias_for<F>(
    renames: &BTreeMap<String, String>,
    canonical: &str,
    read_fn: F,
) -> Option<(ExistingEntry, String)>
where
    F: Fn(&str) -> Option<ExistingEntry>,
{
    // First: check if canonical name already exists (first-match-wins, S5)
    if let Some(existing) = read_fn(canonical) {
        return Some((existing, canonical.to_owned()));
    }

    // Second: look for an alias that maps to this canonical name
    for (alias, canonical_name) in renames {
        if canonical_name == canonical
            && let Some(existing) = read_fn(alias)
        {
            return Some((existing, alias.clone()));
        }
    }

    None
}

// ── ReconcileContext ─────────────────────────────────────────────────────────

/// Context passed to all adapters during reconciliation.
#[derive(Debug, Clone)]
pub struct ReconcileContext<'a> {
    pub root: &'a Path,
    pub agents: &'a [AgentSource],
    pub models: Option<&'a AgentModelsConfig>,
    /// Alias → canonical agent name map built from bundle agents.
    /// Populated by renames_builder() in run_dev_reconcile.
    pub renames: &'a BTreeMap<String, String>,
}

impl<'a> ReconcileContext<'a> {
    /// Builds a ReconcileTarget for a given agent + IDE.
    pub fn build_target(
        &self,
        agent: &AgentSource,
        ide: IdeKey,
        capabilities: &EditorCapabilities,
    ) -> Option<ReconcileTarget> {
        use crate::dev::editor_adapters::resolve_for_models;

        let model = match resolve_for_models(self.models, &agent.name, ide) {
            Ok(m) => m,
            Err(()) => return None, // NoModelConfigured → skip
        };

        let prompt = if capabilities.supports_prompt_ref {
            Some(format!(
                "{{file:{}}}",
                self.root
                    .join("agents")
                    .join(format!("{}.md", agent.name))
                    .to_string_lossy()
            ))
        } else {
            None
        };

        let mode = if capabilities.supports_mode {
            // Primary agents get "primary", others get "subagent"
            Some(
                if crate::dev::editor_adapters::PRIMARY_AGENTS.contains(&agent.name.as_str()) {
                    "primary".to_owned()
                } else {
                    "subagent".to_owned()
                },
            )
        } else {
            None
        };

        let hidden = if capabilities.supports_hidden
            && !crate::dev::editor_adapters::PRIMARY_AGENTS.contains(&agent.name.as_str())
        {
            Some(true)
        } else {
            None
        };

        Some(ReconcileTarget {
            name: agent.name.clone(),
            description: agent.description.clone(),
            model,
            mode,
            hidden,
            prompt,
            tools: agent.tools.clone(),
        })
    }

    /// Diffs an existing entry against a target, returning field-level changes.
    pub fn diff_existing_target(
        existing: &ExistingEntry,
        target: &ReconcileTarget,
        capabilities: &EditorCapabilities,
    ) -> Vec<FieldDiff> {
        let mut diffs = Vec::new();

        // Compare names. Cycle-35 closes INC-DEBT-009 by wiring the missing
        // name comparison (read_existing captures `ExistingEntry.name`, but
        // diff_existing_target never diffed it). All 4 current adapters set
        // existing.name from the lookup key (= bundle agent name), so this
        // comparison is invariantly true today. The capability-framework
        // contract (ADR-0064 §D-5) preserves the field for future adapters
        // that may parse `name:` values from config files.
        if existing.name != target.name {
            diffs.push(FieldDiff {
                field_name: "name",
                old_value: Some(serde_json::Value::String(existing.name.clone())),
                new_value: Some(serde_json::Value::String(target.name.clone())),
            });
        }

        if existing.description.as_ref() != Some(&target.description) {
            diffs.push(FieldDiff {
                field_name: "description",
                old_value: existing
                    .description
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
                new_value: Some(serde_json::Value::String(target.description.clone())),
            });
        }

        if existing.model.as_ref() != target.model.as_ref() {
            diffs.push(FieldDiff {
                field_name: "model",
                old_value: existing
                    .model
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
                new_value: target
                    .model
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
            });
        }

        if capabilities.supports_mode && existing.mode.as_ref() != target.mode.as_ref() {
            diffs.push(FieldDiff {
                field_name: "mode",
                old_value: existing
                    .mode
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
                new_value: target
                    .mode
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
            });
        }

        if capabilities.supports_hidden && existing.hidden != target.hidden {
            diffs.push(FieldDiff {
                field_name: "hidden",
                old_value: existing.hidden.map(serde_json::Value::Bool),
                new_value: target.hidden.map(serde_json::Value::Bool),
            });
        }

        if capabilities.supports_prompt_ref && existing.prompt.as_ref() != target.prompt.as_ref() {
            diffs.push(FieldDiff {
                field_name: "prompt",
                old_value: existing
                    .prompt
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
                new_value: target
                    .prompt
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
            });
        }

        if capabilities.supports_tools && existing.tools.as_ref() != target.tools.as_ref() {
            diffs.push(FieldDiff {
                field_name: "tools",
                old_value: existing
                    .tools
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
                new_value: target
                    .tools
                    .as_ref()
                    .map(|s| serde_json::Value::String(s.clone())),
            });
        }

        diffs
    }
}

// ── ReconcileAdapter trait ─────────────────────────────────────────────────────

/// Trait for IDE-specific reconciliation adapters.
pub trait ReconcileAdapter: Send + Sync {
    /// Reserved for capability framework per ADR-0064 §D-5.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    fn editor_name(&self) -> &'static str;
    /// Reserved for capability framework per ADR-0064 §D-5.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    fn capabilities(&self) -> EditorCapabilities;
    /// Reserved for capability framework per ADR-0064 §D-5.
    /// Do not remove without updating the ADR.
    #[allow(dead_code)]
    fn read_existing(&self, name: &str) -> Option<ExistingEntry>;
    /// Reconciles all bundle agents into this editor's config.
    fn reconcile(&self, ctx: &ReconcileContext<'_>, apply: bool) -> ReconcileReport;
}

// ── Adapter dispatch ──────────────────────────────────────────────────────────

/// Returns reconciliation adapters for the selected editor(s).
pub fn reconcilers_for(
    editor: crate::dev::LinkEditor,
    dirs: &crate::dev::editor_adapters::EditorDirs,
) -> Vec<Box<dyn ReconcileAdapter>> {
    let mut adapters: Vec<Box<dyn ReconcileAdapter>> = Vec::new();
    if matches!(
        editor,
        crate::dev::LinkEditor::OpenCode | crate::dev::LinkEditor::All
    ) {
        adapters.push(Box::new(crate::dev::editor_adapters::OpenCodeAdapter {
            dir: dirs.opencode.clone(),
        }));
    }
    if matches!(
        editor,
        crate::dev::LinkEditor::ZCode | crate::dev::LinkEditor::All
    ) {
        adapters.push(Box::new(crate::dev::editor_adapters::ZCodeAdapter {
            dir: dirs.zcode.clone(),
        }));
    }
    if matches!(
        editor,
        crate::dev::LinkEditor::Claude | crate::dev::LinkEditor::All
    ) {
        adapters.push(Box::new(crate::dev::editor_adapters::ClaudeAdapter {
            dir: dirs.claude.clone(),
        }));
    }
    if matches!(
        editor,
        crate::dev::LinkEditor::Codex | crate::dev::LinkEditor::All
    ) {
        adapters.push(Box::new(crate::dev::editor_adapters::CodexAdapter {
            dir: dirs.codex.clone(),
        }));
    }
    adapters
}
