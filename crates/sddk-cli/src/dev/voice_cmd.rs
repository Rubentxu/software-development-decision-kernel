//! `dev voice` — manage voice profile resolution per agent.
//!
//! Reads `assets/agent-models.yaml` from the active bundle. Per-agent
//! overrides live at `~/.local/share/sddk/voice/<agent>.yaml` and are
//! merged on top of bundle defaults.

use crate::dev::agent_models::{
    AgentModelsConfig, DEFAULT_VOICE_PROFILE_KEY, ModelTier, VoiceProfile, WISECRACKING_ROBOT_ALIAS,
};
use crate::dev::paths::{resolve_active_framework_root, sddk_data_dir};
use crate::{CliEnvironment, CommandOutput, OutputFormat};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Clone, Args)]
pub(crate) struct VoiceArgs {
    #[command(subcommand)]
    pub(super) command: VoiceCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum VoiceCommand {
    /// List all configured voice profiles plus the canonical default.
    List(VoiceListArgs),
    /// Show the resolved voice + tier for one agent (profile, mood, prompt).
    Show(VoiceShowArgs),
    /// Set the voice profile (and optionally tier) for one agent.
    Set(VoiceSetArgs),
    /// Print the path of the per-agent voice override directory.
    Dir,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VoiceListArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VoiceShowArgs {
    /// Agent name (e.g. "orchestrator", "sddk-verify").
    #[arg(long)]
    pub(super) agent: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VoiceSetArgs {
    /// Agent name (e.g. "orchestrator").
    #[arg(long)]
    pub(super) agent: String,
    /// Profile key (e.g. "bender_friendly", "concise", "explanatory").
    /// Use `default` to reset to the canonical default.
    #[arg(long)]
    pub(super) profile: String,
    /// Optional tier override (premium|fast). If absent, tier is read
    /// from the agent's current entry, falling back to the default tier.
    #[arg(long, value_enum)]
    pub(super) tier: Option<ModelTier>,
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn failure(status: i32, message: impl std::fmt::Display) -> CommandOutput {
    CommandOutput {
        status,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}

fn success(text: String) -> CommandOutput {
    CommandOutput {
        status: 0,
        stdout: text,
        stderr: String::new(),
    }
}

// ── Resolution ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(super) struct ResolvedVoice {
    pub source: VoiceSource,
    pub profile_key: String,
    pub tier: ModelTier,
    pub mood: Option<String>,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VoiceSource {
    Override,
    Bundle,
    Fallback,
}

impl VoiceSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Override => "override",
            Self::Bundle => "bundle",
            Self::Fallback => "fallback",
        }
    }
}

fn override_path(env: &CliEnvironment, agent: &str) -> Option<PathBuf> {
    sddk_data_dir(env)
        .ok()
        .map(|d| d.join("voice").join(format!("{agent}.yaml")))
}

fn load_bundle() -> AgentModelsConfig {
    let env = CliEnvironment::default();
    let Ok(root) = resolve_active_framework_root(&env) else {
        return AgentModelsConfig::default();
    };
    let path = root.join("assets").join("agent-models.yaml");
    AgentModelsConfig::from_file(&path)
        .ok()
        .flatten()
        .unwrap_or_default()
}

// ── Subcommand runners ───────────────────────────────────────────────────

pub(super) fn run_voice_list(args: VoiceListArgs, env: &CliEnvironment) -> CommandOutput {
    let cfg = load_bundle();
    let keys = cfg.voice_profile_keys();
    let default_key = cfg.default_voice_key();
    let text = match args.format {
        OutputFormat::Text => {
            let mut out = String::from("voice profiles:\n");
            for k in &keys {
                let marker = if *k == default_key { "* " } else { "  " };
                let label = cfg
                    .voice_profiles()
                    .and_then(|s| s.profiles.get(k))
                    .map(|p| p.label.as_str())
                    .unwrap_or("");
                let _ =
                    std::fmt::Write::write_fmt(&mut out, format_args!("{marker}{k:<22} {label}\n"));
            }
            out.push_str(&format!("\ndefault: {default_key}\n"));
            out
        }
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = keys
                .iter()
                .map(|k| {
                    serde_json::json!({
                        "key": k,
                        "is_default": *k == default_key,
                        "label": cfg.voice_profiles().and_then(|s| s.profiles.get(k)).map(|p| p.label.clone()).unwrap_or_default(),
                    })
                })
                .collect();
            serde_json::to_string_pretty(&serde_json::json!({
                "default": default_key,
                "profiles": items,
            }))
            .unwrap_or_else(|_| "{}".to_string())
        }
    };
    let _ = env;
    success(text)
}

pub(super) fn run_voice_show(args: VoiceShowArgs, env: &CliEnvironment) -> CommandOutput {
    let cfg = load_bundle();
    let resolved = resolve_voice(&args.agent, &cfg, env);
    let text = match args.format {
        OutputFormat::Text => {
            let ResolvedVoice {
                source,
                profile_key,
                tier,
                mood,
                system_prompt,
            } = &resolved;
            let mut out = format!("agent: {}\n", args.agent);
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "voice: {profile_key}\ntier:  {}\nsource: {}\n",
                    tier_to_str(*tier),
                    source.label()
                ),
            );
            if let Some(m) = mood {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("mood:  {m}\n"));
            }
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!("\nsystem_prompt:\n{system_prompt}\n"),
            );
            out
        }
        OutputFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "agent": args.agent,
            "voice": resolved.profile_key,
            "tier": tier_to_str(resolved.tier),
            "source": resolved.source.label(),
            "mood": resolved.mood,
            "system_prompt": resolved.system_prompt,
        }))
        .unwrap_or_else(|_| "{}".to_string()),
    };
    success(text)
}

