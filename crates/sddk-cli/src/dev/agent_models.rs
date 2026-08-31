//! Agent→model mapping: `agent-models.yaml` schema, validation, and resolution.
//! Single source of model IDs per IDE (ADR-0017) — replaces the hardcoded
//! fallback removed from `framework_check.rs`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Model quality tier of an agent.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Premium,
    Fast,
}

impl ModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Premium => "premium",
            Self::Fast => "fast",
        }
    }
}

impl FromStr for ModelTier {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "premium" => Ok(Self::Premium),
            "fast" => Ok(Self::Fast),
            other => Err(format!("unknown tier `{other}` (expected premium|fast)")),
        }
    }
}

/// Editor targets understood by the schema.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum IdeKey {
    Opencode,
    Zcode,
    Claude,
    Codex,
}

impl IdeKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Opencode => "opencode",
            Self::Zcode => "zcode",
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// Parse an IDE key; unknown keys are ignored by the schema (V4).
    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "opencode" => Some(Self::Opencode),
            "zcode" => Some(Self::Zcode),
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }
}

/// Validated agent→model configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AgentModelsConfig {
    tiers: BTreeMap<ModelTier, BTreeMap<IdeKey, String>>,
    agents: BTreeMap<String, AgentModelEntry>,
}

/// Per-agent model configuration: tier + optional per-IDE overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentModelEntry {
    tier: ModelTier,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    overrides: BTreeMap<IdeKey, String>,
}

/// Result of resolving a model for (agent, ide).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResolution {
    Model(String),
    NoModelConfigured { agent: String, ide: IdeKey },
}

/// Load/validation errors for agent-models.yaml.
#[derive(Debug, thiserror::Error)]
pub enum ModelsError {
    #[error("invalid agent-models.yaml: {0}")]
    Parse(String),
    #[error("agent `{agent}`: unknown tier `{tier}` (expected premium|fast)")]
    UnknownTier { agent: String, tier: String },
    #[error("agent `{agent}`: empty model id in overrides.{ide}")]
    EmptyModelId { agent: String, ide: String },
    #[error("agent `{agent}`: no entry configured; set its tier first with --tier")]
    UnknownAgent { agent: String },
    #[error("failed to read {path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

// ── Phase 1: raw tolerant shapes (agent + field names in errors) ──────────────

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    tiers: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    agents: HashMap<String, RawAgent>,
}

#[derive(Debug, Deserialize)]
struct RawAgent {
    tier: Option<String>,
    #[serde(default)]
    overrides: HashMap<String, String>,
}

impl AgentModelsConfig {
    /// Two-phase load: tolerant raw parse (phase 1) → validated typed config
    /// (phase 2). Empty documents load as an empty config.
    pub fn from_yaml(yaml: &str) -> Result<Self, ModelsError> {
        if yaml.trim().is_empty() {
            return Ok(Self::default());
        }
        let raw: RawConfig =
            serde_saphyr::from_str(yaml).map_err(|error| ModelsError::Parse(error.to_string()))?;
        let mut tiers: BTreeMap<ModelTier, BTreeMap<IdeKey, String>> = BTreeMap::new();
        for (tier_name, table) in raw.tiers {
            let tier = ModelTier::from_str(&tier_name).map_err(ModelsError::Parse)?;
            let mut mapped = BTreeMap::new();
            for (ide_name, model) in table {
                let Some(ide) = IdeKey::parse(&ide_name) else {
                    continue;
                };
                if model.trim().is_empty() {
                    return Err(ModelsError::Parse(format!(
                        "tiers.{tier_name}.{ide_name}: empty model id"
                    )));
                }
                mapped.insert(ide, model);
            }
            tiers.insert(tier, mapped);
        }
        let mut agents: BTreeMap<String, AgentModelEntry> = BTreeMap::new();
        for (name, raw_agent) in raw.agents {
            let Some(tier_name) = raw_agent.tier else {
                return Err(ModelsError::Parse(format!(
                    "agent `{name}`: missing required field `tier`"
                )));
            };
            let tier = ModelTier::from_str(&tier_name).map_err(|_| ModelsError::UnknownTier {
                agent: name.clone(),
                tier: tier_name,
            })?;
            let mut overrides = BTreeMap::new();
            for (ide_name, model) in raw_agent.overrides {
                let Some(ide) = IdeKey::parse(&ide_name) else {
                    continue;
                };
                if model.trim().is_empty() {
                    return Err(ModelsError::EmptyModelId {
                        agent: name.clone(),
                        ide: ide_name,
                    });
                }
                overrides.insert(ide, model);
            }
            agents.insert(name, AgentModelEntry { tier, overrides });
        }
        Ok(Self { tiers, agents })
    }

