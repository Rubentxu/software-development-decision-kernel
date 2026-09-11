//! M7.7 — Skill registry disk bridge.
//!
//! Bridges `~/.local/share/sddk/framework/skills/*/SKILL.md` (and
//! project-/user-level skill dirs) into the typed `SkillRegistry` from
//! M7.3. The runtime admission gate in the CLI runner
//! (see `run_with_environment`) calls `SkillRegistry::admits_command`
//! against this bridge-loaded registry before executing each command.
//!
//! This module is read-only at runtime. It does not modify the disk
//! skill directories.
//!
//! Honest scope:
//!
//! - The three `required_skill` placeholders wired in M7.5
//!   (`core.contract-review@v1`, `core.workflow-orchestration@v1`,
//!   `core.release-planning@v1`) are **contractual placeholders**, not
//!   real skill ids present on disk. The bridge does not fabricate
//!   them. The gate therefore emits a `MissingPlaceholderSkill`
//!   warning instead of failing closed — the user sees the gap clearly
//!   without the entire `lint` / `cycle` / `release` command chain
//!   breaking.
//! - The gate is fully operational: when a real `core.*@v*` skill lands
//!   on disk (e.g. via a future cycle that wires the contract-review
//!   workflow as a typed skill), the gate will admit without code
//!   changes.
//! - If the user adds a `required_skill` to a `CommandSpec` that does
//!   NOT match any on-disk skill AND is not one of the three known
//!   placeholders, the gate fails closed (refuses to run).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::command_spec::CommandSpec;
use crate::skill_definition::{
    AppliesWhen, SkillAdmitOutcome, SkillDefinition, SkillOutput, SkillRegistry,
};

/// Outcome of a runtime admission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionOutcome {
    /// The command has no `required_skill` declared — always admitted.
    NotRequired,
    /// All declared `required_skill`s are present in the registry.
    Admitted {
        /// `id@version` tokens that satisfy each declared required skill.
        satisfied_by: Vec<String>,
    },
    /// The required skill is one of the M7.5 contractual placeholders
    /// and is not on disk. Emitted as a warning, not a failure.
    MissingPlaceholderSkill {
        /// The placeholder skill ref_token that is not on disk.
        required: String,
    },
    /// The required skill is not in the registry and is not a known
    /// placeholder. The gate fails closed.
    MissingRealSkill {
        /// The required skill ref_token that is missing from disk.
        required: String,
        /// All skill ref_tokens the registry has on disk (sorted).
        available: Vec<String>,
    },
}

impl AdmissionOutcome {
    /// Returns true if the gate allows execution.
    pub fn is_admitted(&self) -> bool {
        matches!(
            self,
            AdmissionOutcome::NotRequired
                | AdmissionOutcome::Admitted { .. }
                | AdmissionOutcome::MissingPlaceholderSkill { .. }
        )
    }
}

/// The three M7.5 contractual placeholders. Kept as a constant so
/// downstream code can refer to the same set.
pub const PLACEHOLDER_SKILLS: &[&str] = &[
    "core.contract-review@v1",
    "core.workflow-orchestration@v1",
    "core.release-planning@v1",
];

/// A parsed `SkillDefinition` produced from one SKILL.md on disk.
#[derive(Debug, Clone)]
pub struct LoadedSkill {
    /// Typed skill definition constructed from the parsed SKILL.md.
    pub definition: SkillDefinition,
    /// Absolute path to the SKILL.md that produced this entry.
    pub source_path: PathBuf,
    /// Scope this skill was loaded from (framework > user > project).
    pub scope: SkillScope,
}

/// Where a skill was loaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillScope {
    /// Project-level: `{root}/.opencode/skills/`, `.agents/skills/`, etc.
    Project,
    /// User-level: `$HOME/.config/opencode/skills/`, `.claude/skills/`, etc.
    User,
    /// Framework-level: `{framework_root}/skills/`.
    Framework,
}

impl SkillScope {
    /// Stable lowercase id used as the namespacing prefix when
    /// constructing the `SkillDefinition::id` (e.g. `framework.ak`).
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
            Self::Framework => "framework",
        }
    }
}

