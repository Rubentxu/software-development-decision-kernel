//! Per-IDE agent registration adapters (ADR-0019).
//! Shared invariants: first-time only, bounded pruning (framework namespace),
//! ConfigAbsent ≠ NoModelConfigured. `register()` never panics and never
//! returns `Result` — per-file failures are captured in `AdapterReport.errors`.

use super::LinkEditor;
use crate::dev::agent_models::{AgentModelsConfig, IdeKey};
use std::path::{Path, PathBuf};

mod claude;
mod codex;
mod json;
pub(super) mod reconcile;
pub(crate) use reconcile::renames_builder;

pub(super) use claude::ClaudeAdapter;
pub(super) use codex::CodexAdapter;
pub(super) use json::{OpenCodeAdapter, ZCodeAdapter};

/// Agents marked "primary" (visible by default) in editor configs.
pub(super) const PRIMARY_AGENTS: [&str; 2] = ["orchestrator", "book-orchestrator"];

/// A bundle agent parsed from `agents/<name>.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AgentSource {
    pub name: String,
    pub description: String,
    pub tools: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub body: String,
}

/// Shared input for all adapters, built once per `dev link` run.
pub(super) struct RegistrationContext<'a> {
    pub root: &'a Path,
    pub agents: &'a [AgentSource],
    pub models: Option<&'a AgentModelsConfig>,
}

/// Per-editor registration outcome.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub(super) struct AdapterReport {
    pub editor: String,
    pub registered: usize,
    pub updated_stale: usize,
    pub skipped_existing: usize,
    pub skipped_unresolved: usize,
    pub pruned: usize,
    pub errors: Vec<String>,
}

/// Registration seam: one implementation per editor.
pub(super) trait EditorAdapter {
    fn editor_name(&self) -> &'static str;
    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport;
}

/// Resolved editor config directories.
#[derive(Debug, Clone)]
pub(super) struct EditorDirs {
    pub opencode: PathBuf,
    pub zcode: PathBuf,
    pub claude: PathBuf,
    pub codex: PathBuf,
}

/// Symlink surface profile per editor: claude/codex own their agents dir
/// natively, so the agents symlink surface is skipped for them (ADR-0019).
#[derive(Debug, Clone, Copy)]
pub(super) struct LinkProfile {
    pub agents: bool,
    pub skills: bool,
    pub prompts: bool,
    pub workflows: bool,
}

impl LinkProfile {
    /// opencode/zcode: all four surfaces symlinked.
    pub(super) const ALL: Self = Self {
        agents: true,
        skills: true,
        prompts: true,
        workflows: true,
    };

    /// claude/codex: agents are adapter-owned native files, not symlinks.
    pub(super) const NATIVE_AGENTS: Self = Self {
        agents: false,
        skills: true,
        prompts: true,
        workflows: true,
    };

    pub(super) fn for_editor(editor: LinkEditor) -> Self {
        match editor {
            LinkEditor::OpenCode | LinkEditor::ZCode => Self::ALL,
            LinkEditor::Claude | LinkEditor::Codex => Self::NATIVE_AGENTS,
            LinkEditor::All => Self::ALL,
        }
    }
}

/// Dispatch: adapter instances for the selected editor(s).
pub(super) fn adapters_for(editor: LinkEditor, dirs: &EditorDirs) -> Vec<Box<dyn EditorAdapter>> {
    let mut adapters: Vec<Box<dyn EditorAdapter>> = Vec::new();
    if matches!(editor, LinkEditor::OpenCode | LinkEditor::All) {
        adapters.push(Box::new(OpenCodeAdapter {
            dir: dirs.opencode.clone(),
        }));
    }
    if matches!(editor, LinkEditor::ZCode | LinkEditor::All) {
        adapters.push(Box::new(ZCodeAdapter {
            dir: dirs.zcode.clone(),
        }));
    }
    if matches!(editor, LinkEditor::Claude | LinkEditor::All) {
        adapters.push(Box::new(ClaudeAdapter {
            dir: dirs.claude.clone(),
        }));
    }
    if matches!(editor, LinkEditor::Codex | LinkEditor::All) {
        adapters.push(Box::new(CodexAdapter {
            dir: dirs.codex.clone(),
        }));
    }
    adapters
}

