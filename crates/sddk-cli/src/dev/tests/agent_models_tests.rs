//! Unit tests for agent-models schema, validation, and resolution (U1–U13).

use super::{
    AgentModelsConfig, IdeKey, ModelResolution, ModelTier, ModelsError, VoiceResolveError,
};

fn sample_yaml() -> &'static str {
    r#"
tiers:
  premium:
    opencode: deepseek/deepseek-chat
    zcode: deepseek/deepseek-chat
    claude: sonnet
    codex: openai/gpt-5.4
  fast:
    opencode: zai-coding-plan/glm-5-turbo
    zcode: zai-coding-plan/glm-5-turbo
    claude: haiku
    codex: openai/gpt-5.4-fast
agents:
  orchestrator:
    tier: premium
  sddk-explore:
    tier: fast
    overrides:
      opencode: deepseek/deepseek-reasoner
      codex: openai/gpt-5.3-codex-spark
"#
}

// U1 — valid config loads
#[test]
fn loads_valid_config() {
    let config = AgentModelsConfig::from_yaml(sample_yaml()).unwrap();
    assert_eq!(config.agents().len(), 2);
    assert_eq!(config.tier_of("orchestrator"), Some(ModelTier::Premium));
    assert_eq!(config.tier_of("sddk-explore"), Some(ModelTier::Fast));
    assert_eq!(config.tiers().len(), 2);
}

// U2 — unknown override IDE key is ignored
#[test]
fn ignores_unknown_override_key() {
    let yaml = sample_yaml().replace(
        "overrides:\n      opencode:",
        "overrides:\n      cursor: cursor-model\n      opencode:",
    );
    let config = AgentModelsConfig::from_yaml(&yaml).unwrap();
    let overrides = config.overrides_of("sddk-explore").unwrap();
    assert!(!overrides.keys().any(|ide| ide.as_str() == "cursor"));
    assert_eq!(
        overrides.get(&IdeKey::Opencode).map(String::as_str),
        Some("deepseek/deepseek-reasoner")
    );
}

// U3 — empty file loads as empty config
#[test]
fn empty_file_loads_empty_config() {
    let config = AgentModelsConfig::from_yaml("").unwrap();
    assert_eq!(config.agents().len(), 0);
    assert_eq!(config.tiers().len(), 0);
    assert!(matches!(
        config.resolve("orchestrator", IdeKey::Opencode),
        ModelResolution::NoModelConfigured { .. }
    ));
}

// U4 — unknown tier names the agent and the offending value
#[test]
fn unknown_tier_names_agent_and_value() {
    let yaml = "agents:\n  sddk-explore:\n    tier: turbo\n";
    let error = AgentModelsConfig::from_yaml(yaml).unwrap_err();
    assert!(matches!(
        error,
        ModelsError::UnknownTier { agent, tier } if agent == "sddk-explore" && tier == "turbo"
    ));
}

// U5 — empty model ID rejected with agent and field named
#[test]
fn empty_override_rejected() {
    let yaml = "agents:\n  sddk-explore:\n    tier: fast\n    overrides:\n      opencode: \"\"\n";
    let error = AgentModelsConfig::from_yaml(yaml).unwrap_err();
    assert!(matches!(
        error,
        ModelsError::EmptyModelId { agent, ide } if agent == "sddk-explore" && ide == "opencode"
    ));
}

// U6 — duplicate agent rejected at parse level
#[test]
fn duplicate_agent_rejected() {
    let yaml = "agents:\n  sddk-explore:\n    tier: fast\n  sddk-explore:\n    tier: premium\n";
    let error = AgentModelsConfig::from_yaml(yaml).unwrap_err();
    assert!(matches!(error, ModelsError::Parse(_)), "{error}");
}

// U7 — override wins over tier table
#[test]
fn override_wins_over_tier_table() {
    let config = AgentModelsConfig::from_yaml(sample_yaml()).unwrap();
    assert_eq!(
        config.resolve("sddk-explore", IdeKey::Opencode),
        ModelResolution::Model("deepseek/deepseek-reasoner".to_owned())
    );
}

// U8 — tier default used for claude
#[test]
fn tier_default_used_for_claude() {
    let config = AgentModelsConfig::from_yaml(sample_yaml()).unwrap();
    assert_eq!(
        config.resolve("orchestrator", IdeKey::Claude),
        ModelResolution::Model("sonnet".to_owned())
    );
}

// U9 — unknown agent is NoModelConfigured
#[test]
fn unknown_agent_is_no_model() {
    let config = AgentModelsConfig::from_yaml(sample_yaml()).unwrap();
    assert!(matches!(
        config.resolve("not-an-agent", IdeKey::Opencode),
        ModelResolution::NoModelConfigured { agent, ide }
            if agent == "not-an-agent" && ide == IdeKey::Opencode
    ));
}

// U10 — unknown IDE errors (no cross-IDE guessing, no cross-tier fallback)
#[test]
fn unknown_ide_is_no_model() {
    let yaml = "tiers:\n  premium:\n    opencode: deepseek/deepseek-chat\nagents:\n  orchestrator:\n    tier: premium\n";
    let config = AgentModelsConfig::from_yaml(yaml).unwrap();
    assert!(matches!(
        config.resolve("orchestrator", IdeKey::Codex),
        ModelResolution::NoModelConfigured {
            ide: IdeKey::Codex,
            ..
        }
    ));
    // Fast agent must NOT fall back to the premium table.
    let yaml = "tiers:\n  premium:\n    opencode: deepseek/deepseek-chat\nagents:\n  sddk-explore:\n    tier: fast\n";
    let config = AgentModelsConfig::from_yaml(yaml).unwrap();
    assert!(matches!(
        config.resolve("sddk-explore", IdeKey::Opencode),
        ModelResolution::NoModelConfigured { .. }
    ));
}

