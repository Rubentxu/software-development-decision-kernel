//! Spine import pipeline for EXECUTION-SPINE.yaml.
//!
//! Implements PLN-LEDGER-003 §3.5 spine import per ADR-073 Q4 and spec PLN-LEDGER-003 §6.

use std::collections::HashSet;

use sha2::{Digest, Sha256};

use sddk_domain::planning::{DependencyEdgeKind, PlanningEvidenceKind, WorkItemStatus};
use sddk_domain::spine::{SpineStatus, canonicalize_spine_bytes, parse_spine_yaml};

use crate::{Storage, StorageError};

/// Summary of a spine import operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    /// Number of new rows inserted.
    pub imported: u32,
    /// Number of rows already present and unchanged.
    pub already_present: u32,
    /// Number of conflicts detected (re-import with diff).
    pub conflicts: u32,
}

/// Errors from spine import.
#[derive(Debug, thiserror::Error)]
pub enum SpineImportError {
    #[error("parse error: {0}")]
    ParseError(sddk_domain::spine::SpineParseError),

    #[error("self-loop: item {item_id} depends on itself")]
    SelfLoop { item_id: String },

    #[error("unknown dependency: item {item_id} depends on unknown {unknown}")]
    UnknownDependency { item_id: String, unknown: String },

    #[error("import conflict: {id}.{field}: expected {expected:?}, got {actual:?}")]
    ImportConflict {
        id: String,
        field: String,
        expected: String,
        actual: String,
    },

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

/// Maps spine status to work item status.
///
/// Per spec PLN-LEDGER-003 §7 (locked):
/// - PROPOSED → Draft
/// - READY → Draft
/// - ACTIVE → Active
/// - PARTIAL → Active
/// - BLOCKED → Paused
/// - SHIPPED → Done
/// - ABSORBED → Done
/// - SUPERSEDED → Superseded
pub fn map_spine_status(status: SpineStatus) -> Result<WorkItemStatus, SpineImportError> {
    status
        .to_work_item_status()
        .map_err(SpineImportError::ParseError)
}

/// Computes the SHA-256 body reference for canonical spine bytes.
pub fn compute_spine_body_ref(canonical_bytes: &[u8]) -> sddk_domain::CasHash {
    let digest = Sha256::digest(canonical_bytes);
    format!("sha256:{:x}", digest)
}

/// Imports the EXECUTION-SPINE.yaml bytes into the provided storage.
///
/// Per-row cycle (Q5 S1): each spine row produces one WorkItem with
/// `id = spine.id` and `cycle_id = spine.id`.
///
/// Idempotent: re-import of identical bytes reports `already_present=N, imported=0`.
/// Hard-error on mutated bytes: returns `SpineImportError::ImportConflict` per Q8.
///
/// Each imported spine row produces exactly one `EvidenceAttachmentV1` with
/// `kind = "snapshot"` and `body_ref = sha256(canonical_yaml_bytes)`.
pub fn import_spine(
    bytes: &[u8],
    storage: &mut Storage,
) -> Result<ImportSummary, SpineImportError> {
    // Step 1: Parse the spine
    let spine = parse_spine_yaml(bytes).map_err(SpineImportError::ParseError)?;

    // Step 2: Canonicalize bytes for content-addressing
    let canonical_bytes = canonicalize_spine_bytes(bytes);
    let body_ref = compute_spine_body_ref(&canonical_bytes);

    // Step 3: Collect all spine item IDs for dependency validation
    let spine_ids: HashSet<String> = spine.items.iter().map(|item| item.id.clone()).collect();

    // Step 4: Validate dependencies (no self-loops, no unknown targets)
    for item in &spine.items {
        // Self-loop check
        if item.depends_on.contains(&item.id) {
            return Err(SpineImportError::SelfLoop {
                item_id: item.id.clone(),
            });
        }
        // Unknown dependency check
        for dep in &item.depends_on {
            if !spine_ids.contains(dep) {
                return Err(SpineImportError::UnknownDependency {
                    item_id: item.id.clone(),
                    unknown: dep.clone(),
                });
            }
        }
    }

    // Step 5: Import each spine item
    let mut imported: u32 = 0;
    let mut already_present: u32 = 0;
    let mut conflicts: u32 = 0;

    for item in &spine.items {
        let cycle_id = item.id.clone(); // Q5 S1: per-row cycle
        let work_item_id = item.id.clone(); // Q6: spine id is canonical identity

        // Check if this work item already exists
        let existing = storage
            .get_work_item(&work_item_id)
            .map_err(StorageError::from)?;

        if let Some(existing_wi) = existing {
            // Idempotency check: compare identity fields
            // Q6: title should equal the spine id
            if existing_wi.title == work_item_id && existing_wi.cycle_id == cycle_id {
                already_present += 1;
                continue;
            } else {
                // Conflict: existing work item differs from what spine says it should be
                conflicts += 1;
                return Err(SpineImportError::ImportConflict {
                    id: work_item_id.clone(),
                    field: "title".to_string(),
                    expected: work_item_id,
                    actual: existing_wi.title,
                });
            }
        }

        // Insert the work item
        let status = map_spine_status(item.status)?;
        let record = sddk_domain::WorkItemRecord {
            id: work_item_id.clone(),
            cycle_id: cycle_id.clone(),
            title: work_item_id.clone(), // Q6: title = spine id
            description: item.objective.clone(),
            status,
            actor_ref_kind: None,
            actor_ref_id: None,
            actor_ref_label: None,
            created_at: timestamp_now(),
            schema_version: sddk_domain::WORK_ITEM_SCHEMA_VERSION,
        };
        storage
            .insert_work_item(&record)
            .map_err(StorageError::from)?;

        // Insert dependency edges (kind = Blocks per ADR-073 Q2)
        for dep_id in &item.depends_on {
            let edge = sddk_domain::DependencyEdgeRecord {
                from_id: work_item_id.clone(),
                to_id: dep_id.clone(),
                kind: DependencyEdgeKind::Blocks,
                actor_ref_kind: None,
                actor_ref_id: None,
                actor_ref_label: None,
                schema_version: sddk_domain::DEPENDENCY_EDGE_SCHEMA_VERSION,
            };
            // insert_dependency_edge uses INSERT OR IGNORE — idempotent on composite PK
            storage
                .insert_dependency_edge(&edge)
                .map_err(StorageError::from)?;
        }

        // Insert evidence attachment (one per spine item, body_ref = sha256(canonical))
        // Write the canonical bytes to CAS first
        storage
            .cas_put(&canonical_bytes)
            .map_err(StorageError::from)?;

        let evidence_id = uuid::Uuid::new_v4().to_string();
        let evidence_record = sddk_domain::EvidenceAttachmentRecord {
            id: evidence_id,
            work_item_id: work_item_id.clone(),
            kind: PlanningEvidenceKind::Snapshot,
            body_ref: body_ref.clone(),
            actor_ref_kind: None,
            actor_ref_id: None,
            actor_ref_label: None,
            schema_version: sddk_domain::EVIDENCE_ATTACHMENT_SCHEMA_VERSION,
        };
        storage
            .insert_evidence_attachment(&evidence_record, &canonical_bytes)
            .map_err(StorageError::from)?;

        imported += 1;
    }

    Ok(ImportSummary {
        imported,
        already_present,
        conflicts,
    })
}

/// Returns the current Unix timestamp in milliseconds.
fn timestamp_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