/// Disk-bridge options.
#[derive(Debug, Clone)]
pub struct DiskBridgeOptions {
    /// Project root. If `Some`, project-level dirs are also scanned.
    pub project_root: Option<PathBuf>,
    /// Framework root (where `skills/` lives inside the bundle).
    pub framework_root: Option<PathBuf>,
    /// User home dir. Defaults to `$HOME`.
    pub home: Option<PathBuf>,
}

impl Default for DiskBridgeOptions {
    fn default() -> Self {
        Self {
            project_root: None,
            framework_root: None,
            home: std::env::var_os("HOME").map(PathBuf::from),
        }
    }
}

/// Search paths for the bridge, in precedence order (first wins dedupe).
///
/// M7.7 precedence: **framework > user > project**. The framework bundle
/// ships the canonical version of each skill; user-level dirs are local
/// overrides; project-level dirs are workspace-specific. We diverge
/// intentionally from `dev/registry::write_skill_registry` here because
/// runtime admission needs the canonical bundle version to win.
fn search_paths(opts: &DiskBridgeOptions) -> Vec<(SkillScope, PathBuf)> {
    let home = opts.home.clone().unwrap_or_else(|| PathBuf::from("/tmp"));
    let project_root = opts.project_root.clone();
    let framework_root = opts.framework_root.clone();

    let mut paths: Vec<(SkillScope, PathBuf)> = Vec::new();
    if let Some(fr) = framework_root {
        paths.push((SkillScope::Framework, fr.join("skills")));
    }
    for sub in [
        ".config/opencode/skills",
        ".agents/skills",
        ".claude/skills",
        ".zcode/skills",
        ".opencode/skills",
        ".config/kilo/skills",
        ".codex/skills",
    ] {
        paths.push((SkillScope::User, home.join(sub)));
    }
    if let Some(pr) = project_root {
        for sub in [
            ".opencode/skills",
            ".agents/skills",
            ".claude/skills",
            ".zcode/skills",
            ".kilo/skills",
            ".codex/skills",
        ] {
            paths.push((SkillScope::Project, pr.join(sub)));
        }
    }
    paths
}

/// Parse YAML frontmatter from a SKILL.md into a `LoadedSkill`.
///
/// The parser is intentionally permissive: missing or malformed fields
/// fall back to safe defaults rather than failing the entire load. This
/// keeps a single bad SKILL.md from poisoning the whole registry.
pub fn parse_skill_md(path: &Path, scope: SkillScope) -> Option<LoadedSkill> {
    let content = std::fs::read_to_string(path).ok()?;
    let (front_block, _body) = parse_frontmatter_block(&content)?;
    let fm = parse_yaml_frontmatter(front_block);

    let name = fm.name?;
    let version = parse_version(&fm.version);
    let description = fm.description.unwrap_or_default();

    // Dotted-name branch: when the frontmatter `name` already contains
    // at least one `.`, treat it as the fully-namespaced id token. The
    // `SkillScope` is still recorded on the `LoadedSkill` (and used by
    // `load_registry` for precedence dedupe) but does NOT participate in
    // the id. This is the M7.9 extension that lets `core.<...>@v1`
    // placeholders declared on `CommandSpec`s resolve to real ids.
    //
    // Un-dotted names preserve the historical behavior verbatim:
    // id = `<scope>.<name>`, so the 174 existing `framework.<name>@v1`
    // skills continue to round-trip identically.
    //
    // Defensive guard: a name whose first segment equals a known
    // `SkillScope::prefix()` (`framework`, `user`, `project`) would
    // otherwise double-wrap (`framework.framework.<name>`). Such names
    // are skipped silently — consistent with the existing
    // malformed-frontmatter handling.
    let id = if name.contains('.') {
        let first_segment = name.split('.').next().unwrap_or("");
        if matches!(first_segment, "framework" | "user" | "project") {
            return None;
        }
        name.clone()
    } else {
        format!("{}.{}", scope.prefix(), name)
    };

    // instruction_fragments: a single fragment derived from the
    // description so the skill carries a textual contribution.
    let fragment_text = if description.is_empty() {
        format!("{name} skill")
    } else {
        let trimmed = description.chars().take(200).collect::<String>();
        format!("{name} skill: {trimmed}")
    };

    let definition = SkillDefinition {
        id: id.clone(),
        version,
        applies_when: AppliesWhen {
            task_kinds: vec![name.clone()],
            context_keys: vec![],
        },
        inputs: vec![],
        outputs: SkillOutput {
            contract: "ContributionV2".into(),
            notes: Some(description.chars().take(500).collect::<String>()),
        },
        capabilities: Default::default(),
        evidence_contract: Default::default(),
        instruction_fragments: vec![fragment_text],
        compat: Default::default(),
    };

    // Validate; skip if the constructed definition still fails (defensive).
    definition.validate().ok()?;

    Some(LoadedSkill {
        definition,
        source_path: path.to_path_buf(),
        scope,
    })
}

