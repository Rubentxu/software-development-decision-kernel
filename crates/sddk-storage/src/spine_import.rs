//! Spine import pipeline for EXECUTION-SPINE.yaml.
//!
//! Implements PLN-LEDGER-003 §3.5 spine import per ADR-073 Q4 and spec PLN-LEDGER-003 §6.

use std::collections::HashSet;

use sha2::{Digest, Sha256};

use sddk_domain::planning::{DependencyEdgeKind, PlanningEvidenceKind, WorkItemStatus};
use sddk_domain::spine::{SpineStatus, canonicalize_spine_bytes, parse_spine_yaml};

use crate::{Storage, StorageError};
use rusqlite::params;

/// Well-known project ID used for all spine-imported items.
/// This project is created automatically if it doesn't exist.
const SPINE_IMPORT_PROJECT_ID: &str = "__spine_import__";

/// Ensures the spine-import project and workspace exist, creating if necessary.
fn ensure_spine_import_project_and_workspace(storage: &Storage) -> Result<(), SpineImportError> {
    let workspace_id = "__spine_import_ws__";

    // Check if project exists
    let project_exists: bool = storage
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE project_id = ?1)",
            [SPINE_IMPORT_PROJECT_ID],
            |row| row.get(0),
        )
        .map_err(StorageError::Database)
        .map_err(SpineImportError::Storage)?;

    if !project_exists {
        let _ = storage
            .connection
            .execute(
                r#"INSERT INTO projects (project_id, display_name, scope, created_at)
                   VALUES (?1, 'Spine Import Project', 'spine-import', datetime('now'))
                   ON CONFLICT(project_id) DO NOTHING"#,
                params![SPINE_IMPORT_PROJECT_ID],
            )
            .map_err(StorageError::Database)
            .map_err(SpineImportError::Storage)?;
    }

    // Check if workspace exists
    let workspace_exists: bool = storage
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE workspace_id = ?1)",
            [workspace_id],
            |row| row.get(0),
        )
        .map_err(StorageError::Database)
        .map_err(SpineImportError::Storage)?;

    if !workspace_exists {
        let _ = storage
            .connection
            .execute(
                r#"INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
                   VALUES (?1, ?2, 'spine-import', datetime('now'))
                   ON CONFLICT(workspace_id) DO NOTHING"#,
                params![workspace_id, SPINE_IMPORT_PROJECT_ID],
            )
            .map_err(StorageError::Database)
            .map_err(SpineImportError::Storage)?;
    }

    Ok(())
}

/// Ensures a cycle row exists for the given cycle_id, creating it if necessary.
/// Uses INSERT OR IGNORE so it's idempotent.
fn ensure_cycle_exists(storage: &Storage, cycle_id: &str) -> Result<(), SpineImportError> {
    let workspace_id = "__spine_import_ws__";
    let manifest_json = "{}";

    let _ = storage
        .connection
        .execute(
            r#"INSERT OR IGNORE INTO cycles
                   (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
               VALUES
                   (?1, ?2, ?3, 'OPEN', 'build', ?4, datetime('now'), datetime('now'))"#,
            params![cycle_id, SPINE_IMPORT_PROJECT_ID, workspace_id, manifest_json],
        )
        .map_err(StorageError::Database)
        .map_err(SpineImportError::Storage)?;
    Ok(())
}

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

    // Step 5: Ensure the spine-import project and cycle rows exist for all spine items.
    // The work_items_v1.cycle_id FK references cycles(cycle_id), so each spine item's
    // cycle must be registered before the work item can be inserted.
    ensure_spine_import_project_and_workspace(storage)?;
    for item in &spine.items {
        ensure_cycle_exists(storage, &item.id)?;
    }

    // Step 6: Import each spine item
    let mut imported: u32 = 0;
    let mut already_present: u32 = 0;
    let mut conflicts: u32 = 0;

    for item in &spine.items {
        let cycle_id = item.id.clone(); // Q5 S1: per-row cycle
        let work_item_id = item.id.clone(); // Q6: spine id is canonical identity
        let expected_description = item.objective.clone();
        let expected_status = map_spine_status(item.status)?;

        // Check if this work item already exists
        let existing = storage.get_work_item(&work_item_id)?;

        if let Some(existing_wi) = existing {
            // Idempotency check: compare all identity fields.
            // Per Q6, title = spine id; per Q5, cycle_id = spine id.
            // Conflict if description or status differs (Q8).
            if existing_wi.title == work_item_id
                && existing_wi.cycle_id == cycle_id
                && existing_wi.description == expected_description
                && existing_wi.status == expected_status
            {
                already_present += 1;
                continue;
            } else {
                // Conflict: existing work item differs from what spine says it should be
                let field = if existing_wi.description != expected_description {
                    "description"
                } else {
                    "status"
                };
                conflicts += 1;
                return Err(SpineImportError::ImportConflict {
                    id: work_item_id.clone(),
                    field: field.to_string(),
                    expected: expected_description.clone(),
                    actual: existing_wi.description,
                });
            }
        }

        // Insert the work item
        let status = expected_status;
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
        storage.insert_work_item(&record)?;

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
            storage.insert_dependency_edge(&edge)?;
        }

        // Insert evidence attachment (one per spine item, body_ref = sha256(canonical))
        // Write the canonical bytes to CAS first
        storage.cas_put(&canonical_bytes)?;

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
        storage.insert_evidence_attachment(&evidence_record, &canonical_bytes)?;

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