pub(super) fn run_voice_set(args: VoiceSetArgs, env: &CliEnvironment) -> CommandOutput {
    let Some(path) = override_path(env, &args.agent) else {
        return failure(2, "could not resolve data dir");
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let cfg = load_bundle();
    let profile_key = if args.profile == "default" {
        cfg.default_voice_key().to_string()
    } else {
        args.profile.clone()
    };

    // Validate that the profile exists (unless alias or default sentinel).
    if profile_key != DEFAULT_VOICE_PROFILE_KEY
        && profile_key != WISECRACKING_ROBOT_ALIAS
        && !cfg.voice_profile_keys().contains(&profile_key)
    {
        return failure(
            2,
            format!(
                "unknown voice profile `{profile_key}`; available: {}",
                cfg.voice_profile_keys().join(", ")
            ),
        );
    }

    let tier = args.tier.unwrap_or_else(|| {
        cfg.tier_of(&args.agent).unwrap_or_else(|| {
            cfg.voice_profiles()
                .map(|v| v.default_tier)
                .unwrap_or_default()
        })
    });

    let prompt = match cfg.resolve_voice_prompt(&profile_key, tier) {
        Ok(p) => p,
        Err(e) => return failure(2, format!("{e}")),
    };

    let yaml = format!(
        "# per-agent voice override (ADR-0129)\n# managed by `sddk dev voice set`\nprofile: {profile_key}\ntier: {tier_str}\nprompt: |\n  {lines}\n",
        lines = prompt.lines().collect::<Vec<_>>().join("\n  "),
        tier_str = tier_to_str(tier),
    );
    if let Err(e) = std::fs::write(&path, &yaml) {
        return failure(1, format!("failed to write {path:?}: {e}"));
    }

    success(format!(
        "voice set for {agent}\n  profile: {profile_key}\n  tier: {tier}\n  path: {path}\n",
        agent = args.agent,
        tier = tier_to_str(tier),
        path = path.display(),
    ))
}

pub(super) fn run_voice_dir(env: &CliEnvironment) -> CommandOutput {
    match sddk_data_dir(env) {
        Ok(d) => success(format!("{}\n", d.join("voice").display())),
        Err(e) => failure(2, format!("{e}")),
    }
}

pub(super) fn run_voice(args: VoiceArgs, env: &CliEnvironment) -> CommandOutput {
    match args.command {
        VoiceCommand::List(a) => run_voice_list(a, env),
        VoiceCommand::Show(a) => run_voice_show(a, env),
        VoiceCommand::Set(a) => run_voice_set(a, env),
        VoiceCommand::Dir => run_voice_dir(env),
    }
}

fn tier_to_str(t: ModelTier) -> &'static str {
    match t {
        ModelTier::Premium => "premium",
        ModelTier::Fast => "fast",
    }
}

