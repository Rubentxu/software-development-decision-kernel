// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// orchestration_config — SDDK Configuration Model v1 (single resolver).
//
// Authority of the model:
//   docs/architecture/specs/arch-spec-049-sddk-configuration-model-v1.md
//
// Both the `sddk config resolve` CLI surface and `~/.jcode/bin/sddk-config`
// consume THIS module. There is exactly one resolver: jcode MUST NOT
// independently resolve SDDK configuration.

use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// Policy keys of the v1 model, with their compiled default.
pub const POLICY_KEYS: &[(&str, &str)] = &[
    ("autonomy.auto_advance", "false"),
    ("autonomy.campaign_mode", "false"),
    ("autonomy.stop_only_on_human_gate", "false"),
    ("autonomy.reversible_decisions", "confirm"),
    ("personality.preset", "neutral"),
    ("personality.tone", "neutral"),
    ("personality.humor", "none"),
    ("personality.verbosity", "normal"),
    ("human_feed.enabled", "true"),
    ("human_feed.stage_completion", "normal"),
    ("human_feed.important_findings", "stage_end"),
    ("human_feed.routine_progress", "concise"),
    ("human_feed.persist", "true"),
    ("workflow.selection", "manual"),
    ("workflow.automatic_stage_progression", "false"),
    ("workflow.persist_stage_state", "true"),
    ("parallelism.enabled", "false"),
    ("parallelism.max_agents", "1"),
    ("parallelism.require_file_ownership", "true"),
    ("parallelism.integrate_through_orchestrator", "true"),
    ("verification.characterization_first", "false"),
    ("verification.falsify_new_guards", "false"),
    ("verification.require_named_red_cause", "true"),
    ("verification.mutation_check_for_guards", "false"),
    ("git.local_commit", "true"),
    ("documentation.update_specs", "true"),
    ("documentation.update_adrs", "when_needed"),
    ("documentation.update_roadmap", "true"),
    ("documentation.create_receipts", "true"),
    ("planning.stage_size", "medium"),
    ("research.characterization_first", "false"),
    ("recovery.resume_from_state", "true"),
];

/// Non-overridable system laws. A profile that sets one of these keys is
/// rejected (fail closed).
pub const LAWS: &[(&str, &str)] = &[
    ("git.push", "human_gate"),
    ("git.tag", "human_gate"),
    ("git.release", "human_gate"),
    ("git.history_rewrite", "human_gate"),
    ("security.destructive_actions", "human_gate"),
    ("security.irreversible_external", "human_gate"),
    ("security.secrets_exposure", "human_gate"),
    ("evidence.unverified_green", "forbidden"),
    ("evidence.invented", "forbidden"),
    ("evidence.classes", "mandatory"),
    ("adoption.absent_is", "undeclared"),
    ("adoption.invalid_is", "undeclared"),
    ("adoption.unknown_is", "undeclared"),
    ("adoption.duplicate_is", "undeclared"),
    ("adoption.silent_disable", "forbidden"),
];

/// Keys whose mere presence rejects the profile.
pub const FORBIDDEN_KEYS: &[&str] = &[
    "allow_unverified_green",
    "invent_execution_evidence",
    "ignore_human_gate",
    "suppress_evidence",
    "push_without_permission",
    "disable_human_feed",
];

pub const DEFAULT_PROFILE: &str = "default";

/// Effective adoption mode (closed three-case value).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    On,
    Off,
    Undeclared,
}

impl Mode {
    pub fn canonical(self) -> &'static str {
        match self {
            Mode::On => "on",
            Mode::Off => "off",
            Mode::Undeclared => "undeclared",
        }
    }
}

/// One resolved configuration key with its provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolvedKey {
    pub name: String,
    pub value: String,
    pub source: String,
}

/// The effective orchestration configuration for one identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveConfig {
    pub mode: Mode,
    pub reason: String,
    pub profile: String,
    pub keys: Vec<ResolvedKey>,
}

impl EffectiveConfig {
    pub fn get(&self, name: &str) -> Option<&ResolvedKey> {
        self.keys.iter().find(|k| k.name == name)
    }
}

