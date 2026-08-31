//! Durable repair-receipt queue for scoped vault diagnostics.
//!
//! Provides monotonic append-only I/O for `RepairReceipt` entries that authorize
//! scoped down-classification of vault diagnostics (e.g. VAULT003 broken-link
//! diagnostics whose target cycle node has been subsequently created).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// The closed set of diagnostic codes eligible for scoped down-classification.
pub const ALLOW_LIST: &[&str] = &["VAULT003"];

/// Repair action that produced the target file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairAction {
    /// The target node was created as part of the repair.
    NodeCreation,
    /// The target file was rewritten as plain text.
    PlainTextRewrite,
}

/// A durable receipt recording one scoped repair operation.
///
/// The receipt binds the repair to:
/// - The specific cycle and diagnostic it resolves
/// - The SHA-256 of the repair artifact (evidence that the target now exists)
/// - A validity window (≤ 90 days from creation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RepairReceipt {
    /// Cycle that authorized this repair.
    pub cycle_id: String,
    /// Diagnostic code this receipt resolves (must be in `ALLOW_LIST`).
    pub code: String,
    /// Node id that contained the broken wikilink.
    pub node: String,
    /// Missing wikilink target this repair addresses.
    pub target: String,
    /// Action taken to repair the broken link.
    pub repair_action: RepairAction,
    /// SHA-256 hex of the repair artifact (the created/rewritten file).
    pub durable_evidence_sha: String,
    /// When this receipt was created (RFC3339).
    pub created_at: OffsetDateTime,
    /// Receipt validity upper bound (RFC3339, ≤ created_at + 90 days).
    pub valid_to: OffsetDateTime,
}

/// Errors that can occur when loading or appending to the repair queue.
#[derive(Debug, Error)]
pub enum RepairQueueError {
    /// A required field is missing from a queue entry.
    #[error("missing field `{0}` in repair queue entry")]
    MissingField(&'static str),

    /// The timestamp is not valid RFC3339.
    #[error("invalid RFC3339 timestamp: {0}")]
    InvalidRfc3339(String),

    /// The evidence hash does not match the repair artifact.
    #[error("receipt evidence hash mismatch (want {expected}, got {actual})")]
    ReceiptEvidenceHashMismatch {
        /// Expected hash from the receipt.
        expected: String,
        /// Actual hash computed from the artifact.
        actual: String,
    },

    /// The receipt has expired.
    #[error("repair receipt has expired (valid_to: {0})")]
    ReceiptExpired(String),

    /// The queue YAML file is malformed.
    #[error("repair queue YAML is malformed: {0}")]
    QueueMalformed(String),
}

/// Loads the monotonic append-only repair queue from a YAML file.
///
/// Returns a `HashMap` keyed by `{cycle_id}/{code}/{node}` for O(1) lookup
/// during scoped down-classification.
pub fn load_repair_queue(path: &Path) -> Result<HashMap<String, RepairReceipt>, RepairQueueError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| RepairQueueError::QueueMalformed(e.to_string()))?;

    let entries: Vec<RepairReceipt> = serde_yaml::from_str(&content)
        .map_err(|e| RepairQueueError::QueueMalformed(e.to_string()))?;

    let mut queue = HashMap::new();
    for entry in entries {
        let key = format!("{}/{}/{}", entry.cycle_id, entry.code, entry.node);
        queue.insert(key, entry);
    }

    Ok(queue)
}

/// Appends a new repair receipt to the monotonic queue.
///
/// Refuses to overwrite existing entries — the queue is append-only.
/// Returns an error if the entry already exists.
pub fn append_repair_receipt(path: &Path, receipt: &RepairReceipt) -> Result<(), RepairQueueError> {
    // Validate required fields
    if receipt.cycle_id.is_empty() {
        return Err(RepairQueueError::MissingField("cycle_id"));
    }
    if receipt.code.is_empty() {
        return Err(RepairQueueError::MissingField("code"));
    }
    if receipt.node.is_empty() {
        return Err(RepairQueueError::MissingField("node"));
    }

    // Validate 90-day bound
    let max_valid_to = receipt
        .created_at
        .checked_add(time::Duration::days(90))
        .ok_or_else(|| {
            RepairQueueError::InvalidRfc3339("created_at overflow when computing valid_to".into())
        })?;

    if receipt.valid_to > max_valid_to {
        return Err(RepairQueueError::InvalidRfc3339(format!(
            "valid_to exceeds 90-day window (max: {})",
            max_valid_to
        )));
    }

    // Load existing entries
    let existing: Vec<RepairReceipt> = if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RepairQueueError::QueueMalformed(e.to_string()))?;
        serde_yaml::from_str(&content)
            .map_err(|e| RepairQueueError::QueueMalformed(e.to_string()))?
    } else {
        Vec::new()
    };

    // Check for existing entry (append-only enforcement)
    let key = format!("{}/{}/{}", receipt.cycle_id, receipt.code, receipt.node);
    for entry in &existing {
        let entry_key = format!("{}/{}/{}", entry.cycle_id, entry.code, entry.node);
        if entry_key == key {
            // Entry exists — refuse to overwrite
            return Err(RepairQueueError::QueueMalformed(format!(
                "entry for {key} already exists (append-only)"
            )));
        }
    }

    // Append new entry
    let mut entries = existing;
    entries.push(receipt.clone());

    let yaml = serde_yaml::to_string(&entries)
        .map_err(|e| RepairQueueError::QueueMalformed(e.to_string()))?;

    // Atomic write: write to temp file then rename
    let temp_path = path.with_extension("yaml.tmp");
    std::fs::write(&temp_path, &yaml)
        .map_err(|e| RepairQueueError::QueueMalformed(e.to_string()))?;
    std::fs::rename(&temp_path, path)
        .map_err(|e| RepairQueueError::QueueMalformed(e.to_string()))?;

    Ok(())
}

