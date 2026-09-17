//! `sddk config explain` — resolve configuration values and report their
//! precedence source chain (M6.1, SPEC-012 / arch-spec-012).
//!
//! Precedence (highest → lowest):
//!   1. CLI flags (runtime)
//!   2. `SDDK_*` / `XDG_*` environment variables
//!   3. `.sddk/config.toml` (scoped, cwd or parents)
//!   4. `sddk.toml` (project, cwd or parents)
//!   5. Compiled defaults
//!
//! Scoped configuration inherits from the project file and can only override
//! keys it declares. Unknown keys fail explicitly (never silently drift).
//!
//! Output is JSON or Text. JSON is machine-readable; text is human-friendly.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at
//! module scope.

#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use clap::Subcommand;
use serde::Serialize;

use crate::{CliEnvironment, CommandOutput, OutputFormat};
use sddk_engine::orchestration_config::{self, EffectiveConfig, Mode, ResolvedKey};

/// Compiled defaults for the project configuration keys declared by SPEC-012.
///
/// `None` means "no compiled default" (the key is simply unset).
const CONFIG_KEYS: &[(&str, Option<&str>)] = &[
    ("project.kind", None),
    ("workflow.default_target", Some("change")),
    ("packs.use", None),
    ("memory.hot_budget", Some("2000")),
    ("memory.context_budget", Some("16000")),
    ("policy.profile", Some("team-default")),
];

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the configuration precedence chain (highest → lowest).
    Explain {
        /// Optional configuration key (e.g. `memory.hot_budget`). When omitted,
        /// every known key is reported.
        key: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Print the EFFECTIVE SDDK orchestration configuration (model v1) with the
    /// SOURCE of every value. This is the single resolver: jcode consumes it.
    Resolve {
        /// Working directory whose identity is resolved (default: cwd).
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Print the non-overridable laws and forbidden keys of the model.
    Laws {
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// List the available SDDK profiles.
    Profiles {
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Declare the adoption mode (and optionally the profile) for an identity.
    Set {
        /// `on` | `off`.
        mode: String,
        /// Declare at project scope (default).
        #[arg(long, conflicts_with = "workspace")]
        project: bool,
        /// Declare at workspace scope (overrides project).
        #[arg(long, conflicts_with = "project")]
        workspace: bool,
        /// Profile name, or `-` to inherit. Must exist under the profiles dir.
        #[arg(long)]
        profile: Option<String>,
        /// Working directory whose identity is used (default: cwd).
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    /// Remove this identity's declaration (returns to inheritance / UNDECLARED).
    Clear {
        /// Clear at project scope (default).
        #[arg(long, conflicts_with = "workspace")]
        project: bool,
        /// Clear at workspace scope.
        #[arg(long, conflicts_with = "project")]
        workspace: bool,
        /// Working directory whose identity is used (default: cwd).
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
}

/// One candidate layer in a key's precedence chain.
#[derive(Debug, Serialize)]
pub struct ConfigLayer {
    pub layer: String,
    pub present: bool,
    pub value: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigKeyReport {
    pub name: String,
    pub source: String,
    pub resolved: Option<String>,
    pub note: String,
    /// Full candidate chain, highest precedence first.
    pub chain: Vec<ConfigLayer>,
}

#[derive(Debug, Serialize)]
pub struct ConfigReport {
    pub precedence: Vec<String>,
    pub keys: Vec<ConfigKeyReport>,
}

fn precedence_bands() -> Vec<String> {
    vec![
        "1. CLI flags (runtime)".into(),
        "2. SDDK_* / XDG_* environment variables".into(),
        "3. .sddk/config.toml (scoped; cwd or parents)".into(),
        "4. sddk.toml (project; cwd or parents)".into(),
        "5. Compiled defaults".into(),
    ]
}

fn env_key_for(name: &str) -> String {
    format!("SDDK_{}", name.replace('.', "_").to_uppercase())
}

/// Render the precedence chain for the current environment.
pub fn render_config_report(env: &CliEnvironment, format: OutputFormat) -> CommandOutput {
    render_config_report_at(
        env,
        format,
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    )
}

/// Render the precedence chain resolving files relative to `root`.
pub fn render_config_report_at(
    env: &CliEnvironment,
    format: OutputFormat,
    root: &Path,
) -> CommandOutput {
    let mut keys: Vec<ConfigKeyReport> = Vec::new();

    // SPEC-012 configuration keys with full precedence chains.
    for (name, default) in CONFIG_KEYS {
        keys.push(resolve_key(name, *default, root));
    }
    // Environment pseudo-keys (backward compatible with the M6.1 surface).
    for (name, resolved, note) in [
        ("HOME", env.home.as_ref().map(path_to_string), "process env"),
        (
            "XDG_DATA_HOME",
            env.data_home.as_ref().map(path_to_string),
            "process env",
        ),
        (
            "SDDK_DATA_DIR",
            env.sddk_data_dir.as_ref().map(path_to_string),
            "process env (overrides XDG_DATA_HOME)",
        ),
        (
            "XDG_STATE_HOME",
            env.state_home.as_ref().map(path_to_string),
            "process env",
        ),
        (
            "XDG_CACHE_HOME",
            env.cache_home.as_ref().map(path_to_string),
            "process env",
        ),
        (
            "SDDK_ACTOR",
            env.sddk_actor.clone(),
            "process env (overrides USER)",
        ),
        ("USER", env.user.clone(), "process env (default actor)"),
    ] {
        keys.push(ConfigKeyReport {
            name: name.to_string(),
            source: "process_env_or_file".to_string(),
            resolved,
            note: note.to_string(),
            chain: vec![],
        });
    }

    let report = ConfigReport {
        precedence: precedence_bands(),
        keys,
    };
    emit(report, format)
}

/// `explain <key>`: resolve one declared config key or fail explicitly on an
/// unknown key (never silently drift).
pub fn render_config_key_report(format: OutputFormat, root: &Path, key: &str) -> CommandOutput {
    if let Some((name, default)) = CONFIG_KEYS.iter().find(|(n, _)| *n == key) {
        let report = ConfigReport {
            precedence: precedence_bands(),
            keys: vec![resolve_key(name, *default, root)],
        };
        return emit(report, format);
    }
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!(
            "error: unknown configuration key '{key}'; known keys: {}\n",
            CONFIG_KEYS
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn resolve_key(name: &str, default: Option<&str>, root: &Path) -> ConfigKeyReport {
    resolve_key_with_env(name, default, &nonempty_env, root)
}

fn resolve_key_with_env(
    name: &str,
    default: Option<&str>,
    env_lookup: &dyn Fn(&str) -> Option<String>,
    root: &Path,
) -> ConfigKeyReport {
    let (scoped_path, scoped) = load_config_file(root, ".sddk/config.toml");
    let (project_path, project) = load_config_file(root, "sddk.toml");

    let env_name = env_key_for(name);
    let env_value = env_lookup(&env_name);
    let scoped_value = scoped.as_ref().and_then(|v| toml_lookup(v, name));
    let project_value = project.as_ref().and_then(|v| toml_lookup(v, name));

    let chain = vec![
        ConfigLayer {
            layer: "cli".into(),
            present: false,
            value: None,
        },
        ConfigLayer {
            layer: format!("env:{env_name}"),
            present: env_value.is_some(),
            value: env_value.clone(),
        },
        ConfigLayer {
            layer: scoped_path
                .as_ref()
                .map(|p| format!(".sddk/config.toml ({})", p.display()))
                .unwrap_or_else(|| ".sddk/config.toml".into()),
            present: scoped_value.is_some(),
            value: scoped_value.clone(),
        },
        ConfigLayer {
            layer: project_path
                .as_ref()
                .map(|p| format!("sddk.toml ({})", p.display()))
                .unwrap_or_else(|| "sddk.toml".into()),
            present: project_value.is_some(),
            value: project_value.clone(),
        },
        ConfigLayer {
            layer: "default".into(),
            present: default.is_some(),
            value: default.map(str::to_owned),
        },
    ];

    let (source, resolved) = if let Some(v) = env_value {
        ("env", Some(v))
    } else if let Some(v) = scoped_value {
        ("scoped_config", Some(v))
    } else if let Some(v) = project_value {
        ("project_config", Some(v))
    } else {
        ("default", default.map(str::to_owned))
    };

    ConfigKeyReport {
        name: name.to_string(),
        source: source.to_string(),
        resolved,
        note: format!(
            "precedence: cli > env > .sddk/config.toml > sddk.toml > default ({env_name})"
        ),
        chain,
    }
}

/// Walk up from `root` looking for `rel`, returning `(path, parsed)`.
fn load_config_file(root: &Path, rel: &str) -> (Option<PathBuf>, Option<toml::Value>) {
    let mut dir = root.to_path_buf();
    for _ in 0..6 {
        let candidate = dir.join(rel);
        if candidate.is_file()
            && let Ok(text) = std::fs::read_to_string(&candidate)
            && let Ok(value) = text.parse::<toml::Value>()
        {
            return (Some(candidate), Some(value));
        }
        if !dir.pop() {
            break;
        }
    }
    (None, None)
}

fn toml_lookup(value: &toml::Value, dotted: &str) -> Option<String> {
    let mut cur = value;
    for part in dotted.split('.') {
        cur = cur.get(part)?;
    }
    Some(toml_scalar_to_string(cur))
}

fn toml_scalar_to_string(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(a) => a
            .iter()
            .map(toml_scalar_to_string)
            .collect::<Vec<_>>()
            .join(","),
        other => other.to_string(),
    }
}

fn nonempty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn emit(report: ConfigReport, format: OutputFormat) -> CommandOutput {
    match format {
        OutputFormat::Json => {
            let stdout = serde_json::to_string_pretty(&report).unwrap_or_default();
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str("Configuration precedence (highest → lowest):\n");
            for line in &report.precedence {
                out.push_str(&format!("  {line}\n"));
            }
            out.push('\n');
            out.push_str("Resolved keys:\n");
            for k in &report.keys {
                let resolved = k.resolved.as_deref().unwrap_or("(unset)");
                out.push_str(&format!(
                    "  {:<28} = {:<28} [{}]\n",
                    k.name, resolved, k.source
                ));
                for layer in &k.chain {
                    let marker = if layer.present { "set" } else { "-" };
                    let value = layer.value.as_deref().unwrap_or("-");
                    out.push_str(&format!(
                        "      {:<8} {:<40} {}\n",
                        marker, layer.layer, value
                    ));
                }
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}

#[allow(clippy::ptr_arg)]
fn path_to_string(p: &PathBuf) -> String {
    p.display().to_string()
}

/// Run the `config` subcommand dispatcher.
pub(crate) fn run_config(command: ConfigCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        ConfigCommand::Explain { key, format } => {
            let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            match key {
                Some(key) => render_config_key_report(format, &root, &key),
                None => render_config_report_at(environment, format, &root),
            }
        }
        ConfigCommand::Resolve { cwd, format } => {
            let root = cwd
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            render_orchestration_resolve(environment, &root, format)
        }
        ConfigCommand::Laws { format } => render_orchestration_laws(format),
        ConfigCommand::Profiles { format } => render_orchestration_profiles(format),
        ConfigCommand::Set {
            mode,
            project: _,
            workspace,
            profile,
            cwd,
        } => run_config_set(environment, cwd, workspace, &mode, profile),
        ConfigCommand::Clear {
            project: _,
            workspace,
            cwd,
        } => run_config_clear(environment, cwd, workspace),
    }
}

fn config_root(cwd: Option<PathBuf>) -> PathBuf {
    cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn identity_for(root: &Path, workspace_scope: bool) -> Result<String, CommandOutput> {
    match crate::resolve_project_ids(root, ".", None, None) {
        Ok((p, w)) => Ok(if workspace_scope { w } else { p }),
        Err(err) => Err(CommandOutput {
            status: 2,
            stdout: String::new(),
            stderr: format!("config: no se pudo resolver la identidad: {err}\n"),
        }),
    }
}

fn run_config_set(
    environment: &CliEnvironment,
    cwd: Option<PathBuf>,
    workspace_scope: bool,
    mode: &str,
    profile: Option<String>,
) -> CommandOutput {
    let root = config_root(cwd);
    let mode = match mode {
        "on" => Mode::On,
        "off" => Mode::Off,
        other => {
            return CommandOutput {
                status: 2,
                stdout: String::new(),
                stderr: format!("error: valor invalido '{other}' (on|off)\n"),
            };
        }
    };
    if let Some(p) = &profile
        && p != "-"
        && !profiles_dir(environment)
            .join(format!("{p}.yaml"))
            .is_file()
    {
        return CommandOutput {
            status: 2,
            stdout: String::new(),
            stderr: format!("error: perfil desconocido '{p}' (ver: sddk config profiles)\n"),
        };
    }
    let id = match identity_for(&root, workspace_scope) {
        Ok(id) => id,
        Err(out) => return out,
    };
    let path = mode_index_path(environment);
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let next = orchestration_config::upsert_declaration(&current, &id, mode, profile.as_deref());
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(err) = std::fs::write(&path, next) {
        return CommandOutput {
            status: 2,
            stdout: String::new(),
            stderr: format!(
                "error: no se pudo escribir {}: {err}\n",
                path.to_string_lossy()
            ),
        };
    }
    let scope = if workspace_scope {
        "workspace"
    } else {
        "project"
    };
    let suffix = profile.map(|p| format!(" profile={p}")).unwrap_or_default();
    CommandOutput {
        status: 0,
        stdout: format!("set {id} ({scope}){}: {}\n", suffix, mode.canonical()),
        stderr: String::new(),
    }
}

fn run_config_clear(
    environment: &CliEnvironment,
    cwd: Option<PathBuf>,
    workspace_scope: bool,
) -> CommandOutput {
    let root = config_root(cwd);
    let id = match identity_for(&root, workspace_scope) {
        Ok(id) => id,
        Err(out) => return out,
    };
    let path = mode_index_path(environment);
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let next = orchestration_config::remove_declaration(&current, &id);
    if let Err(err) = std::fs::write(&path, next) {
        return CommandOutput {
            status: 2,
            stdout: String::new(),
            stderr: format!(
                "error: no se pudo escribir {}: {err}\n",
                path.to_string_lossy()
            ),
        };
    }
    let scope = if workspace_scope {
        "workspace"
    } else {
        "project"
    };
    CommandOutput {
        status: 0,
        stdout: format!("clear {id} ({scope})\n"),
        stderr: String::new(),
    }
}

// ── SDDK orchestration configuration (arch-spec-049) ────────────────────────

/// `SDDK_PROFILE_DIR` or `~/.config/sddk/profiles`.
fn profiles_dir(environment: &CliEnvironment) -> PathBuf {
    if let Some(dir) = nonempty_env("SDDK_PROFILE_DIR") {
        return PathBuf::from(dir);
    }
    match &environment.home {
        Some(home) => home.join(".config/sddk/profiles"),
        None => PathBuf::from(".config/sddk/profiles"),
    }
}

/// `SDDK_MODE_INDEX` or `<sddk_data_root>/mode-index`.
fn mode_index_path(environment: &CliEnvironment) -> PathBuf {
    if let Some(path) = nonempty_env("SDDK_MODE_INDEX") {
        return PathBuf::from(path);
    }
    match crate::sddk_data_root(environment) {
        Ok(root) => root.join("mode-index"),
        Err(_) => PathBuf::from("mode-index"),
    }
}

#[derive(Debug, Serialize)]
struct OrchestrationIdentity {
    project: String,
    workspace: String,
}

#[derive(Debug, Serialize)]
struct OrchestrationConfigReport {
    mode: String,
    reason: String,
    profile: String,
    identity: OrchestrationIdentity,
    keys: Vec<ResolvedKey>,
}

/// Build the effective-config report without touching process env or identity.
/// `render_orchestration_resolve` resolves identity and delegates here.
fn orchestration_report(
    index_text: Option<&str>,
    profile_dir: &Path,
    project: &str,
    workspace: &str,
) -> Result<OrchestrationConfigReport, String> {
    let effective = orchestration_config::resolve(index_text, profile_dir, project, workspace)
        .map_err(|e| e.to_string())?;
    Ok(OrchestrationConfigReport {
        mode: effective.mode.canonical().to_string(),
        reason: effective.reason.clone(),
        profile: effective.profile.clone(),
        identity: OrchestrationIdentity {
            project: project.to_string(),
            workspace: workspace.to_string(),
        },
        keys: effective.keys.clone(),
    })
}

fn render_orchestration_resolve(
    environment: &CliEnvironment,
    root: &Path,
    format: OutputFormat,
) -> CommandOutput {
    let (project, workspace) = match crate::resolve_project_ids(root, ".", None, None) {
        Ok(ids) => ids,
        Err(err) => {
            return CommandOutput {
                status: 2,
                stdout: String::new(),
                stderr: format!("config resolve failed: could not resolve identity: {err}\n"),
            };
        }
    };
    let index_path = mode_index_path(environment);
    let index_text = std::fs::read_to_string(&index_path).ok();
    let profile_dir = profiles_dir(environment);
    let report =
        match orchestration_report(index_text.as_deref(), &profile_dir, &project, &workspace) {
            Ok(report) => report,
            Err(err) => {
                return CommandOutput {
                    status: 2,
                    stdout: String::new(),
                    stderr: format!("config resolve failed: {err}\n"),
                };
            }
        };
    let effective = EffectiveConfig {
        mode: if report.mode == "on" {
            Mode::On
        } else if report.mode == "off" {
            Mode::Off
        } else {
            Mode::Undeclared
        },
        reason: report.reason.clone(),
        profile: report.profile.clone(),
        keys: report.keys.clone(),
    };
    match format {
        OutputFormat::Json => {
            let stdout = serde_json::to_string_pretty(&report).unwrap_or_default();
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str(&format!("mode      {}\n", effective.mode.canonical()));
            out.push_str(&format!("reason    {}\n", effective.reason));
            out.push_str(&format!("profile   {}\n", effective.profile));
            out.push_str(&format!("project   {project}\n"));
            out.push_str(&format!("workspace {workspace}\n\n"));
            out.push_str(&format!("{:<42} {:<16} {}\n", "KEY", "VALUE", "SOURCE"));
            for k in &effective.keys {
                out.push_str(&format!("{:<42} {:<16} {}\n", k.name, k.value, k.source));
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}

fn render_orchestration_laws(format: OutputFormat) -> CommandOutput {
    match format {
        OutputFormat::Json => {
            let laws: Vec<serde_json::Value> = orchestration_config::LAWS
                .iter()
                .map(|(k, v)| serde_json::json!({"key": k, "value": v}))
                .collect();
            let body = serde_json::json!({
                "laws": laws,
                "forbidden_keys": orchestration_config::FORBIDDEN_KEYS,
            });
            CommandOutput {
                status: 0,
                stdout: serde_json::to_string_pretty(&body).unwrap_or_default(),
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str("Non-overridable laws (arch-spec-049 §5):\n");
            for (k, v) in orchestration_config::LAWS {
                out.push_str(&format!("  {:<34} {}\n", k, v));
            }
            out.push_str("\nForbidden profile keys (presence rejects the profile):\n");
            out.push_str(&format!(
                "  {}\n",
                orchestration_config::FORBIDDEN_KEYS.join(" ")
            ));
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}

fn render_orchestration_profiles(format: OutputFormat) -> CommandOutput {
    let dir = match nonempty_env("SDDK_PROFILE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => match std::env::var("HOME") {
            Ok(home) => PathBuf::from(home).join(".config/sddk/profiles"),
            Err(_) => PathBuf::from(".config/sddk/profiles"),
        },
    };
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("yaml")
                && let Some(stem) = p.file_stem().and_then(|x| x.to_str())
            {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    match format {
        OutputFormat::Json => CommandOutput {
            status: 0,
            stdout: serde_json::to_string_pretty(&serde_json::json!({
                "dir": dir.to_string_lossy(),
                "profiles": names,
            }))
            .unwrap_or_default(),
            stderr: String::new(),
        },
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str(&format!("profiles dir: {}\n", dir.to_string_lossy()));
            for n in &names {
                out.push_str(&format!("  {n}\n"));
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn precedence_lists_five_bands() {
        let out = render_config_report_at(
            &CliEnvironment::default(),
            OutputFormat::Text,
            Path::new("/nonexistent"),
        );
        assert!(out.stdout.contains("precedence"));
        assert!(out.stdout.contains("Compiled defaults"));
    }

    #[test]
    fn scoped_overrides_project_but_inherits_undeclared() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            &root.join("sddk.toml"),
            "[project]\nkind = \"rust-service\"\n[memory]\nhot_budget = 1000\ncontext_budget = 8000\n",
        );
        write(
            &root.join(".sddk/config.toml"),
            "[memory]\nhot_budget = 4000\n",
        );

        let project_only = resolve_key("project.kind", None, root);
        assert_eq!(project_only.resolved.as_deref(), Some("rust-service"));
        assert_eq!(project_only.source, "project_config");

        let overridden = resolve_key("memory.hot_budget", Some("2000"), root);
        assert_eq!(overridden.resolved.as_deref(), Some("4000"));
        assert_eq!(overridden.source, "scoped_config");

        let inherited = resolve_key("memory.context_budget", Some("16000"), root);
        assert_eq!(inherited.resolved.as_deref(), Some("8000"));
        assert_eq!(inherited.source, "project_config");

        let defaulted = resolve_key("policy.profile", Some("team-default"), root);
        assert_eq!(defaulted.resolved.as_deref(), Some("team-default"));
        assert_eq!(defaulted.source, "default");
    }

    #[test]
    fn unknown_key_fails_explicitly() {
        let dir = tempfile::tempdir().unwrap();
        let out = render_config_key_report(OutputFormat::Json, dir.path(), "does.not.exist");
        assert_eq!(out.status, 1);
        assert!(out.stderr.contains("unknown configuration key"));
    }

    #[test]
    fn json_output_is_valid_json() {
        let env = CliEnvironment::default();
        let out = render_config_report(&env, OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert_eq!(parsed["precedence"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn text_output_includes_all_seven_env_keys() {
        let env = CliEnvironment::default();
        let out = render_config_report(&env, OutputFormat::Text);
        for name in [
            "HOME",
            "XDG_DATA_HOME",
            "SDDK_DATA_DIR",
            "XDG_STATE_HOME",
            "XDG_CACHE_HOME",
            "SDDK_ACTOR",
            "USER",
        ] {
            assert!(out.stdout.contains(name), "missing key {name}");
        }
    }

    #[test]
    fn env_layer_beats_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("sddk.toml"), "[policy]\nprofile = \"file\"\n");
        // Injected env lookup: SDDK_POLICY_PROFILE wins over sddk.toml.
        let lookup = |k: &str| -> Option<String> {
            if k == "SDDK_POLICY_PROFILE" {
                Some("from-env".to_string())
            } else {
                None
            }
        };
        let report = resolve_key_with_env("policy.profile", Some("team-default"), &lookup, root);
        assert_eq!(report.resolved.as_deref(), Some("from-env"));
        assert_eq!(report.source, "env");
    }
}

#[cfg(test)]
mod orchestration_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn profdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sddk-cli-cfg-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("default.yaml"),
            "profile: default\nversion: 1\nautonomy.auto_advance: false\n",
        )
        .unwrap();
        fs::write(
            dir.join("bender.yaml"),
            "profile: bender\nversion: 1\nextends: default\nautonomy.auto_advance: true\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn report_resolves_mode_profile_and_sources() {
        let dir = profdir("report");
        let report = orchestration_report(Some("p-1 on bender\n"), &dir, "p-1", "w-1").unwrap();
        assert_eq!(report.mode, "on");
        assert_eq!(report.reason, "declared:project");
        assert_eq!(report.profile, "bender");
        assert_eq!(report.identity.project, "p-1");
        let adv = report
            .keys
            .iter()
            .find(|k| k.name == "autonomy.auto_advance")
            .unwrap();
        assert_eq!(adv.value, "true");
        assert_eq!(adv.source, "profile:bender");
    }

    #[test]
    fn report_laws_are_always_system_law() {
        let dir = profdir("laws");
        let report = orchestration_report(Some("p-1 on bender\n"), &dir, "p-1", "w-1").unwrap();
        for key in ["git.push", "git.tag", "git.release", "git.history_rewrite"] {
            let row = report.keys.iter().find(|k| k.name == key).unwrap();
            assert_eq!(row.source, "system-law");
            assert_eq!(row.value, "human_gate");
        }
    }

    #[test]
    fn report_undeclared_when_index_absent() {
        let dir = profdir("absent");
        let report = orchestration_report(None, &dir, "p-1", "w-1").unwrap();
        assert_eq!(report.mode, "undeclared");
        assert_eq!(report.reason, "index-absent");
    }

    #[test]
    fn report_rejects_law_override_in_profile() {
        let dir = profdir("evillaw");
        fs::write(
            dir.join("evil.yaml"),
            "profile: evil\nversion: 1\ngit.push: allowed\n",
        )
        .unwrap();
        let err = orchestration_report(Some("p-1 on evil\n"), &dir, "p-1", "w-1").unwrap_err();
        assert!(err.contains("fail-closed"), "got: {err}");
    }

    #[test]
    fn report_json_is_parseable_and_has_sources() {
        let dir = profdir("json");
        let report = orchestration_report(Some("p-1 on bender\n"), &dir, "p-1", "w-1").unwrap();
        let text = serde_json::to_string(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["mode"], "on");
        assert!(value["keys"].as_array().unwrap().len() >= 30);
        assert!(value["keys"][0]["source"].is_string());
    }
}
