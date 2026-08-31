//! Skill registry — write idempotent skill registry markdown.

use std::path::{Path, PathBuf};

use crate::CliEnvironment;

use super::common::atomic_write;
use super::paths::sddk_data_dir;

/// Skill registry entry: name, trigger/description, scope, and path of one skill.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct SkillRegistryEntry {
    name: String,
    trigger: String,
    description: String,
    scope: String,
    path: String,
}

/// Minimal frontmatter extraction (name, description) from a skill SKILL.md.
struct SkillFrontmatter {
    name: String,
    description: String,
}

fn parse_skill_frontmatter(path: &Path) -> Option<SkillFrontmatter> {
    let content = std::fs::read_to_string(path).ok()?;
    let block = content.strip_prefix("---")?.split_once("---")?.0;
    let mut name = String::new();
    let mut description = String::new();
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            name = value.trim().trim_matches('"').to_owned();
        } else if let Some(value) = line.strip_prefix("description:") {
            description = value.trim().trim_matches('"').to_owned();
        }
    }
    if name.is_empty() || description.is_empty() {
        return None;
    }
    Some(SkillFrontmatter { name, description })
}

/// Escape pipes and newlines in a markdown table cell so the table renders correctly.
fn escape_md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', "\\n")
}

/// Write an idempotent, deduplicated skill registry to
/// `$SDDK_DATA_DIR/projects/<project_id>/skill-registry.md`.
///
/// Scans skills from three scopes in precedence order (first wins dedupe):
/// 1. Project-level: `{project_root}/.opencode/skills/`, `.agents/skills/`,
///    `.claude/skills/`, `.zcode/skills/`
/// 2. User-level: `$HOME/.config/opencode/skills/`, `claude/skills/`, `zcode/skills/`
/// 3. Framework-level: `{framework_root}/skills/`
///
/// Skips `_shared` and `skill-registry`. Parses frontmatter name + description.
/// Extracts trigger from description (text before first ". "). Renders markdown table.
/// File is written atomically so a second invocation produces byte-identical result.
pub(super) fn write_skill_registry(
    environment: &CliEnvironment,
    project_root: &Path,
    framework_root: &Path,
) -> anyhow::Result<(PathBuf, usize)> {
    let project_id = resolve_project_id_for_registry(environment, project_root)?;
    let registry_dir = sddk_data_dir(environment)?
        .join("projects")
        .join(&project_id);
    let registry_path = registry_dir.join("skill-registry.md");
    std::fs::create_dir_all(&registry_dir)?;

    // Determine home dir for user-level scans.
    // Prefer environment.home when set (tests, isolated environments);
    // fall back to the system HOME variable.
    let home = environment.home.clone().unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
    });

    // Define all scopes with their search paths in precedence order.
    // Each entry: (scope_label, base_dir).
    // User-level: documented editor config dirs (AGENTS/layout supports zcode + others from bundle).
    // Project-level: project-root-relative dirs for adopted projects.
    // Framework-level: skills/ inside the active framework bundle.
    let scopes: Vec<(&str, PathBuf)> = vec![
        // Project-level dirs under the adopted project root.
        ("project", project_root.join(".opencode/skills")),
        ("project", project_root.join(".agents/skills")),
        ("project", project_root.join(".claude/skills")),
        ("project", project_root.join(".zcode/skills")),
        ("project", project_root.join(".kilo/skills")),
        ("project", project_root.join(".codex/skills")),
        // User-level dirs (XDG_CONFIG_HOME and HOME-relative documented paths).
        ("user", home.join(".config/opencode/skills")),
        ("user", home.join(".agents/skills")),
        ("user", home.join(".claude/skills")),
        ("user", home.join(".zcode/skills")),
        ("user", home.join(".opencode/skills")),
        ("user", home.join(".config/kilo/skills")),
        ("user", home.join(".codex/skills")),
        // Framework-level dirs under the framework root.
        ("framework", framework_root.join("skills")),
    ];

    let mut entries: Vec<SkillRegistryEntry> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (scope, skills_base) in scopes {
        if let Ok(skill_dirs) = std::fs::read_dir(&skills_base) {
            // Collect and sort for deterministic processing order.
            let mut dirs: Vec<_> = skill_dirs.flatten().collect();
            dirs.sort_by_key(|e| e.file_name());
            for skill_dir in dirs {
                let dir_name = skill_dir.file_name();
                let dir_name_str = dir_name.to_string_lossy();
                // Skip internal entries that are not user-facing skills.
                if dir_name_str == "_shared" || dir_name_str == "skill-registry" {
                    continue;
                }
                let skill_path = skill_dir.path();
                if !skill_path.is_dir() {
                    continue;
                }
                let skl_md = skill_path.join("SKILL.md");
                if !skl_md.is_file() {
                    continue;
                }
                // Dedupe by frontmatter name (first wins — higher precedence scope wins).
                let frontmatter_name = if let Some(fm) = parse_skill_frontmatter(&skl_md) {
                    fm.name.clone()
                } else {
                    dir_name_str.to_string()
                };
                if seen_names.contains(&frontmatter_name) {
                    continue;
                }
                seen_names.insert(frontmatter_name.clone());

                // Parse frontmatter for trigger and description.
                let (trigger, description) = if let Some(fm) = parse_skill_frontmatter(&skl_md) {
                    // Trigger: text before first ". " or first period in description.
                    let trigger_text = fm
                        .description
                        .split_once(". ")
                        .map(|(t, _)| t.to_string())
                        .or_else(|| fm.description.split_once('.').map(|(t, _)| t.to_string()))
                        .unwrap_or_else(|| fm.description.clone());
                    (trigger_text, fm.description)
                } else {
                    (String::new(), String::new())
                };

                entries.push(SkillRegistryEntry {
                    name: frontmatter_name,
                    trigger,
                    description,
                    scope: scope.to_string(),
                    path: skl_md.to_string_lossy().replace('\\', "/"),
                });
            }
        }
    }

    // Sort alphabetically by name.
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    // Render as markdown table with escaped cells.
    let mut content = String::new();
    content.push_str("# Skill Registry\n\n");
    content.push_str("| Name | Trigger | Description | Scope | Path |\n");
    content.push_str("|------|---------|-------------|-------|------|\n");
    for entry in &entries {
        content.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape_md_cell(&entry.name),
            escape_md_cell(&entry.trigger),
            escape_md_cell(&entry.description),
            escape_md_cell(&entry.scope),
            escape_md_cell(&entry.path),
        ));
    }

    atomic_write(&registry_path, content.as_bytes(), None)?;
    Ok((registry_path, entries.len()))
}