/// One declaration found in the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexEntry {
    Declared { mode: Mode, profile: Option<String> },
    Duplicate,
    Invalid,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    UnknownProfile(String),
    ExtendsCycle(String),
    MissingProfile(String),
    ExtendsTooDeep,
    LawInProfile { profile: String, key: String },
    ForbiddenKeyInProfile { profile: String, key: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::UnknownProfile(p) => write!(f, "perfil no encontrado: {p}"),
            ConfigError::ExtendsCycle(p) => write!(f, "ciclo de extends en perfiles ('{p}')"),
            ConfigError::MissingProfile(p) => {
                write!(f, "el indice declara el perfil '{p}' pero no existe")
            }
            ConfigError::ExtendsTooDeep => write!(f, "cadena de extends demasiado profunda"),
            ConfigError::LawInProfile { profile, key } => write!(
                f,
                "perfil '{profile}' intenta fijar la ley '{key}' (fail-closed)"
            ),
            ConfigError::ForbiddenKeyInProfile { profile, key } => write!(
                f,
                "perfil '{profile}' usa la clave prohibida '{key}' (fail-closed)"
            ),
        }
    }
}

/// Parse the mode index. Format v1: `<id> <mode> [profile]`.
/// Backward compatible: two columns mean `profile = None` (inherit).
/// Duplicate ids are reported as `Duplicate` (L5), never an arbitrary winner.
pub fn parse_index(text: &str) -> BTreeMap<String, IndexEntry> {
    let mut out: BTreeMap<String, IndexEntry> = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 2 {
            continue;
        }
        let id = f[0].to_string();
        let entry = match f[1] {
            "on" | "off" => {
                let mode = if f[1] == "on" { Mode::On } else { Mode::Off };
                let profile = match f.get(2) {
                    Some(p) if *p != "-" => Some((*p).to_string()),
                    _ => None,
                };
                IndexEntry::Declared { mode, profile }
            }
            _ => IndexEntry::Invalid,
        };
        match out.get(&id) {
            Some(_) => {
                out.insert(id, IndexEntry::Duplicate);
            }
            None => {
                out.insert(id, entry);
            }
        }
    }
    out
}

/// Parse a profile: flat dotted keys `key: value`, `#` comments.
pub fn parse_profile(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if k.is_empty() {
            continue;
        }
        out.push((k.to_string(), v.to_string()));
    }
    out
}

fn load_profile(dir: &Path, name: &str) -> Result<Vec<(String, String)>, ConfigError> {
    let path = dir.join(format!("{name}.yaml"));
    let text = std::fs::read_to_string(&path)
        .map_err(|_| ConfigError::UnknownProfile(name.to_string()))?;
    Ok(parse_profile(&text))
}

fn extends_of(entries: &[(String, String)]) -> Option<String> {
    entries
        .iter()
        .find(|(k, _)| k == "extends")
        .map(|(_, v)| v.clone())
}

/// Build the inheritance chain, base first. Cycles / missing bases fail closed.
pub fn profile_chain(dir: &Path, name: &str) -> Result<Vec<String>, ConfigError> {
    let mut chain: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut cur = Some(name.to_string());
    while let Some(p) = cur {
        if p.is_empty() || p == "-" {
            break;
        }
        if seen.contains(&p) {
            return Err(ConfigError::ExtendsCycle(p));
        }
        if seen.len() >= 8 {
            return Err(ConfigError::ExtendsTooDeep);
        }
        seen.push(p.clone());
        chain.insert(0, p.clone());
        let entries = load_profile(dir, &p)?;
        cur = extends_of(&entries);
    }
    if !chain.iter().any(|p| p == DEFAULT_PROFILE) {
        chain.insert(0, DEFAULT_PROFILE.to_string());
    }
    Ok(chain)
}

fn validate_profile(name: &str, entries: &[(String, String)]) -> Result<(), ConfigError> {
    for (k, _) in entries {
        if k == "profile" || k == "version" || k == "extends" {
            continue;
        }
        if LAWS.iter().any(|(lk, _)| lk == k) {
            return Err(ConfigError::LawInProfile {
                profile: name.to_string(),
                key: k.clone(),
            });
        }
        if FORBIDDEN_KEYS.contains(&k.as_str()) {
            return Err(ConfigError::ForbiddenKeyInProfile {
                profile: name.to_string(),
                key: k.clone(),
            });
        }
    }
    Ok(())
}