fn parse_version(s: &str) -> u32 {
    if s.is_empty() {
        return 1;
    }
    // YAML may produce quoted strings like "1.0"; strip quotes and take
    // the major component.
    let trimmed = s.trim().trim_matches('"').trim_matches('\'');
    let major = trimmed.split('.').next().unwrap_or("1");
    major.parse::<u32>().unwrap_or(1).max(1)
}

/// Splits `---` frontmatter from body. Returns `(front_block, body)`.
fn parse_frontmatter_block(content: &str) -> Option<(&str, &str)> {
    let stripped = content.strip_prefix("---")?;
    let (front, rest) = stripped.split_once("---")?;
    Some((front, rest))
}

/// Minimal YAML frontmatter key/value extractor. Only handles the keys
/// the existing SKILL.md files use: `name`, `description`, and
/// `metadata.version`. No recursion, no arrays, no flow-style.
fn parse_yaml_frontmatter(block: &str) -> FrontmatterFields {
    let mut fields = FrontmatterFields::default();
    let mut in_metadata = false;
    for line in block.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Top-level keys.
        if let Some(rest) = line.strip_prefix("metadata:") {
            in_metadata = true;
            let _ = rest;
            continue;
        }
        if in_metadata {
            if let Some(rest) = line.trim_start().strip_prefix("version:") {
                fields.version = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                continue;
            }
            if let Some(rest) = line.trim_start().strip_prefix("author:") {
                fields.author = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                continue;
            }
            // Any other metadata key ends the block (or stays ignored).
            continue;
        }
        if let Some(rest) = line.strip_prefix("name:") {
            fields.name = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("description:") {
            let v = rest.trim().trim_matches('"').trim_matches('\'').to_string();
            fields.description = Some(v);
            continue;
        }
        if let Some(rest) = line.strip_prefix("license:") {
            fields.license = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            continue;
        }
    }
    fields
}

#[derive(Default, Debug)]
struct FrontmatterFields {
    name: Option<String>,
    description: Option<String>,
    version: String,
    author: String,
    license: Option<String>,
}

/// Load the runtime SkillRegistry from disk using the bridge options.
///
/// Files that fail to parse are silently skipped (logged via stderr
/// only in debug mode). Returns `(registry, loaded_skills)`.
///
/// Dedupe semantics: by the **canonical name** parsed from each
/// `SKILL.md` frontmatter (e.g. `shared-skill`). Higher-precedence scope
/// wins, mirroring `dev/registry::write_skill_registry`. The scope
/// prefix is added to the resulting `SkillDefinition::id` (so the
/// validator accepts it) but does NOT participate in dedupe.
pub fn load_registry(opts: &DiskBridgeOptions) -> (SkillRegistry, Vec<LoadedSkill>) {
    let mut registry = SkillRegistry::new();
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut loaded = Vec::new();

    for (scope, base) in search_paths(opts) {
        let Ok(dirs) = std::fs::read_dir(&base) else {
            continue;
        };
        let mut dir_entries: Vec<_> = dirs.flatten().collect();
        dir_entries.sort_by_key(|e| e.file_name());

        for dir_entry in dir_entries {
            let dir_name = dir_entry.file_name();
            let dir_name_str = dir_name.to_string_lossy();
            // Skip internal / hidden entries.
            if dir_name_str.starts_with('_')
                || dir_name_str.starts_with('.')
                || dir_name_str == "skill-registry"
                || dir_name_str == "BOOK-REPO-CONTEXT.md"
            {
                continue;
            }
            let skill_dir = dir_entry.path();
            if !skill_dir.is_dir() {
                continue;
            }
            // SKILL.md is the canonical entry point.
            let skill_md = skill_dir.join("SKILL.md");
            if !skill_md.is_file() {
                continue;
            }
            // Pre-check dedupe by the canonical name from frontmatter
            // (or fall back to the directory name).
            let content = std::fs::read_to_string(&skill_md).ok();
            let canonical_name = content
                .as_deref()
                .and_then(|c| parse_frontmatter_block(c))
                .and_then(|(f, _)| parse_yaml_frontmatter(f).name)
                .unwrap_or_else(|| dir_name_str.to_string());

            if seen_names.contains(&canonical_name) {
                continue;
            }

            let Some(parsed) = parse_skill_md(&skill_md, scope) else {
                continue;
            };
            // Belt-and-braces: confirm the parsed name matches what we
            // expected (it always does; defensive against parser drift).
            let parsed_name = parsed
                .definition
                .id
                .split_once('.')
                .map(|(_, n)| n.to_string())
                .unwrap_or_default();
            if parsed_name != canonical_name && seen_names.contains(&parsed_name) {
                continue;
            }
            seen_names.insert(canonical_name);
            seen_names.insert(parsed_name);
            // Register into the typed registry (skip duplicates silently).
            let _ = registry.register(parsed.definition.clone());
            loaded.push(parsed);
        }
    }

    (registry, loaded)
}