/// Checks whether a receipt's evidence hash matches the expected artifact hash.
pub fn verify_receipt_evidence(
    receipt: &RepairReceipt,
    artifact_path: &Path,
) -> Result<(), RepairReceiptError> {
    use sha2::{Digest, Sha256};

    let artifact_bytes =
        std::fs::read(artifact_path).map_err(|_| RepairReceiptError::ArtifactNotFound)?;
    let actual_sha = format!("{:x}", Sha256::digest(&artifact_bytes));

    if receipt.durable_evidence_sha != actual_sha {
        return Err(RepairReceiptError::EvidenceHashMismatch {
            expected: receipt.durable_evidence_sha.clone(),
            actual: actual_sha,
        });
    }

    Ok(())
}

/// Errors for receipt verification.
#[derive(Debug, Error)]
pub enum RepairReceiptError {
    /// The repair artifact file could not be found.
    #[error("repair artifact not found at path")]
    ArtifactNotFound,
    /// The evidence hash in the receipt does not match the actual artifact hash.
    #[error("receipt evidence hash mismatch (want {expected}, got {actual})")]
    EvidenceHashMismatch {
        /// Expected hash from the receipt.
        expected: String,
        /// Actual hash computed from the artifact.
        actual: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn receipt_fixture() -> RepairReceipt {
        let created_at = OffsetDateTime::now_utc();
        RepairReceipt {
            cycle_id: "p-52b95ef55999f9de/cycle-44-build-remediate-transition".to_string(),
            code: "VAULT003".to_string(),
            node: "test-node".to_string(),
            target: "p-52b95ef55999f9de/cycle-44-build-remediate-transition".to_string(),
            repair_action: RepairAction::NodeCreation,
            durable_evidence_sha: "abc123".to_string(),
            created_at,
            valid_to: created_at + time::Duration::days(90),
        }
    }

    #[test]
    fn load_repair_queue_empty_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("repair-queue.yaml");
        let queue = load_repair_queue(&path).unwrap();
        assert!(queue.is_empty());
    }

    #[test]
    fn append_refuses_overwrite() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("repair-queue.yaml");

        let receipt = receipt_fixture();
        append_repair_receipt(&path, &receipt).unwrap();

        let result = append_repair_receipt(&path, &receipt);
        assert!(result.is_err());
        match result.unwrap_err() {
            RepairQueueError::QueueMalformed(msg) => {
                assert!(msg.contains("already exists"));
            }
            _ => panic!("expected QueueMalformed error"),
        }
    }

    #[test]
    fn append_enforces_90_day_window() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("repair-queue.yaml");

        let mut receipt = receipt_fixture();
        // valid_to exceeds 90 days
        receipt.valid_to = receipt.created_at + time::Duration::days(91);

        let result = append_repair_receipt(&path, &receipt);
        assert!(result.is_err());
        match result.unwrap_err() {
            RepairQueueError::InvalidRfc3339(_) => {}
            _ => panic!("expected InvalidRfc3339 error"),
        }
    }

    #[test]
    fn load_queue_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("repair-queue.yaml");

        let receipt = receipt_fixture();
        append_repair_receipt(&path, &receipt).unwrap();

        let queue = load_repair_queue(&path).unwrap();
        assert_eq!(queue.len(), 1);

        let key = "p-52b95ef55999f9de/cycle-44-build-remediate-transition/VAULT003/test-node";
        let loaded = queue.get(key).unwrap();
        assert_eq!(loaded.cycle_id, receipt.cycle_id);
        assert_eq!(loaded.code, receipt.code);
    }
}