/// Resolve the effective configuration for one identity.
///
/// `index_text` is `None` when the index file is absent (→ UNDECLARED,
/// `index-absent`). The workspace declaration wins over the project one (L1).
pub fn resolve(
    index_text: Option<&str>,
    profile_dir: &Path,
    project_id: &str,
    workspace_id: &str,
) -> Result<EffectiveConfig, ConfigError> {
    let index = index_text.map(parse_index).unwrap_or_default();

    let mut mode = Mode::Undeclared;
    let mut reason = if index_text.is_none() {
        "index-absent".to_string()
    } else {
        "no-entry".to_string()
    };
    let mut profile = DEFAULT_PROFILE.to_string();

    for (scope, id) in [("workspace", workspace_id), ("project", project_id)] {
        if id.is_empty() {
            continue;
        }
        match index.get(id) {
            Some(IndexEntry::Declared {
                mode: m,
                profile: p,
            }) => {
                mode = *m;
                reason = format!("declared:{scope}");
                profile = match (m, p) {
                    (Mode::On, Some(p)) => p.clone(),
                    (Mode::On, None) => DEFAULT_PROFILE.to_string(),
                    // OFF (and any declared mode) yields an inert profile.
                    _ => "-".to_string(),
                };
                break;
            }
            Some(IndexEntry::Duplicate) => {
                return Ok(EffectiveConfig {
                    mode: Mode::Undeclared,
                    reason: format!("duplicate:{scope}"),
                    profile: DEFAULT_PROFILE.to_string(),
                    keys: policy_rows(DEFAULT_PROFILE, &[]),
                });
            }
            Some(IndexEntry::Invalid) => {
                return Ok(EffectiveConfig {
                    mode: Mode::Undeclared,
                    reason: format!("invalid-value:{scope}"),
                    profile: DEFAULT_PROFILE.to_string(),
                    keys: policy_rows(DEFAULT_PROFILE, &[]),
                });
            }
            None => {}
        }
    }

    if profile == "-" {
        return Ok(EffectiveConfig {
            mode,
            reason,
            profile,
            keys: Vec::new(),
        });
    }

    let chain = profile_chain(profile_dir, &profile)?;
    let mut merged: Vec<ResolvedKey> = Vec::new();
    let push = |name: &str, value: &str, source: &str, merged: &mut Vec<ResolvedKey>| {
        if let Some(slot) = merged.iter_mut().find(|k| k.name == name) {
            slot.value = value.to_string();
            slot.source = source.to_string();
        } else {
            merged.push(ResolvedKey {
                name: name.to_string(),
                value: value.to_string(),
                source: source.to_string(),
            });
        }
    };
    for p in &chain {
        let entries = load_profile(profile_dir, p)?;
        validate_profile(p, &entries)?;
        for (k, v) in entries {
            if k == "profile" || k == "version" || k == "extends" {
                continue;
            }
            push(&k, &v, &format!("profile:{p}"), &mut merged);
        }
    }
    // Policy defaults for keys nobody set.
    for (k, v) in POLICY_KEYS {
        if !merged.iter().any(|m| m.name == *k) {
            merged.push(ResolvedKey {
                name: (*k).to_string(),
                value: (*v).to_string(),
                source: "builtin".to_string(),
            });
        }
    }
    // Laws override everything.
    for (k, v) in LAWS {
        if k.starts_with("adoption.") {
            continue;
        }
        push(k, v, "system-law", &mut merged);
    }
    merged.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(EffectiveConfig {
        mode,
        reason,
        profile,
        keys: merged,
    })
}