/// Run the admission gate against a `CommandSpec`. The gate is fail-open
/// for known M7.5 contractual placeholders (emits a warning) and
/// fail-closed for any other missing required skill.
pub fn check_admission(spec: &CommandSpec, registry: &SkillRegistry) -> AdmissionOutcome {
    if spec.required_skills.is_empty() {
        return AdmissionOutcome::NotRequired;
    }

    // Use the typed admission API for real checks.
    let typed_outcome = registry.admits_command(spec);
    if typed_outcome.is_admitted() {
        // Pull the satisfied tokens for the admitted outcome.
        let satisfied_by: Vec<String> = match &typed_outcome {
            SkillAdmitOutcome::Satisfied => Vec::new(),
            SkillAdmitOutcome::SatisfiedBy(s) => s.clone(),
            SkillAdmitOutcome::Missing { .. } => Vec::new(),
        };
        return AdmissionOutcome::Admitted { satisfied_by };
    }

    // Typed outcome is Missing. Distinguish placeholder vs real.
    let placeholder_hits: Vec<&String> = spec
        .required_skills
        .iter()
        .filter(|r| PLACEHOLDER_SKILLS.contains(&r.as_str()))
        .collect();

    if !placeholder_hits.is_empty() && placeholder_hits.len() == spec.required_skills.len() {
        // All required skills are known placeholders missing from disk.
        return AdmissionOutcome::MissingPlaceholderSkill {
            required: spec.required_skills.first().cloned().unwrap_or_default(),
        };
    }

    let mut available: Vec<String> = registry
        .entries()
        .iter()
        .map(|e| e.definition.ref_token())
        .collect();
    available.sort();
    AdmissionOutcome::MissingRealSkill {
        required: spec
            .required_skills
            .first()
            .cloned()
            .unwrap_or_else(|| "<unknown>".into()),
        available,
    }
}