// ── Agent source loading ──────────────────────────────────────────────────────

#[derive(Debug)]
struct ParsedAgent {
    description: String,
    tools: Option<String>,
    aliases: Option<Vec<String>>,
    body: String,
}

/// Block frontmatter parser: description, optional `tools:`, optional `aliases:`,
/// and the body (everything after the closing `---`). The legacy `model:`
/// frontmatter key is deliberately not read for registration (ADR-0017).
///
/// `aliases:` accepts three forms:
/// - Array:   `aliases: [a, b]`
/// - Bare:    `aliases: x`
/// - Multi:   `aliases:\n  - a\n  - b`
///
/// Returns `aliases: None` when the field is absent from frontmatter (VC2).
fn parse_agent_file(content: &str) -> Option<ParsedAgent> {
    let rest = content.strip_prefix("---")?;
    let (block, body) = rest.split_once("---")?;
    let mut description = String::new();
    let mut tools = None;
    let mut aliases: Option<Vec<String>> = None;
    let mut in_aliases_list = false;

    for line in block.lines() {
        let trimmed = line.trim();
        if in_aliases_list {
            if trimmed.starts_with('-') {
                if let Some(ref mut v) = aliases {
                    v.push(trimmed.trim_start_matches('-').trim().to_owned());
                }
            } else {
                in_aliases_list = false;
            }
        }
        if let Some(value) = trimmed.strip_prefix("description:") {
            description = value.trim().trim_matches('"').to_owned();
        } else if let Some(value) = trimmed.strip_prefix("tools:") {
            let t = value.trim().trim_matches('"');
            if !t.is_empty() {
                tools = Some(t.to_owned());
            }
        } else if trimmed == "aliases:" || trimmed.starts_with("aliases:") {
            // Check for array on same line: aliases: [a, b]
            let rest = trimmed.strip_prefix("aliases:").unwrap_or("").trim();
            if rest.starts_with('[') && rest.ends_with(']') {
                // array form: aliases: [a, b]
                let inner = &rest[1..rest.len() - 1];
                let mut items = Vec::new();
                for item in inner.split(',') {
                    let item = item.trim().trim_matches('"').trim();
                    if !item.is_empty() {
                        items.push(item.to_owned());
                    }
                }
                aliases = Some(items);
            } else if !rest.is_empty() && !rest.starts_with('-') {
                // bare form: aliases: x
                aliases = Some(vec![rest.trim_matches('"').to_owned()]);
            } else if rest.is_empty() {
                // multiline list follows
                aliases = Some(Vec::new());
                in_aliases_list = true;
            }
        }
    }
    if description.is_empty() {
        return None;
    }
    Some(ParsedAgent {
        description,
        tools,
        aliases,
        body: body.to_owned(),
    })
}

/// Load every `root/agents/*.md` into `AgentSource` (agents without a
/// description are skipped — existing behavior). One read per `dev link` run.
pub(super) fn load_agent_sources(root: &Path) -> Vec<AgentSource> {
    let agents_dir = root.join("agents");
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some(parsed) = parse_agent_file(&content) else {
                continue;
            };
            sources.push(AgentSource {
                name: name.to_owned(),
                description: parsed.description,
                tools: parsed.tools,
                aliases: parsed.aliases,
                body: parsed.body,
            });
        }
    }
    sources.sort_by(|a, b| a.name.cmp(&b.name));
    sources
}

/// Framework namespaces subject to pruning (bounded scope, ADR-0018).
pub(super) fn is_framework_namespaced(name: &str) -> bool {
    name.starts_with("sddk-") || name.starts_with("sdd-") || name.starts_with("gentle-")
}