/// Rewrite the index with `id` set to `mode` (and optional `profile`).
///
/// Replacement, never append: two entries for the same id would be `L5`
/// (duplicate) and would fall back to UNDECLARED. Pure string transform.
pub fn upsert_declaration(index_text: &str, id: &str, mode: Mode, profile: Option<&str>) -> String {
    let mut out = String::new();
    for line in index_text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 2 && f[0] == id {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    match (mode, profile) {
        (_, Some(p)) if p != "-" => out.push_str(&format!("{id} {} {p}\n", mode.canonical())),
        _ => out.push_str(&format!("{id} {}\n", mode.canonical())),
    }
    out
}

/// Rewrite the index with `id` removed (returns to inheritance / UNDECLARED).
pub fn remove_declaration(index_text: &str, id: &str) -> String {
    let mut out = String::new();
    for line in index_text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() >= 2 && f[0] == id {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Policy rows with defaults only (used for the UNDECLARED early returns).
fn policy_rows(profile: &str, extra: &[(String, String)]) -> Vec<ResolvedKey> {
    let mut rows: Vec<ResolvedKey> = POLICY_KEYS
        .iter()
        .map(|(k, v)| ResolvedKey {
            name: (*k).to_string(),
            value: (*v).to_string(),
            source: "builtin".to_string(),
        })
        .collect();
    for (k, v) in extra {
        rows.push(ResolvedKey {
            name: k.clone(),
            value: v.clone(),
            source: format!("profile:{profile}"),
        });
    }
    for (k, v) in LAWS {
        if k.starts_with("adoption.") {
            continue;
        }
        rows.push(ResolvedKey {
            name: (*k).to_string(),
            value: (*v).to_string(),
            source: "system-law".to_string(),
        });
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Unique scratch profile dir per test (no external deps).
    fn profdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sddk-cfg-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn write_profile(dir: &Path, name: &str, body: &str) {
        fs::write(dir.join(format!("{name}.yaml")), body).unwrap();
    }
    fn default_profile(dir: &Path) {
        write_profile(
            dir,
            "default",
            "profile: default\nversion: 1\nautonomy.auto_advance: false\n",
        );
    }
    fn bender(dir: &Path) {
        write_profile(
            dir,
            "bender",
            "profile: bender\nversion: 1\nextends: default\nautonomy.auto_advance: true\npersonality.preset: bender\n",
        );
    }

    #[test]
    fn index_parses_modes_and_profiles() {
        let idx = parse_index("p-1 on bender\np-2 off\nw-1 on -\n");
        assert_eq!(
            idx.get("p-1"),
            Some(&IndexEntry::Declared {
                mode: Mode::On,
                profile: Some("bender".into())
            })
        );
        // two-column entry: backward compatible, profile inherits
        assert_eq!(
            idx.get("p-2"),
            Some(&IndexEntry::Declared {
                mode: Mode::Off,
                profile: None
            })
        );
        // explicit `-`: inherit
        assert_eq!(
            idx.get("w-1"),
            Some(&IndexEntry::Declared {
                mode: Mode::On,
                profile: None
            })
        );
    }

    #[test]
    fn duplicate_and_invalid_are_undeclared_never_off() {
        let idx = parse_index("p-1 on\np-1 off\n");
        assert_eq!(idx.get("p-1"), Some(&IndexEntry::Duplicate));
        let idx = parse_index("p-2 banana\n");
        assert_eq!(idx.get("p-2"), Some(&IndexEntry::Invalid));
    }

    #[test]
    fn l1_workspace_wins_and_profile_follows_it() {
        let dir = profdir("l1");
        default_profile(&dir);
        bender(&dir);
        let idx = "p-1 on default\nw-1 on bender\n";
        let cfg = resolve(Some(idx), &dir, "p-1", "w-1").unwrap();
        assert_eq!(cfg.mode, Mode::On);
        assert_eq!(cfg.reason, "declared:workspace");
        assert_eq!(cfg.profile, "bender");
        assert_eq!(cfg.get("autonomy.auto_advance").unwrap().value, "true");
    }

    #[test]
    fn undeclared_workspace_inherits_project_on_without_asking() {
        let dir = profdir("inherit");
        default_profile(&dir);
        let cfg = resolve(Some("p-1 on\n"), &dir, "p-1", "w-none").unwrap();
        assert_eq!(cfg.mode, Mode::On);
        assert_eq!(cfg.reason, "declared:project");
        assert_eq!(cfg.profile, "default");
    }

    #[test]
    fn absent_index_is_undeclared_index_absent() {
        let dir = profdir("absent");
        default_profile(&dir);
        let cfg = resolve(None, &dir, "p-1", "w-1").unwrap();
        assert_eq!(cfg.mode, Mode::Undeclared);
        assert_eq!(cfg.reason, "index-absent");
        assert_eq!(cfg.profile, "default");
    }

    #[test]
    fn duplicate_declaration_is_undeclared() {
        let dir = profdir("dup");
        default_profile(&dir);
        let cfg = resolve(Some("w-1 on\nw-1 off\n"), &dir, "p-1", "w-1").unwrap();
        assert_eq!(cfg.mode, Mode::Undeclared);
        assert_eq!(cfg.reason, "duplicate:workspace");
    }

    #[test]
    fn off_yields_inert_profile() {
        let dir = profdir("off");
        default_profile(&dir);
        let cfg = resolve(Some("p-1 off\n"), &dir, "p-1", "w-1").unwrap();
        assert_eq!(cfg.mode, Mode::Off);
        assert_eq!(cfg.profile, "-");
        assert!(cfg.keys.is_empty());
    }

    #[test]
    fn laws_never_come_from_a_profile() {
        let dir = profdir("laws");
        default_profile(&dir);
        bender(&dir);
        let cfg = resolve(Some("p-1 on bender\n"), &dir, "p-1", "w-1").unwrap();
        for key in [
            "git.push",
            "git.tag",
            "git.release",
            "git.history_rewrite",
            "evidence.unverified_green",
            "evidence.invented",
        ] {
            let row = cfg.get(key).unwrap_or_else(|| panic!("missing {key}"));
            assert_eq!(row.source, "system-law", "{key} must be a system law");
        }
        assert_eq!(cfg.get("git.push").unwrap().value, "human_gate");
        assert_eq!(
            cfg.get("evidence.unverified_green").unwrap().value,
            "forbidden"
        );
    }

    #[test]
    fn profile_setting_a_law_is_rejected() {
        let dir = profdir("evillaw");
        default_profile(&dir);
        write_profile(
            &dir,
            "evil",
            "profile: evil\nversion: 1\ngit.push: allowed\n",
        );
        let err = resolve(Some("p-1 on evil\n"), &dir, "p-1", "w-1").unwrap_err();
        assert!(
            matches!(err, ConfigError::LawInProfile { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn forbidden_key_is_rejected() {
        let dir = profdir("evilkey");
        default_profile(&dir);
        write_profile(
            &dir,
            "evil",
            "profile: evil\nversion: 1\nallow_unverified_green: true\n",
        );
        let err = resolve(Some("p-1 on evil\n"), &dir, "p-1", "w-1").unwrap_err();
        assert!(
            matches!(err, ConfigError::ForbiddenKeyInProfile { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn extends_cycle_is_rejected() {
        let dir = profdir("cycle");
        default_profile(&dir);
        write_profile(&dir, "a", "profile: a\nversion: 1\nextends: b\n");
        write_profile(&dir, "b", "profile: b\nversion: 1\nextends: a\n");
        let err = resolve(Some("p-1 on a\n"), &dir, "p-1", "w-1").unwrap_err();
        assert!(matches!(err, ConfigError::ExtendsCycle(_)), "got {err:?}");
    }

    #[test]
    fn declared_but_missing_profile_is_rejected() {
        let dir = profdir("missing");
        default_profile(&dir);
        let err = resolve(Some("p-1 on ghost\n"), &dir, "p-1", "w-1").unwrap_err();
        assert!(matches!(err, ConfigError::UnknownProfile(_)), "got {err:?}");
    }

    #[test]
    fn unset_policy_keys_fall_back_to_builtin_defaults() {
        let dir = profdir("defaults");
        default_profile(&dir);
        let cfg = resolve(Some("p-1 on\n"), &dir, "p-1", "w-1").unwrap();
        let row = cfg.get("parallelism.max_agents").unwrap();
        assert_eq!(row.value, "1");
        assert_eq!(row.source, "builtin");
        // every policy key is present in the resolved view
        for (k, _) in POLICY_KEYS {
            assert!(
                cfg.get(k).is_some(),
                "policy key {k} missing from resolve()"
            );
        }
    }

    #[test]
    fn profile_values_win_over_defaults_and_report_source() {
        let dir = profdir("source");
        default_profile(&dir);
        bender(&dir);
        let cfg = resolve(Some("p-1 on bender\n"), &dir, "p-1", "w-1").unwrap();
        let adv = cfg.get("autonomy.auto_advance").unwrap();
        assert_eq!(adv.value, "true");
        assert_eq!(adv.source, "profile:bender");
        let preset = cfg.get("personality.preset").unwrap();
        assert_eq!(preset.source, "profile:bender");
    }
}

#[cfg(test)]
mod writer_tests {
    use super::*;

    #[test]
    fn upsert_replaces_never_appends() {
        let a = upsert_declaration("p-1 on\nw-1 on\n", "p-1", Mode::Off, None);
        assert_eq!(a.matches("p-1 ").count(), 1, "exactly one p-1 line");
        assert!(a.contains("p-1 off"));
        assert!(a.contains("w-1 on"));
        let b = upsert_declaration(&a, "p-1", Mode::On, Some("bender"));
        assert_eq!(b.matches("p-1 ").count(), 1);
        assert!(b.contains("p-1 on bender"));
    }

    #[test]
    fn upsert_with_dash_omits_the_profile_column() {
        let a = upsert_declaration("", "w-1", Mode::On, Some("-"));
        assert_eq!(a, "w-1 on\n");
    }

    #[test]
    fn remove_restores_inheritance() {
        let a = remove_declaration("p-1 on bender\nw-1 off\n", "w-1");
        assert_eq!(a, "p-1 on bender\n");
        let b = remove_declaration(&a, "p-1");
        assert_eq!(b, "");
    }
}
