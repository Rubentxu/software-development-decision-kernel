//! Gate classification taxonomy and closed registry loading.
//!
//! Per [[REQ-Gate-Classification-Discriminator]]: gates are classified as
//! `Security`, `Process`, or `Mixed`. Wave-1 budget gates default to `Process`
//! and are recoverable.
//!
//! Per [[REQ-Recovery-Action-Contract]]: each classification carries a
//! `RecoveryAction` that determines what the system does when the gate fails.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Kind of gate — determines recovery policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateKind {
    Security,
    Process,
    Mixed,
}

impl GateKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            GateKind::Security => "security",
            GateKind::Process => "process",
            GateKind::Mixed => "mixed",
        }
    }
}

impl std::str::FromStr for GateKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "security" => Ok(GateKind::Security),
            "process" => Ok(GateKind::Process),
            "mixed" => Ok(GateKind::Mixed),
            _ => Err(format!("invalid gate kind: {s}")),
        }
    }
}

/// Recovery action taken when a gate fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    RecoverForward,
    FailClosed,
    Advisory,
}

impl RecoveryAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecoveryAction::RecoverForward => "recover_forward",
            RecoveryAction::FailClosed => "fail_closed",
            RecoveryAction::Advisory => "advisory",
        }
    }
}

impl std::str::FromStr for RecoveryAction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "recover_forward" => Ok(RecoveryAction::RecoverForward),
            "fail_closed" => Ok(RecoveryAction::FailClosed),
            "advisory" => Ok(RecoveryAction::Advisory),
            _ => Err(format!("invalid recovery action: {s}")),
        }
    }
}

/// Who can grant a waiver for a gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WaiverAuthority {
    Owner,
    Lead,
    Security,
}

/// RFC 9457-shaped structured hint for operator recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryHint {
    /// CLI command or action to recover forward.
    pub recovery_command: String,
    /// Human-readable hint explaining the recovery path.
    pub hint: String,
}

/// One gate's classification entry from the closed registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateClassification {
    /// Gate kind (Security | Process | Mixed).
    #[serde(rename = "class")]
    pub class: GateKind,
    /// Whether this gate supports recovery-forward on failure.
    pub recoverable: bool,
    /// Recovery action when the gate fails and is recoverable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<RecoveryAction>,
    /// Structured hint for recovery (RFC 9457 shape).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_hint: Option<RecoveryHint>,
    /// Who may issue a waiver for this gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiver_authority: Option<WaiverAuthority>,
    /// Days a waiver is valid (≤ 30 per REQ-Process-Gate-Recoverable-Default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiver_expiry_days: Option<u32>,
}

/// Errors loading the gate classification registry.
#[derive(Debug, Error)]
pub enum GateClassificationError {
    #[error("registry not found: {0}")]
    RegistryNotFound(String),
    #[error("registry parse error: {0}")]
    Parse(String),
    #[error("invalid gate kind '{gate}' in registry: {kind}")]
    InvalidGateKind { gate: String, kind: String },
    #[error("invalid recovery action '{action}' for gate {gate}: {action_str}")]
    InvalidRecoveryAction {
        gate: String,
        action: String,
        action_str: String,
    },
}

/// Load the closed gate classification registry from a TOML file.
///
/// The file must be a TOML table mapping gate names to `GateClassification`
/// entries. Wave-1 gates default to `class = "process"` and `recoverable = true`.
pub fn load_classifications(
    registry_path: impl AsRef<Path>,
) -> Result<BTreeMap<String, GateClassification>, GateClassificationError> {
    let path = registry_path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|_| GateClassificationError::RegistryNotFound(path.display().to_string()))?;
    let raw: BTreeMap<String, toml::Value> =
        toml::from_str(&content).map_err(|e| GateClassificationError::Parse(e.to_string()))?;
    let mut classifications = BTreeMap::new();
    for (gate_name, value) in raw {
        let class: GateClassification =
            TryFrom::try_from(value.clone()).map_err(GateClassificationError::Parse)?;
        // Validate gate kind string if present as raw string
        if let toml::Value::Table(ref t) = value
            && let Some(class_str) = t.get("class").and_then(|v| v.as_str())
        {
            let _: GateKind =
                class_str
                    .parse()
                    .map_err(|_| GateClassificationError::InvalidGateKind {
                        gate: gate_name.clone(),
                        kind: class_str.to_string(),
                    })?;
        }
        classifications.insert(gate_name, class);
    }
    Ok(classifications)
}

impl TryFrom<toml::Value> for GateClassification {
    type Error = String;
    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        let table = value
            .as_table()
            .ok_or_else(|| "GateClassification must be a table".to_string())?;
        let class_str = table
            .get("class")
            .and_then(|v| v.as_str())
            .unwrap_or("process");
        let class: GateKind = class_str
            .parse()
            .map_err(|_| format!("invalid gate kind: {class_str}"))?;
        let recoverable = table
            .get("recoverable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let recovery_action = table
            .get("recovery_action")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<RecoveryAction>().ok());
        let recovery_hint = table
            .get("recovery_hint")
            .and_then(|v| v.as_table())
            .map(|t| RecoveryHint {
                recovery_command: t
                    .get("recovery_command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                hint: t
                    .get("hint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        let waiver_authority = table
            .get("waiver_authority")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "owner" => Some(WaiverAuthority::Owner),
                "lead" => Some(WaiverAuthority::Lead),
                "security" => Some(WaiverAuthority::Security),
                _ => None,
            });
        let waiver_expiry_days = table
            .get("waiver_expiry_days")
            .and_then(|v| v.as_integer())
            .map(|i| i as u32);
        Ok(GateClassification {
            class,
            recoverable,
            recovery_action,
            recovery_hint,
            waiver_authority,
            waiver_expiry_days,
        })
    }
}