/// Returns true if the agent is owned by the sddk framework.
/// Follows the ownership rule (ADR-0064 §D-3): an agent is "of sddk" if it
/// is framework-namespaced OR if it is in PRIMARY_AGENTS (orchestrator,
/// book-orchestrator), regardless of naming prefix.
pub(super) fn is_sddk_owned(name: &str) -> bool {
    is_framework_namespaced(name) || PRIMARY_AGENTS.contains(&name)
}

/// Model resolution for an (agent, ide) pair:
/// `Ok(None)` = ConfigAbsent (register without a model field), `Ok(Some)` =
/// model id, `Err(())` = NoModelConfigured (skip the agent).
pub(super) fn resolve_for_models(
    models: Option<&AgentModelsConfig>,
    agent: &str,
    ide: IdeKey,
) -> Result<Option<String>, ()> {
    match models {
        None => Ok(None),
        Some(config) => match config.resolve(agent, ide) {
            crate::dev::agent_models::ModelResolution::Model(model) => Ok(Some(model)),
            crate::dev::agent_models::ModelResolution::NoModelConfigured { .. } => Err(()),
        },
    }
}

#[cfg(test)]
pub(super) mod test_fixtures {
    use super::*;
    use crate::dev::agent_models::AgentModelsConfig;

    /// Wrapper for test inspection of parsed frontmatter (aliases only).
    /// name is read from the filename stem externally — not stored here.
    #[derive(Debug)]
    pub(crate) struct ParsedAgentForTest {
        pub aliases: Option<Vec<String>>,
    }

    /// Calls the internal parse_agent_file and wraps the result for test inspection.
    pub(crate) fn parse_agent_file_for_test(content: &str) -> Option<ParsedAgentForTest> {
        parse_agent_file(content).map(|p| ParsedAgentForTest { aliases: p.aliases })
    }

    pub(crate) const FIXTURE_YAML: &str = "tiers:\n  premium:\n    opencode: deepseek/deepseek-chat\n    zcode: deepseek/deepseek-chat\n    claude: sonnet\n    codex: openai/gpt-5.4\n  fast:\n    opencode: zai-coding-plan/glm-5-turbo\n    zcode: zai-coding-plan/glm-5-turbo\n    claude: haiku\n    codex: openai/gpt-5.4-fast\nagents:\n  orchestrator:\n    tier: premium\n  sddk-foo:\n    tier: fast\n    overrides:\n      opencode: deepseek/deepseek-reasoner\n  gentle-bar:\n    tier: fast\n";

    pub(super) struct Fixture {
        pub root: tempfile::TempDir,
        pub agents: Vec<AgentSource>,
        pub models: AgentModelsConfig,
    }

    /// Temp framework root with 3 synthetic agents: orchestrator (primary),
    /// sddk-foo, gentle-bar.
    pub(super) fn build() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("agents")).unwrap();
        let specs: [(&str, &str, Option<&str>, &str); 3] = [
            (
                "orchestrator",
                "Team coordinator",
                None,
                "# Orchestrator body\n",
            ),
            (
                "sddk-foo",
                "Foo explorer",
                Some("read, bash"),
                "# Foo body\n",
            ),
            ("gentle-bar", "Bar reviewer", None, "# Bar body\n"),
        ];
        for (name, description, tools, body) in specs {
            let mut frontmatter = format!("---\nname: {name}\ndescription: {description}\n");
            if let Some(tools) = tools {
                frontmatter.push_str(&format!("tools: {tools}\n"));
            }
            frontmatter.push_str("---\n");
            std::fs::write(
                root.join("agents").join(format!("{name}.md")),
                format!("{frontmatter}{body}"),
            )
            .unwrap();
        }
        let agents = load_agent_sources(root);
        let models = AgentModelsConfig::from_yaml(FIXTURE_YAML).unwrap();
        Fixture {
            root: tmp,
            agents,
            models,
        }
    }

    pub(super) fn ctx<'a>(
        fixture: &'a Fixture,
        models: Option<&'a AgentModelsConfig>,
    ) -> RegistrationContext<'a> {
        RegistrationContext {
            root: fixture.root.path(),
            agents: &fixture.agents,
            models,
        }
    }
}