// U11 — round-trip stable serialization
#[test]
fn round_trip_serialize_stable() {
    let config = AgentModelsConfig::from_yaml(sample_yaml()).unwrap();
    let yaml = config.to_yaml().unwrap();
    let reparsed = AgentModelsConfig::from_yaml(&yaml).unwrap();
    assert_eq!(reparsed, config);
}

// U12 — missing tier field errors naming the agent
#[test]
fn missing_tier_field_errors() {
    let yaml = "agents:\n  sddk-explore:\n    overrides:\n      opencode: x\n";
    let error = AgentModelsConfig::from_yaml(yaml).unwrap_err();
    assert!(matches!(error, ModelsError::Parse(message) if message.contains("sddk-explore")));
}

// U13 — absent file loads as None
#[test]
fn from_file_missing_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.yaml");
    assert_eq!(AgentModelsConfig::from_file(&missing).unwrap(), None);
}

// BundleCoverage — the shipped canonical file parses and covers every
// bundle agent in the workspace (missing agent = silent skip on link).
#[test]
fn asset_parses_and_covers_all_bundle_agents() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let asset = workspace.join("assets/agent-models.yaml");
    assert!(
        asset.is_file(),
        "canonical asset missing: {}",
        asset.display()
    );
    let config = AgentModelsConfig::from_file(&asset)
        .expect("assets/agent-models.yaml must parse")
        .expect("assets/agent-models.yaml must exist");
    let agents_dir = workspace.join("agents");
    let mut stems: Vec<String> = std::fs::read_dir(&agents_dir)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("md"))
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .collect();
    stems.sort();
    let mut configured: Vec<String> = config.agents().keys().cloned().collect();
    configured.sort();
    assert_eq!(
        configured, stems,
        "agent-models.yaml must declare exactly one entry per bundle agent"
    );
}

// ── Voice profile tests (ADR-0129) ───────────────────────────────────────────

fn voice_yaml() -> &'static str {
    r#"
tiers:
  premium:
    opencode: deepseek/deepseek-chat
    zcode: deepseek/deepseek-chat
  fast:
    opencode: zai-coding-plan/glm-5-turbo
agents:
  orchestrator:
    tier: premium

voice_profiles:
  bender_friendly:
    label: "Bender (sarcástico, simpático)"
    mood: "warm sarcasm"
    tier_premium: "Bender work-safe. Cite file:line."
    tier_fast:    "Bender work-safe short. Cite file."
  concise:
    label: "Concise"
    tier_premium: "Reply terse."
    tier_fast:    "Reply terse."
  socratic:
    label: "Socratic"
    tier_premium: "Ask back."
    tier_fast:    "Ask back."

defaults:
  voice: bender_friendly
  tier: premium
"#
}

#[test]
fn voice_profiles_load_from_yaml() {
    let cfg = AgentModelsConfig::from_yaml(voice_yaml()).expect("parse");
    let profiles = cfg.voice_profiles().expect("voice set");
    assert!(profiles.profiles.contains_key("bender_friendly"));
    assert!(profiles.profiles.contains_key("concise"));
    assert!(profiles.profiles.contains_key("socratic"));
    assert_eq!(profiles.default_voice, "bender_friendly");
    assert_eq!(profiles.default_tier, ModelTier::Premium);
}

#[test]
fn voice_profile_resolution_by_tier() {
    let cfg = AgentModelsConfig::from_yaml(voice_yaml()).expect("parse");
    let premium = cfg
        .resolve_voice_prompt("bender_friendly", ModelTier::Premium)
        .unwrap();
    let fast = cfg
        .resolve_voice_prompt("bender_friendly", ModelTier::Fast)
        .unwrap();
    assert!(premium.contains("file:line"));
    assert!(fast.contains("short"));
}

#[test]
fn wisecracking_robot_aliases_to_bender_friendly() {
    let cfg = AgentModelsConfig::from_yaml(voice_yaml()).expect("parse");
    let via_alias = cfg
        .resolve_voice_prompt("wisecracking_robot", ModelTier::Premium)
        .unwrap();
    let direct = cfg
        .resolve_voice_prompt("bender_friendly", ModelTier::Premium)
        .unwrap();
    assert_eq!(via_alias, direct, "alias must return the same prompt");
}

#[test]
fn unknown_voice_profile_errors() {
    let cfg = AgentModelsConfig::from_yaml(voice_yaml()).expect("parse");
    let r = cfg.resolve_voice_prompt("does_not_exist", ModelTier::Premium);
    assert!(matches!(r, Err(VoiceResolveError::UnknownProfile { .. })));
}

#[test]
fn missing_voice_profiles_block_returns_none() {
    let yaml = r#"
tiers:
  premium:
    opencode: foo
agents:
  orchestrator:
    tier: premium
"#;
    let cfg = AgentModelsConfig::from_yaml(yaml).expect("parse");
    assert!(cfg.voice_profiles().is_none());
    assert_eq!(cfg.default_voice_key(), "bender_friendly");
}

#[test]
fn voice_profile_keys_sorted_deterministically() {
    let cfg = AgentModelsConfig::from_yaml(voice_yaml()).expect("parse");
    let keys = cfg.voice_profile_keys();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "BTreeMap iteration must be alphabetical");
}