/// CLI runner admission gate helper. Loads the runtime skill registry
/// from the active framework bundle + user-level skill dirs and checks
/// admission for the supplied `CommandSpec`. Returns:
/// - `Ok(None)` when the command is admitted (or no required skills).
/// - `Ok(Some(CommandOutput))` when the command is blocked (only when a
///   non-placeholder required skill is missing — fail-closed).
/// - `Err(anyhow::Error)` when the gate itself cannot run (no framework
///   installed, IO error). Caller may treat Err as "skip gate".
///
/// The placeholder check is the gap-honest behavior: the three
/// `core.*@v1` M7.5 placeholders are surfaced as a stderr warning but
/// do not block execution. This keeps existing user workflows alive
/// while making the missing-skill gap visible.
pub fn gate_command_for_environment(
    spec: &crate::command_spec::CommandSpec,
    environment: &crate::CliEnvironment,
) -> anyhow::Result<Option<crate::CommandOutput>> {
    // Resolve framework root. If we can't (no bundle installed), skip the
    // gate — failing closed here would block every command on a fresh
    // install before skills have been populated.
    let framework_root = match crate::dev::paths::resolve_active_framework_root(environment) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let home = environment
        .home
        .clone()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from));

    let opts = DiskBridgeOptions {
        project_root: std::env::current_dir().ok(),
        framework_root: Some(framework_root),
        home,
    };

    let (registry, _loaded) = load_registry(&opts);
    let outcome = check_admission(spec, &registry);

    match outcome {
        AdmissionOutcome::NotRequired => Ok(None),
        AdmissionOutcome::Admitted { .. } => Ok(None),
        AdmissionOutcome::MissingPlaceholderSkill { required } => {
            // Surface the gap but do not block.
            eprintln!(
                "sddk: skill admission gate — placeholder `{required}` is not on disk; \
                 proceeding with warning. See docs/architecture/specs/SPEC-016.md for the \
                 skill contract and docs/architecture/decisions/M7-roadmap.md for follow-up."
            );
            Ok(None)
        }
        AdmissionOutcome::MissingRealSkill {
            required,
            available,
        } => {
            let mut msg =
                format!("skill admission gate: required skill `{required}` is not registered.");
            if !available.is_empty() {
                msg.push_str("\navailable skills:\n");
                for s in &available {
                    msg.push_str(&format!("  - {s}\n"));
                }
            }
            Ok(Some(crate::CommandOutput {
                status: 64,
                stdout: String::new(),
                stderr: msg,
            }))
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(dir: &Path, name: &str, version: &str, description: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let content = format!(
            "---\nname: {name}\ndescription: \"{description}\"\nmetadata:\n  version: \"{version}\"\n---\n\nBody\n"
        );
        std::fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn parse_version_handles_yaml_variants() {
        assert_eq!(parse_version("\"1.0\""), 1);
        assert_eq!(parse_version("\"2\""), 2);
        assert_eq!(parse_version("10"), 10);
        assert_eq!(parse_version(""), 1);
        assert_eq!(parse_version("garbage"), 1);
        assert_eq!(parse_version("\"3.4.5\""), 3);
    }

    #[test]
    fn parse_skill_md_extracts_id_with_scope_prefix() {
        let dir = tempdir();
        write_skill(&dir, "my-skill", "1", "Test skill");
        let loaded = parse_skill_md(&dir.join("SKILL.md"), SkillScope::Framework).unwrap();
        assert_eq!(loaded.definition.id, "framework.my-skill");
        assert_eq!(loaded.definition.version, 1);
        assert!(!loaded.definition.instruction_fragments.is_empty());
    }

    #[test]
    fn parse_skill_md_handles_missing_frontmatter() {
        let dir = tempdir();
        std::fs::write(dir.join("SKILL.md"), "no frontmatter\n").unwrap();
        assert!(parse_skill_md(&dir.join("SKILL.md"), SkillScope::Framework).is_none());
    }

    #[test]
    fn load_registry_dedupes_across_scopes() {
        let root = tempdir();
        // Framework skills live at {framework_root}/skills/<name>/SKILL.md
        let framework = root.join("fw");
        write_skill(
            &framework.join("skills").join("shared"),
            "shared-skill",
            "1",
            "shared",
        );
        // User skills under the canonical XDG path: $HOME/.config/opencode/skills/<name>/
        let home = root.join("home");
        write_skill(
            &home
                .join(".config")
                .join("opencode")
                .join("skills")
                .join("shared"),
            "shared-skill",
            "2",
            "shared user override",
        );
        write_skill(
            &framework.join("skills").join("unique"),
            "unique",
            "1",
            "unique",
        );

        let opts = DiskBridgeOptions {
            project_root: None,
            framework_root: Some(framework),
            home: Some(home),
        };
        let (reg, loaded) = load_registry(&opts);
        // ref_token format is "{id}@v{version}"
        assert!(
            reg.entries()
                .iter()
                .any(|e| e.definition.ref_token() == "framework.shared-skill@v1")
        );
        assert!(
            reg.entries()
                .iter()
                .any(|e| e.definition.ref_token() == "framework.unique@v1")
        );
        // Framework wins precedence — user version (v2) is shadowed.
        assert!(
            !reg.entries()
                .iter()
                .any(|e| e.definition.ref_token() == "user.shared-skill@v2")
        );
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn admission_admits_command_without_required_skills() {
        // Empty registry to keep this test deterministic regardless of
        // any skills present on the developer's machine.
        let reg = SkillRegistry::new();
        let cmd = CommandSpec::new("test", "test command");
        let outcome = check_admission(&cmd, &reg);
        assert_eq!(outcome, AdmissionOutcome::NotRequired);
        assert!(outcome.is_admitted());
    }

    #[test]
    fn admission_warns_on_missing_placeholder_skill() {
        // Empty registry → placeholder is missing on disk → warning.
        let reg = SkillRegistry::new();
        let cmd =
            CommandSpec::new("lint", "validate").with_required_skill("core.contract-review@v1");
        let outcome = check_admission(&cmd, &reg);
        assert!(matches!(
            outcome,
            AdmissionOutcome::MissingPlaceholderSkill { .. }
        ));
        assert!(outcome.is_admitted(), "placeholder warnings do not block");
    }

    #[test]
    fn admission_admits_when_required_skill_present_on_disk() {
        let root = tempdir();
        let framework = root.join("fw");
        write_skill(
            &framework.join("skills").join("contract-review"),
            "contract-review",
            "1",
            "real contract review",
        );
        let (reg, _) = load_registry(&DiskBridgeOptions {
            project_root: None,
            framework_root: Some(framework),
            home: Some(root.join("empty-home")),
        });
        let cmd = CommandSpec::new("lint", "validate")
            .with_required_skill("framework.contract-review@v1");
        let outcome = check_admission(&cmd, &reg);
        assert!(matches!(outcome, AdmissionOutcome::Admitted { .. }));
        assert!(outcome.is_admitted());
    }

    #[test]
    fn admission_fails_closed_for_missing_real_skill() {
        let reg = SkillRegistry::new();
        let cmd = CommandSpec::new("x", "test").with_required_skill("not.a.real.skill@v1");
        let outcome = check_admission(&cmd, &reg);
        assert!(matches!(outcome, AdmissionOutcome::MissingRealSkill { .. }));
        assert!(!outcome.is_admitted());
    }

    #[test]
    fn admission_mixed_placeholder_and_real_blocks() {
        // If a user adds BOTH a placeholder AND a non-placeholder required
        // skill, the placeholder is no longer the only category; the gate
        // treats this as MissingRealSkill because we cannot tell which
        // missing entry caused the failure.
        let reg = SkillRegistry::new();
        let cmd = CommandSpec::new("x", "test")
            .with_required_skill("core.contract-review@v1")
            .with_required_skill("not.real@v1");
        let outcome = check_admission(&cmd, &reg);
        assert!(matches!(outcome, AdmissionOutcome::MissingRealSkill { .. }));
        assert!(!outcome.is_admitted());
    }

    #[test]
    fn admission_empty_registry_admits_no_required_skill() {
        // Confirms `SkillRegistry::new()` is admitted for commands that
        // declare no skills — even with zero skills registered.
        let reg = SkillRegistry::new();
        let cmd = CommandSpec::new("anything", "any command");
        let outcome = check_admission(&cmd, &reg);
        assert_eq!(outcome, AdmissionOutcome::NotRequired);
    }

    #[test]
    fn search_paths_include_framework_and_user_levels() {
        let opts = DiskBridgeOptions {
            project_root: Some(PathBuf::from("/tmp/proj")),
            framework_root: Some(PathBuf::from("/tmp/fw")),
            home: Some(PathBuf::from("/tmp/home")),
        };
        let paths = search_paths(&opts);
        assert!(
            paths
                .iter()
                .any(|(_, p)| p.ends_with("/tmp/proj/.opencode/skills"))
        );
        assert!(
            paths
                .iter()
                .any(|(_, p)| p.ends_with("/tmp/home/.config/opencode/skills"))
        );
        assert!(paths.iter().any(|(_, p)| p.ends_with("/tmp/fw/skills")));
    }

    // Minimal tempdir helper (avoids pulling in `tempfile`).
    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "sddk-skill-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