/// Resolve project_id for the skill registry writer.
///
/// Uses ONLY `crate::resolve_remote` (git command), `crate::find_persisted_fallback_seed`
/// (adoption receipt), and `sddk_domain::resolve_project_identity` — never fabricates
/// a UUID from a hash.
///
/// Priority:
///  1. Git remote URL → canonical resolver (stable p-* across machines)
///  2. Persisted adoption receipt seed → seeded fallback (stable p-* for adopted dirs)
///  3. Neither → explicit error with instructions to run `sddk adopt`
pub(super) fn resolve_project_id_for_registry(
    environment: &CliEnvironment,
    project_root: &Path,
) -> anyhow::Result<String> {
    let canonical = std::fs::canonicalize(project_root)?;
    let root_display = canonical.to_string_lossy().to_string();

    // Try git remote first.
    if let Some(remote_url) = crate::resolve_remote(project_root, None)? {
        // Use "." as scope (the CLI default) — remote already provides uniqueness.
        let identity = sddk_domain::resolve_project_identity(Some(&remote_url), ".", None);
        return identity
            .map(|id| id.project_id.to_string())
            .map_err(|e| anyhow::anyhow!("{e}"));
    }

    // No remote — look for a persisted adoption receipt for this workspace.
    if let Some(seed) = crate::find_persisted_fallback_seed(environment, &canonical, ".")? {
        let identity = sddk_domain::resolve_project_identity(None, ".", Some(&seed));
        return identity
            .map(|id| id.project_id.to_string())
            .map_err(|e| anyhow::anyhow!("{e}"));
    }

    // Neither remote nor adoption receipt found — require explicit adoption.
    anyhow::bail!(
        "cannot resolve project identity for registry: \
         no git remote found in {root_display} and no adoption receipt exists. \
         Run `sddk adopt --scope .` first to create a persistent project identity, \
         then retry `sddk dev link --write-registry`."
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/skill_registry_tests.rs"]
mod skill_registry_tests;