// ── Resolution helper ────────────────────────────────────────────────────

#[allow(dead_code)]
pub(super) fn resolve_voice(
    agent: &str,
    bundle_cfg: &AgentModelsConfig,
    env: &CliEnvironment,
) -> ResolvedVoice {
    if let Some(p) = override_path(env, agent) {
        let yaml_result = std::fs::read_to_string(&p);
        let parsed_result = yaml_result
            .as_ref()
            .ok()
            .and_then(|s| serde_saphyr::from_str::<OverrideYaml>(s).ok());
        if let Some(parsed) = parsed_result {
            let tier = parsed.tier.unwrap_or_else(|| {
                bundle_cfg.tier_of(agent).unwrap_or_else(|| {
                    bundle_cfg
                        .voice_profiles()
                        .map(|v| v.default_tier)
                        .unwrap_or_default()
                })
            });
            if let Some(profile) = bundle_cfg
                .voice_profiles()
                .and_then(|v| v.profiles.get(&parsed.profile))
            {
                let prompt = match tier {
                    ModelTier::Premium => profile.tier_premium.clone(),
                    ModelTier::Fast => profile.tier_fast.clone(),
                };
                return ResolvedVoice {
                    source: VoiceSource::Override,
                    profile_key: parsed.profile,
                    tier,
                    mood: profile.mood.clone(),
                    system_prompt: prompt,
                };
            }
            return ResolvedVoice {
                source: VoiceSource::Override,
                profile_key: parsed.profile,
                tier,
                mood: None,
                system_prompt: parsed.prompt.unwrap_or_default(),
            };
        }
    }

    let profile_key = bundle_cfg.default_voice_key().to_string();
    let tier = bundle_cfg.tier_of(agent).unwrap_or_else(|| {
        bundle_cfg
            .voice_profiles()
            .map(|v| v.default_tier)
            .unwrap_or_default()
    });
    if let Ok(prompt) = bundle_cfg.resolve_voice_prompt(&profile_key, tier) {
        let mood = bundle_cfg
            .voice_profiles()
            .and_then(|v| v.profiles.get(&profile_key))
            .and_then(|p| p.mood.clone());
        return ResolvedVoice {
            source: VoiceSource::Bundle,
            profile_key,
            tier,
            mood,
            system_prompt: prompt,
        };
    }

    ResolvedVoice {
        source: VoiceSource::Fallback,
        profile_key: DEFAULT_VOICE_PROFILE_KEY.to_string(),
        tier,
        mood: Some("generic".to_string()),
        system_prompt:
            "You are a helpful assistant. Reply clearly and cite file:line when relevant."
                .to_string(),
    }
}

#[derive(Debug, serde::Deserialize)]
struct OverrideYaml {
    profile: String,
    #[serde(default)]
    tier: Option<ModelTier>,
    #[serde(default)]
    prompt: Option<String>,
}

#[allow(dead_code)]
pub(super) fn make_voice_profile(
    key: String,
    label: String,
    tier_premium: String,
    tier_fast: String,
) -> VoiceProfile {
    VoiceProfile {
        key,
        label,
        mood: None,
        tier_premium,
        tier_fast,
    }
}
