//! Capability execution lifecycle and receipts.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Lifecycle state of a capability execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Started,
    Succeeded,
    Failed,
    Unknown,
}

/// A capability receipt before its deterministic request hash is assigned.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityReceiptInput {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub capability: String,
    pub idempotency_key: String,
    pub request: Value,
    pub status: CapabilityStatus,
    pub result: Option<Value>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub agent_version_hash: Option<String>,
    pub behavior_version_hash: Option<String>,
}

/// A persisted capability execution receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityReceipt {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub capability: String,
    pub request_hash: String,
    pub request: Value,
    pub status: CapabilityStatus,
    pub result: Option<Value>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub agent_version_hash: Option<String>,
    pub behavior_version_hash: Option<String>,
}

/// Outcome of an idempotent capability receipt write.
#[derive(Debug, Clone, PartialEq)]
pub enum IdempotencyOutcome {
    Inserted(CapabilityReceipt),
    Replayed(CapabilityReceipt),
}