    /// Load from a file; absence is not an error (`Ok(None)`).
    pub fn from_file(path: &Path) -> Result<Option<Self>, ModelsError> {
        match std::fs::read_to_string(path) {
            Ok(yaml) => Self::from_yaml(&yaml).map(Some),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ModelsError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Serialize to canonical YAML (deterministic BTreeMap ordering).
    pub fn to_yaml(&self) -> Result<String, ModelsError> {
        serde_saphyr::to_string(self).map_err(|error| ModelsError::Parse(error.to_string()))
    }

    /// Resolution precedence (ADR-0017): per-IDE override → per-tier default
    /// table → `NoModelConfigured`. No cross-IDE or cross-tier fallback.
    pub fn resolve(&self, agent: &str, ide: IdeKey) -> ModelResolution {
        let Some(entry) = self.agents.get(agent) else {
            return ModelResolution::NoModelConfigured {
                agent: agent.to_owned(),
                ide,
            };
        };
        if let Some(model) = entry.overrides.get(&ide) {
            return ModelResolution::Model(model.clone());
        }
        if let Some(model) = self
            .tiers
            .get(&entry.tier)
            .and_then(|table| table.get(&ide))
        {
            return ModelResolution::Model(model.clone());
        }
        ModelResolution::NoModelConfigured {
            agent: agent.to_owned(),
            ide,
        }
    }

    pub fn tiers(&self) -> &BTreeMap<ModelTier, BTreeMap<IdeKey, String>> {
        &self.tiers
    }

    pub fn agents(&self) -> &BTreeMap<String, AgentModelEntry> {
        &self.agents
    }

    pub fn tier_of(&self, agent: &str) -> Option<ModelTier> {
        self.agents.get(agent).map(|entry| entry.tier)
    }

    pub fn overrides_of(&self, agent: &str) -> Option<&BTreeMap<IdeKey, String>> {
        self.agents.get(agent).map(|entry| &entry.overrides)
    }

    /// Set (or update) an agent's tier, creating the entry when absent.
    pub fn set_tier(&mut self, agent: &str, tier: ModelTier) {
        self.agents
            .entry(agent.to_owned())
            .and_modify(|entry| entry.tier = tier)
            .or_insert(AgentModelEntry {
                tier,
                overrides: BTreeMap::new(),
            });
    }

    /// Set a per-IDE override on an existing entry.
    pub fn set_override(
        &mut self,
        agent: &str,
        ide: IdeKey,
        model: String,
    ) -> Result<(), ModelsError> {
        if model.trim().is_empty() {
            return Err(ModelsError::EmptyModelId {
                agent: agent.to_owned(),
                ide: ide.as_str().to_owned(),
            });
        }
        let entry = self
            .agents
            .get_mut(agent)
            .ok_or_else(|| ModelsError::UnknownAgent {
                agent: agent.to_owned(),
            })?;
        entry.overrides.insert(ide, model);
        Ok(())
    }

    /// Remove a per-IDE override. Returns false when nothing was removed.
    pub fn clear_override(&mut self, agent: &str, ide: IdeKey) -> bool {
        self.agents
            .get_mut(agent)
            .map(|entry| entry.overrides.remove(&ide).is_some())
            .unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "tests/agent_models_tests.rs"]
mod agent_models_tests;
