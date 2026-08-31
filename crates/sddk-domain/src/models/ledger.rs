//! Ledger event records and verification.
use serde::Serialize;
use serde_json::Value;

/// Data required to append one ledger event.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerEventInput {
    pub event_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub frame_id: String,
    pub command_id: String,
    pub actor: String,
    pub event_type: String,
    pub occurred_at: String,
    pub state_before: Option<Value>,
    pub state_after: Option<Value>,
    pub payload: Value,
}

/// An immutable hash-linked ledger event.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LedgerEvent {
    pub sequence: i64,
    pub event_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub frame_id: String,
    pub command_id: String,
    pub actor: String,
    pub event_type: String,
    pub occurred_at: String,
    pub state_before: Option<Value>,
    pub state_after: Option<Value>,
    pub payload: Value,
    pub previous_hash: Option<String>,
    pub event_hash: String,
}

impl LedgerEvent {
    pub fn as_input(&self) -> LedgerEventInput {
        LedgerEventInput {
            event_id: self.event_id.clone(),
            project_id: self.project_id.clone(),
            cycle_id: self.cycle_id.clone(),
            frame_id: self.frame_id.clone(),
            command_id: self.command_id.clone(),
            actor: self.actor.clone(),
            event_type: self.event_type.clone(),
            occurred_at: self.occurred_at.clone(),
            state_before: self.state_before.clone(),
            state_after: self.state_after.clone(),
            payload: self.payload.clone(),
        }
    }
}

/// Metadata for an artifact stored outside SQLite.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub kind: String,
    pub path: String,
    pub sha256: Option<String>,
    pub producer: Option<String>,
    pub created_at: String,
    pub metadata: Value,
}

/// Result of verifying the complete ledger chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerVerification {
    pub event_count: usize,
    pub last_hash: Option<String>,
}
