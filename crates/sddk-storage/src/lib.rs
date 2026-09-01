//! SQLite-backed operational persistence for SDDK.
//!
//! The crate stores project identity, workspaces, cycle snapshots, immutable
//! hash-linked ledger events, artifact metadata, capability receipts, and cycle
//! leases. Callers supply all timestamps; this crate never reads the system clock.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

pub mod control_plane;
pub mod event_store;
pub mod fork_store;
pub mod graph_store;
mod migrations;
mod models;
pub mod projection_store;
pub mod rebuild;
pub use control_plane::{ProjectStatusRow, SCHEMA_V1, SqliteControlPlane};
pub use event_store::SqliteEventStore;
pub use fork_store::SqliteForkStore;
pub use graph_store::SqliteGraphStore;
pub use projection_store::SqliteProjectionStore;
pub use rebuild::rebuild;

use std::path::Path;
use std::time::Duration;

use migrations::{LATEST_SCHEMA_VERSION, run_migrations};
pub use models::*;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Row, Transaction, TransactionBehavior, params,
};
use sddk_domain::CycleManifest;
use sddk_domain::ports::{ArtifactStore, LedgerFactory};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Result type returned by storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;

/// Errors emitted by the SQLite storage boundary.
#[derive(Debug, Error)]
pub enum StorageError {
    /// SQLite rejected an operation.
    #[error("sqlite storage error: {0}")]
    Database(#[from] rusqlite::Error),
    /// A persisted JSON value could not be encoded or decoded.
    #[error("storage serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// A database parent directory could not be created.
    #[error("storage filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// A requested record does not exist.
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Entity kind.
        entity: &'static str,
        /// Missing entity identifier.
        id: String,
    },
    /// An idempotency key was reused for a different request.
    #[error("idempotency key {key:?} was already used for a different request")]
    IdempotencyConflict {
        /// Conflicting idempotency key.
        key: String,
    },
    /// A capability receipt must be created in the started state.
    #[error("capability receipt must begin in started status")]
    InvalidReceiptBegin,
    /// A capability receipt can only be finalized from the started state.
    #[error("capability receipt {receipt_id} is already terminal")]
    TerminalReceipt {
        /// Receipt that cannot transition again.
        receipt_id: String,
    },
    /// A non-expired lease is owned by another runtime.
    #[error("cycle {cycle_id:?} is leased by {owner:?} until {expires_at_ms}")]
    LeaseConflict {
        /// Contended cycle identifier.
        cycle_id: String,
        /// Current lease owner.
        owner: String,
        /// Current lease expiry in Unix milliseconds.
        expires_at_ms: i64,
    },
    /// The lease's `expires_at_ms` is at or before `now_ms`; the caller must
    /// re-acquire before any protected operation succeeds.
    #[error(
        "cycle {cycle_id:?} lease held by {owner:?} (token={fencing_token}) expired at \
         {expires_at_ms} (now={now_ms}); re-acquire before retrying"
    )]
    LeaseExpired {
        /// Contended cycle identifier.
        cycle_id: String,
        /// Persisted lease owner.
        owner: String,
        /// Persisted fencing token.
        fencing_token: i64,
        /// Persisted lease expiry in Unix milliseconds.
        expires_at_ms: i64,
        /// Caller-supplied current time in Unix milliseconds.
        now_ms: i64,
    },
    /// Lease times do not define a positive interval.
    #[error("lease expiry must be greater than acquisition time")]
    InvalidLease,
    /// A `renew` was attempted for a lease that the caller does not hold.
    #[error(
        "cycle {cycle_id:?} lease is not renewable: persisted owner={current_owner:?} \
         persisted fencing_token={current_fencing_token}; pass --fencing-token if you \
         actually hold the current lease"
    )]
    LeaseNotRenewable {
        /// Contended cycle identifier.
        cycle_id: String,
        /// Currently persisted lease owner (empty when the row is absent).
        current_owner: String,
        /// Currently persisted fencing token (zero when the row is absent).
        current_fencing_token: i64,
    },
    /// Cycle state and event input refer to different cycles or projects.
    #[error("cycle state and ledger event identifiers do not match")]
    EventScopeMismatch,
    /// Existing identity data disagrees with an idempotent registration request.
    #[error("adoption registration conflicts with existing {entity}: {id}")]
    RegistrationConflict {
        /// Conflicting entity kind.
        entity: &'static str,
        /// Conflicting entity identifier.
        id: String,
    },
    /// A read-only database does not use the expected schema version.
    #[error("unsupported storage schema version {actual}; expected {expected}")]
    SchemaVersion {
        /// Version found in SQLite.
        actual: i32,
        /// Version supported by this runtime.
        expected: i32,
    },
    /// The ledger sequence or hash chain is invalid.
    #[error("ledger integrity failure at sequence {sequence}: {reason}")]
    LedgerIntegrity {
        /// Sequence at which verification failed.
        sequence: i64,
        /// Human-readable integrity failure.
        reason: String,
    },
    /// The plan_hash is too short to slice for the receipt_id format.
    #[error("plan_hash is too short: {actual} chars, need at least 23 (got {actual})")]
    PlanHashTooShort {
        /// Actual length of the provided plan_hash.
        actual: usize,
        /// Required minimum length.
        required: usize,
    },
    /// The gate name violates the 1..=128 char limit imposed by
    /// [`RID_FORMAT_REGEX`] on the receipt_id format.
    #[error("gate name is invalid: {actual} chars, must be {min}..={max}")]
    GateNameInvalid {
        /// Actual length of the provided gate name.
        actual: usize,
        /// Minimum allowed length.
        min: usize,
        /// Maximum allowed length.
        max: usize,
    },
    /// The cycle's project prefix does not match the workspace's adopted project.
    #[error(
        "cycle {cycle_id} belongs to project {cycle_project_id}, \
         which does not match the current workspace adoption ({expected_project_id})"
    )]
    CycleProjectMismatch {
        /// The cycle identifier supplied by the caller.
        cycle_id: String,
        /// The project extracted from the cycle's prefix.
        cycle_project_id: String,
        /// The project the workspace has adopted.
        expected_project_id: String,
    },
}

/// SQLite-backed SDDK persistence.
pub struct Storage {
    connection: Connection,
}

/// Canonical regex for a gate receipt identifier produced by
/// [`Storage::insert_gate_receipt_next_seq`](Storage::insert_gate_receipt_next_seq).
/// Format: `gate-{gate(1..128)}-{plan_hash[7..23]}-{seq}`.
pub const RID_FORMAT_REGEX: &str = r"^gate-.{1,128}-[0-9a-f]{16}-[0-9]+$";

impl Storage {
    /// Opens or creates a database and applies all pending migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?, true)
    }

    /// Opens an existing database without creating files or applying migrations.
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Self::from_connection(connection, false)
    }

    /// Opens an isolated in-memory database and applies all migrations.
    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?, true)
    }

    fn from_connection(mut connection: Connection, writable: bool) -> Result<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        if writable {
            connection.pragma_update(None, "journal_mode", "WAL")?;
            migrate(&mut connection)?;
        } else {
            let actual: i32 =
                connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
            if actual != LATEST_SCHEMA_VERSION {
                return Err(StorageError::SchemaVersion {
                    actual,
                    expected: LATEST_SCHEMA_VERSION,
                });
            }
        }
        Ok(Self { connection })
    }

    /// Returns the currently applied storage schema version.
    pub fn schema_version(&self) -> Result<i32> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    /// Inserts a logical project.
    pub fn insert_project(&self, project: &ProjectRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO projects (
                project_id, display_name, remote_url, scope, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                project.project_id,
                project.display_name,
                project.remote_url,
                project.scope,
                project.created_at
            ],
        )?;
        Ok(())
    }

    /// Loads a logical project by identifier.
    pub fn get_project(&self, project_id: &str) -> Result<ProjectRecord> {
        self.get_project_optional(project_id)?
            .ok_or_else(|| not_found("project", project_id))
    }

    /// Loads a logical project when present.
    pub fn get_project_optional(&self, project_id: &str) -> Result<Option<ProjectRecord>> {
        Ok(self
            .connection
            .query_row(
                "SELECT project_id, display_name, remote_url, scope, created_at
                 FROM projects WHERE project_id = ?1",
                [project_id],
                |row| {
                    Ok(ProjectRecord {
                        project_id: row.get(0)?,
                        display_name: row.get(1)?,
                        remote_url: row.get(2)?,
                        scope: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// Inserts a workspace belonging to an existing project.
    pub fn insert_workspace(&self, workspace: &WorkspaceRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO workspaces (
                workspace_id, project_id, canonical_path, created_at
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                workspace.workspace_id,
                workspace.project_id,
                workspace.canonical_path,
                workspace.created_at
            ],
        )?;
        Ok(())
    }

    /// Loads a workspace by identifier.
    pub fn get_workspace(&self, workspace_id: &str) -> Result<WorkspaceRecord> {
        self.get_workspace_optional(workspace_id)?
            .ok_or_else(|| not_found("workspace", workspace_id))
    }

    /// Loads a workspace when present.
    pub fn get_workspace_optional(&self, workspace_id: &str) -> Result<Option<WorkspaceRecord>> {
        Ok(self
            .connection
            .query_row(
                "SELECT workspace_id, project_id, canonical_path, created_at
                 FROM workspaces WHERE workspace_id = ?1",
                [workspace_id],
                |row| {
                    Ok(WorkspaceRecord {
                        workspace_id: row.get(0)?,
                        project_id: row.get(1)?,
                        canonical_path: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// Reports whether the database contains any project registration.
    pub fn has_projects(&self) -> Result<bool> {
        Ok(self
            .connection
            .query_row("SELECT EXISTS(SELECT 1 FROM projects)", [], |row| {
                row.get(0)
            })?)
    }

    /// Registers a project and workspace in one SQLite transaction.
    ///
    /// Replaying matching identity data is a no-op. Existing identity data that
    /// disagrees with the request is rejected rather than overwritten.
    pub fn register_project_workspace(
        &mut self,
        project: &ProjectRecord,
        workspace: &WorkspaceRecord,
    ) -> Result<()> {
        if workspace.project_id != project.project_id {
            return Err(StorageError::RegistrationConflict {
                entity: "workspace project",
                id: workspace.workspace_id.clone(),
            });
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing_project = project_optional_on(&transaction, &project.project_id)?;
        match existing_project {
            Some(existing)
                if existing.remote_url != project.remote_url || existing.scope != project.scope =>
            {
                return Err(StorageError::RegistrationConflict {
                    entity: "project",
                    id: project.project_id.clone(),
                });
            }
            Some(_) => {}
            None => {
                let has_other: bool =
                    transaction.query_row("SELECT EXISTS(SELECT 1 FROM projects)", [], |row| {
                        row.get(0)
                    })?;
                if has_other {
                    return Err(StorageError::RegistrationConflict {
                        entity: "project",
                        id: project.project_id.clone(),
                    });
                }
                transaction.execute(
                    "INSERT INTO projects (
                        project_id, display_name, remote_url, scope, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        project.project_id,
                        project.display_name,
                        project.remote_url,
                        project.scope,
                        project.created_at
                    ],
                )?;
            }
        }
        let existing_workspace = workspace_optional_on(&transaction, &workspace.workspace_id)?;
        match existing_workspace {
            Some(existing)
                if existing.project_id != workspace.project_id
                    || existing.canonical_path != workspace.canonical_path =>
            {
                return Err(StorageError::RegistrationConflict {
                    entity: "workspace",
                    id: workspace.workspace_id.clone(),
                });
            }
            Some(_) => {}
            None => {
                transaction.execute(
                    "INSERT INTO workspaces (
                        workspace_id, project_id, canonical_path, created_at
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        workspace.workspace_id,
                        workspace.project_id,
                        workspace.canonical_path,
                        workspace.created_at
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Inserts a cycle snapshot without a ledger event.
    ///
    /// Runtime code should normally prefer [`Storage::insert_cycle_with_event`]
    /// so the initial state and causal event are committed atomically.
    pub fn insert_cycle(&self, cycle: &CycleRecord) -> Result<()> {
        insert_cycle_on(&self.connection, cycle)
    }

    /// Inserts a cycle snapshot and its initial event atomically.
    pub fn insert_cycle_with_event(
        &mut self,
        cycle: &CycleRecord,
        event: &LedgerEventInput,
    ) -> Result<LedgerEvent> {
        ensure_event_scope(&cycle.manifest, event)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        insert_cycle_on(&transaction, cycle)?;
        let appended = append_event_on(&transaction, event)?;
        transaction.commit()?;
        Ok(appended)
    }

    /// Loads a cycle snapshot by identifier.
    pub fn get_cycle(&self, cycle_id: &str) -> Result<CycleRecord> {
        self.connection
            .query_row(
                "SELECT manifest_json, created_at, updated_at
                 FROM cycles WHERE cycle_id = ?1",
                [cycle_id],
                cycle_from_row,
            )
            .optional()?
            .ok_or_else(|| not_found("cycle", cycle_id))
    }

    /// Replaces a cycle snapshot and appends its causal event atomically.
    ///
    /// When `release_lease_on_phase_change` is `true`, the method also
    /// deletes the `cycles_lease` row and appends a `lease.released` ledger
    /// event inside the same transaction. The caller (typically
    /// `Engine::apply_transition`) opts in only when the transition changes
    /// the cycle's `phase` and the outcome is `Succeeded`; on rollback both
    /// the cycle update and the lease release are discarded.
    pub fn update_cycle_with_event(
        &mut self,
        manifest: &CycleManifest,
        updated_at: &str,
        event: &LedgerEventInput,
        release_lease_on_phase_change: bool,
    ) -> Result<LedgerEvent> {
        ensure_event_scope(manifest, event)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE cycles SET
                project_id = ?2,
                workspace_id = ?3,
                status = ?4,
                phase = ?5,
                manifest_json = ?6,
                updated_at = ?7
             WHERE cycle_id = ?1",
            params![
                manifest.cycle_id,
                manifest.project_id,
                manifest.workspace_id,
                enum_string(&manifest.status)?,
                enum_string(&manifest.phase)?,
                serde_json::to_string(manifest)?,
                updated_at
            ],
        )?;
        if changed == 0 {
            return Err(not_found("cycle", &manifest.cycle_id));
        }
        let appended = append_event_on(&transaction, event)?;
        if release_lease_on_phase_change {
            let deleted = transaction
                .execute(
                    "DELETE FROM cycle_leases WHERE cycle_id = ?1",
                    [&manifest.cycle_id],
                )
                .map_err(StorageError::from)?;
            if deleted > 0 {
                let release_event = LedgerEventInput {
                    event_id: format!("evt-lease-released-{}", uuid::Uuid::new_v4().hyphenated()),
                    project_id: manifest.project_id.clone(),
                    cycle_id: Some(manifest.cycle_id.clone()),
                    frame_id: event.frame_id.clone(),
                    command_id: event.command_id.clone(),
                    actor: event.actor.clone(),
                    event_type: "lease.released".to_owned(),
                    occurred_at: event.occurred_at.clone(),
                    state_before: None,
                    state_after: None,
                    payload: json!({
                        "cycle_id": manifest.cycle_id,
                        "released_at_ms": updated_at,
                    }),
                };
                append_event_on(&transaction, &release_event)?;
            }
        }
        transaction.commit()?;
        Ok(appended)
    }

    /// Appends one immutable event to the ledger.
    pub fn append_event(&mut self, event: &LedgerEventInput) -> Result<LedgerEvent> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let appended = append_event_on(&transaction, event)?;
        transaction.commit()?;
        Ok(appended)
    }

    /// Lists all ledger events in ascending sequence order.
    pub fn list_events(&self) -> Result<Vec<LedgerEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, event_id, project_id, cycle_id, frame_id,
                    command_id, actor, event_type, occurred_at,
                    state_before_json, state_after_json, payload_json,
                    previous_hash, event_hash
             FROM ledger_events ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([], event_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Lists ledger events for one cycle in ascending global sequence order.
    pub fn list_cycle_events(&self, cycle_id: &str) -> Result<Vec<LedgerEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, event_id, project_id, cycle_id, frame_id,
                    command_id, actor, event_type, occurred_at,
                    state_before_json, state_after_json, payload_json,
                    previous_hash, event_hash
             FROM ledger_events WHERE cycle_id = ?1 ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([cycle_id], event_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Loads all ledger events in ascending sequence order.
    ///
    /// Used by telemetry ingest to derive metrics for cycles that have no
    /// metrics.jsonl entry.
    pub fn load_all_ledger_events(&self) -> Result<Vec<LedgerEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, event_id, project_id, cycle_id, frame_id,
                    command_id, actor, event_type, occurred_at,
                    state_before_json, state_after_json, payload_json,
                    previous_hash, event_hash
             FROM ledger_events ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([], event_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Lists ledger events sharing one command frame in ascending sequence order.
    pub fn list_frame_events(&self, frame_id: &str) -> Result<Vec<LedgerEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, event_id, project_id, cycle_id, frame_id,
                    command_id, actor, event_type, occurred_at,
                    state_before_json, state_after_json, payload_json,
                    previous_hash, event_hash
             FROM ledger_events WHERE frame_id = ?1 ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([frame_id], event_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Deletes only the materialized cycle snapshot, preserving its ledger events.
    ///
    /// This is a destructive repair primitive used by rebuild workflows and
    /// tests that simulate a lost snapshot. Foreign-key enforcement is suspended
    /// for the delete because ledger events reference the snapshot row. The
    /// causal ledger itself is untouched.
    pub fn delete_cycle_snapshot(&self, cycle_id: &str) -> Result<()> {
        self.connection.pragma_update(None, "foreign_keys", false)?;
        let result = self
            .connection
            .execute("DELETE FROM cycles WHERE cycle_id = ?1", [cycle_id]);
        self.connection.pragma_update(None, "foreign_keys", true)?;
        let changed = result?;
        if changed == 0 {
            return Err(not_found("cycle", cycle_id));
        }
        Ok(())
    }

    /// Verifies that the current lease still matches the caller's fencing
    /// token and has not expired at `now_ms`. A lease whose
    /// `expires_at_ms <= now_ms` is rejected with [`StorageError::LeaseExpired`]
    /// even when the owner and fencing token match, so that protected
    /// operations fail-closed once the lease instant has elapsed.
    pub fn verify_cycle_lease(
        &self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
    ) -> Result<CycleLease> {
        let lease = self
            .get_cycle_lease_on_optional(cycle_id)?
            .ok_or_else(|| not_found("cycle lease", cycle_id))?;
        if lease.owner != owner || lease.fencing_token != fencing_token {
            return Err(StorageError::LeaseConflict {
                cycle_id: cycle_id.to_owned(),
                owner: lease.owner,
                expires_at_ms: lease.expires_at_ms,
            });
        }
        if lease.expires_at_ms <= now_ms {
            return Err(StorageError::LeaseExpired {
                cycle_id: cycle_id.to_owned(),
                owner: lease.owner,
                fencing_token: lease.fencing_token,
                expires_at_ms: lease.expires_at_ms,
                now_ms,
            });
        }
        Ok(lease)
    }

    /// Verifies sequence continuity, predecessor links, and event hashes.
    pub fn verify_ledger(&self) -> Result<LedgerVerification> {
        let events = self.list_events()?;
        let mut previous_hash: Option<String> = None;
        for (expected_sequence, event) in (1_i64..).zip(&events) {
            if event.sequence != expected_sequence {
                return Err(integrity_error(event.sequence, "sequence gap"));
            }
            if event.previous_hash != previous_hash {
                return Err(integrity_error(event.sequence, "previous hash mismatch"));
            }
            let expected_hash = hash_event(event.sequence, &event.as_input(), &previous_hash)?;
            if event.event_hash != expected_hash {
                return Err(integrity_error(event.sequence, "event hash mismatch"));
            }
            previous_hash = Some(event.event_hash.clone());
        }
        Ok(LedgerVerification {
            event_count: events.len(),
            last_hash: previous_hash,
        })
    }

    /// Inserts artifact metadata. Artifact bytes remain in the external store.
    pub fn insert_artifact(&self, artifact: &ArtifactRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO artifacts (
                artifact_id, project_id, cycle_id, kind, path, sha256,
                producer, created_at, metadata_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                artifact.artifact_id,
                artifact.project_id,
                artifact.cycle_id,
                artifact.kind,
                artifact.path,
                artifact.sha256,
                artifact.producer,
                artifact.created_at,
                serde_json::to_string(&artifact.metadata)?
            ],
        )?;
        Ok(())
    }

    /// Loads artifact metadata by identifier.
    pub fn get_artifact(&self, artifact_id: &str) -> Result<ArtifactRecord> {
        self.connection
            .query_row(
                "SELECT artifact_id, project_id, cycle_id, kind, path, sha256,
                        producer, created_at, metadata_json
                 FROM artifacts WHERE artifact_id = ?1",
                [artifact_id],
                artifact_from_row,
            )
            .optional()?
            .ok_or_else(|| not_found("artifact", artifact_id))
    }

    /// Lists all artifact metadata for a project.
    pub fn list_project_artifacts(&self, project_id: &str) -> Result<Vec<ArtifactRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT artifact_id, project_id, cycle_id, kind, path, sha256,
                    producer, created_at, metadata_json
             FROM artifacts WHERE project_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([project_id], artifact_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Records the start of a capability execution exactly once for an idempotency key.
    ///
    /// The receipt is created in the started state only. Reusing the key with the
    /// same request returns the original receipt; reusing it with a different
    /// request returns [`StorageError::IdempotencyConflict`]. Terminal states are
    /// written exclusively through [`Storage::finalize_capability_receipt`].
    pub fn begin_capability_receipt(
        &mut self,
        input: &CapabilityReceiptInput,
    ) -> Result<CapabilityReceipt> {
        if input.status != CapabilityStatus::Started {
            return Err(StorageError::InvalidReceiptBegin);
        }
        let request_json = serde_json::to_string(&input.request)?;
        let request_hash = hash_capability_request(input)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing = transaction
            .query_row(
                "SELECT request_hash, receipt_id FROM idempotency_records
                 WHERE project_id = ?1 AND idempotency_key = ?2",
                params![input.project_id, input.idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        if let Some((existing_hash, receipt_id)) = existing {
            if existing_hash != request_hash {
                return Err(StorageError::IdempotencyConflict {
                    key: input.idempotency_key.clone(),
                });
            }
            let receipt = get_capability_receipt_on(&transaction, &receipt_id)?;
            transaction.commit()?;
            return Ok(receipt);
        }

        transaction.execute(
            "INSERT INTO capability_receipts (
                receipt_id, project_id, cycle_id, capability, request_hash,
                request_json, status, result_json, started_at, completed_at,
                agent_version_hash, behavior_version_hash
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                input.receipt_id,
                input.project_id,
                input.cycle_id,
                input.capability,
                request_hash,
                request_json,
                enum_string(&input.status)?,
                optional_json(&input.result)?,
                input.started_at,
                input.completed_at,
                input.agent_version_hash,
                input.behavior_version_hash
            ],
        )?;
        transaction.execute(
            "INSERT INTO idempotency_records (
                project_id, idempotency_key, request_hash, receipt_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                input.project_id,
                input.idempotency_key,
                request_hash,
                input.receipt_id,
                input.started_at
            ],
        )?;
        let receipt = get_capability_receipt_on(&transaction, &input.receipt_id)?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Finalizes a capability receipt from the started state.
    ///
    /// Accepts only terminal outcomes (`Succeeded`, `Failed`, `Unknown`) and
    /// rejects transitions on receipts that are already terminal.
    pub fn finalize_capability_receipt(
        &mut self,
        receipt_id: &str,
        status: CapabilityStatus,
        result: Option<Value>,
        completed_at: &str,
    ) -> Result<CapabilityReceipt> {
        self.finalize_capability_receipt_with_hashes(
            receipt_id,
            status,
            result,
            completed_at,
            None,
            None,
        )
    }

    ///
    /// Accepts only terminal outcomes (`Succeeded`, `Failed`, `Unknown`) and
    /// rejects transitions on receipts that are already terminal.
    /// Optionally updates the version hashes if provided.
    pub fn finalize_capability_receipt_with_hashes(
        &mut self,
        receipt_id: &str,
        status: CapabilityStatus,
        result: Option<Value>,
        completed_at: &str,
        agent_version_hash: Option<String>,
        behavior_version_hash: Option<String>,
    ) -> Result<CapabilityReceipt> {
        if status == CapabilityStatus::Started {
            return Err(StorageError::InvalidReceiptBegin);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = get_capability_receipt_on(&transaction, receipt_id)?;
        if current.status != CapabilityStatus::Started {
            return Err(StorageError::TerminalReceipt {
                receipt_id: receipt_id.to_owned(),
            });
        }

        // Build dynamic UPDATE based on which fields are provided
        if agent_version_hash.is_some() || behavior_version_hash.is_some() {
            transaction.execute(
                "UPDATE capability_receipts
                 SET status = ?2, result_json = ?3, completed_at = ?4,
                     agent_version_hash = COALESCE(?5, agent_version_hash),
                     behavior_version_hash = COALESCE(?6, behavior_version_hash)
                 WHERE receipt_id = ?1",
                params![
                    receipt_id,
                    enum_string(&status)?,
                    optional_json(&result)?,
                    completed_at,
                    agent_version_hash,
                    behavior_version_hash
                ],
            )?;
        } else {
            transaction.execute(
                "UPDATE capability_receipts
                 SET status = ?2, result_json = ?3, completed_at = ?4
                 WHERE receipt_id = ?1",
                params![
                    receipt_id,
                    enum_string(&status)?,
                    optional_json(&result)?,
                    completed_at
                ],
            )?;
        }
        let receipt = get_capability_receipt_on(&transaction, receipt_id)?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Lists capability receipts for one project in insertion order.
    pub fn list_capability_receipts(&self, project_id: &str) -> Result<Vec<CapabilityReceipt>> {
        let mut statement = self.connection.prepare(
            "SELECT receipt_id, project_id, cycle_id, capability, request_hash,
                    request_json, status, result_json, started_at, completed_at,
                    agent_version_hash, behavior_version_hash
             FROM capability_receipts WHERE project_id = ?1
             ORDER BY started_at ASC",
        )?;
        let rows = statement.query_map([project_id], capability_receipt_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Lists all capability receipts across projects in insertion order.
    pub fn list_all_capability_receipts(&self) -> Result<Vec<CapabilityReceipt>> {
        let mut statement = self.connection.prepare(
            "SELECT receipt_id, project_id, cycle_id, capability, request_hash,
                    request_json, status, result_json, started_at, completed_at,
                    agent_version_hash, behavior_version_hash
             FROM capability_receipts ORDER BY started_at ASC",
        )?;
        let rows = statement.query_map([], capability_receipt_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Loads a capability receipt by identifier.
    pub fn get_capability_receipt(&self, receipt_id: &str) -> Result<CapabilityReceipt> {
        get_capability_receipt_on(&self.connection, receipt_id)
    }

    /// Acquires an absent or expired cycle lease.
    ///
    /// `now_ms` and `expires_at_ms` are supplied by the caller. Replacing an
    /// expired lease increments its fencing token.
    pub fn acquire_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<CycleLease> {
        if now_ms < 0 || expires_at_ms <= now_ms {
            return Err(StorageError::InvalidLease);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = get_cycle_lease_on(&transaction, cycle_id).optional()?;
        let fencing_token = match existing {
            Some(lease) if lease.expires_at_ms > now_ms => {
                return Err(StorageError::LeaseConflict {
                    cycle_id: cycle_id.to_owned(),
                    owner: lease.owner,
                    expires_at_ms: lease.expires_at_ms,
                });
            }
            Some(lease) => lease.fencing_token + 1,
            None => 1,
        };
        transaction.execute(
            "INSERT INTO cycle_leases (
                cycle_id, owner, acquired_at_ms, expires_at_ms, fencing_token
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(cycle_id) DO UPDATE SET
                owner = excluded.owner,
                acquired_at_ms = excluded.acquired_at_ms,
                expires_at_ms = excluded.expires_at_ms,
                fencing_token = excluded.fencing_token",
            params![cycle_id, owner, now_ms, expires_at_ms, fencing_token],
        )?;
        transaction.commit()?;
        Ok(CycleLease {
            cycle_id: cycle_id.to_owned(),
            owner: owner.to_owned(),
            acquired_at_ms: now_ms,
            expires_at_ms,
            fencing_token,
        })
    }

    /// Loads the current cycle lease.
    pub fn get_cycle_lease(&self, cycle_id: &str) -> Result<CycleLease> {
        self.get_cycle_lease_on_optional(cycle_id)?
            .ok_or_else(|| not_found("cycle lease", cycle_id))
    }

    fn get_cycle_lease_on_optional(&self, cycle_id: &str) -> Result<Option<CycleLease>> {
        Ok(get_cycle_lease_on(&self.connection, cycle_id).optional()?)
    }

    /// Extends the expiry of the lease you already hold without changing the
    /// fencing token (reuse / renew semantics).
    pub fn renew_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
        new_expires_at_ms: i64,
    ) -> Result<CycleLease> {
        if now_ms < 0 || new_expires_at_ms <= now_ms {
            return Err(StorageError::InvalidLease);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = match get_cycle_lease_on(&transaction, cycle_id).optional()? {
            Some(lease) => lease,
            None => {
                return Err(StorageError::LeaseNotRenewable {
                    cycle_id: cycle_id.to_owned(),
                    current_owner: String::new(),
                    current_fencing_token: 0,
                });
            }
        };
        if existing.owner != owner || existing.fencing_token != fencing_token {
            return Err(StorageError::LeaseNotRenewable {
                cycle_id: cycle_id.to_owned(),
                current_owner: existing.owner,
                current_fencing_token: existing.fencing_token,
            });
        }
        transaction.execute(
            "UPDATE cycle_leases SET expires_at_ms = ?2 WHERE cycle_id = ?1",
            params![cycle_id, new_expires_at_ms],
        )?;
        transaction.commit()?;
        Ok(CycleLease {
            cycle_id: cycle_id.to_owned(),
            owner: owner.to_owned(),
            acquired_at_ms: existing.acquired_at_ms,
            expires_at_ms: new_expires_at_ms,
            fencing_token,
        })
    }

    /// Releases a cycle lease only when owner and fencing token still match.
    ///
    /// When the delete removes one row, appends a `lease.released` ledger
    /// event in the same transaction. Returns `true` iff the event was
    /// appended.
    #[allow(clippy::too_many_arguments)]
    pub fn release_cycle_lease(
        &mut self,
        project_id: &str,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        actor: &str,
        command_id: &str,
        occurred_at: &str,
    ) -> Result<bool> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changes = transaction.execute(
            "DELETE FROM cycle_leases
             WHERE cycle_id = ?1 AND owner = ?2 AND fencing_token = ?3",
            params![cycle_id, owner, fencing_token],
        )?;
        if changes == 0 {
            transaction.commit()?;
            return Ok(false);
        }
        let payload = serde_json::json!({
            "cycle_id": cycle_id,
            "owner": owner,
            "fencing_token": fencing_token,
            "actor": actor,
        });
        let event = LedgerEventInput {
            event_id: format!("evt-{}", uuid::Uuid::new_v4().hyphenated()),
            project_id: project_id.to_owned(),
            cycle_id: Some(cycle_id.to_owned()),
            frame_id: format!("frame:{command_id}"),
            command_id: command_id.to_owned(),
            actor: actor.to_owned(),
            event_type: "lease.released".to_owned(),
            occurred_at: occurred_at.to_owned(),
            state_before: None,
            state_after: None,
            payload,
        };
        append_event_on(&transaction, &event)?;
        transaction.commit()?;
        Ok(true)
    }

    /// Validates that a gate name conforms to the 1..=128 char limit.
    fn validate_gate_name(gate: &str) -> Result<()> {
        const GATE_MIN: usize = 1;
        const GATE_MAX: usize = 128;
        if !(GATE_MIN..=GATE_MAX).contains(&gate.len()) {
            return Err(StorageError::GateNameInvalid {
                actual: gate.len(),
                min: GATE_MIN,
                max: GATE_MAX,
            });
        }
        Ok(())
    }

    /// Builds a gate receipt identifier from its components.
    ///
    /// The gate name must be 1..=128 characters and the plan_hash must be at
    /// least 23 characters (`sha256:` prefix + 16 hex digits of the actual
    /// hash). Returns `GateNameInvalid` if the gate-length guard fails and
    /// `PlanHashTooShort` if the plan_hash guard fails.
    pub fn build_gate_receipt_id(gate: &str, plan_hash: &str, seq: i64) -> Result<String> {
        Self::validate_gate_name(gate)?;
        const REQUIRED_LEN: usize = 23;
        if plan_hash.len() < REQUIRED_LEN {
            return Err(StorageError::PlanHashTooShort {
                actual: plan_hash.len(),
                required: REQUIRED_LEN,
            });
        }
        Ok(format!("gate-{}-{}-{}", gate, &plan_hash[7..23], seq))
    }

    /// Persists one authorized gate evaluation receipt with atomic seq allocation.
    ///
    /// Computes `seq = COALESCE(MAX(seq)+1, 1)` and builds the `receipt_id`
    /// **inside the same IMMEDIATE transaction** as the `INSERT`, so concurrent
    /// callers are serialized by SQLite's write lock and receive distinct sequences.
    pub fn insert_gate_receipt_next_seq(
        &mut self,
        input: &GateReceiptNextSeqInput,
    ) -> Result<GateReceipt> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let seq: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(seq) + 1, 1) FROM gate_receipts WHERE gate = ?1 AND plan_hash = ?2",
            [&input.gate, &input.plan_hash],
            |row| row.get(0),
        )?;
        let receipt_id = Self::build_gate_receipt_id(&input.gate, &input.plan_hash, seq)?;
        transaction.execute(
            "INSERT INTO gate_receipts (
                receipt_id, project_id, cycle_id, gate, evaluator, transition_id,
                plan_hash, outcome, evidence, actor, command_id, frame_id, evaluated_at, seq
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                receipt_id,
                input.project_id,
                input.cycle_id,
                input.gate,
                input.evaluator,
                input.transition_id,
                input.plan_hash,
                enum_string(&input.outcome)?,
                serde_json::to_string(&input.evidence)?,
                input.actor,
                input.command_id,
                input.frame_id,
                input.evaluated_at,
                seq
            ],
        )?;
        transaction.commit()?;
        Ok(GateReceipt {
            receipt_id,
            project_id: input.project_id.clone(),
            cycle_id: input.cycle_id.clone(),
            gate: input.gate.clone(),
            evaluator: input.evaluator.clone(),
            transition_id: input.transition_id.clone(),
            plan_hash: input.plan_hash.clone(),
            outcome: input.outcome,
            evidence: input.evidence.clone(),
            actor: input.actor.clone(),
            command_id: input.command_id.clone(),
            frame_id: input.frame_id.clone(),
            evaluated_at: input.evaluated_at.clone(),
            seq,
        })
    }

    /// Persists one authorized gate evaluation receipt with a caller-supplied `seq`.
    ///
    /// Prefer [`Storage::insert_gate_receipt_next_seq`] for normal receipt
    /// persistence: it allocates `seq` atomically inside the same IMMEDIATE
    /// transaction as the INSERT, so concurrent callers are serialized by
    /// SQLite's write lock and receive distinct sequences under real thread
    /// contention. This method preserves the caller-supplied `seq` and is
    /// kept only for bootstrap and test compatibility (e.g. legacy v1.9.14
    /// rows from before `seq` existed); it does NOT assign `seq`.
    pub fn insert_gate_receipt(&mut self, input: &GateReceiptInput) -> Result<GateReceipt> {
        Self::validate_gate_name(&input.gate)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO gate_receipts (
                receipt_id, project_id, cycle_id, gate, evaluator, transition_id,
                plan_hash, outcome, evidence, actor, command_id, frame_id, evaluated_at, seq
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                input.receipt_id,
                input.project_id,
                input.cycle_id,
                input.gate,
                input.evaluator,
                input.transition_id,
                input.plan_hash,
                enum_string(&input.outcome)?,
                serde_json::to_string(&input.evidence)?,
                input.actor,
                input.command_id,
                input.frame_id,
                input.evaluated_at,
                input.seq
            ],
        )?;
        transaction.commit()?;
        Ok(GateReceipt {
            receipt_id: input.receipt_id.clone(),
            project_id: input.project_id.clone(),
            cycle_id: input.cycle_id.clone(),
            gate: input.gate.clone(),
            evaluator: input.evaluator.clone(),
            transition_id: input.transition_id.clone(),
            plan_hash: input.plan_hash.clone(),
            outcome: input.outcome,
            evidence: input.evidence.clone(),
            actor: input.actor.clone(),
            command_id: input.command_id.clone(),
            frame_id: input.frame_id.clone(),
            evaluated_at: input.evaluated_at.clone(),
            seq: input.seq,
        })
    }

    /// Loads one gate receipt by identifier.
    pub fn get_gate_receipt(&self, receipt_id: &str) -> Result<GateReceipt> {
        self.connection
            .query_row(
                "SELECT receipt_id, project_id, cycle_id, gate, evaluator, transition_id,
                        plan_hash, outcome, evidence, actor, command_id, frame_id, evaluated_at, seq
                 FROM gate_receipts WHERE receipt_id = ?1",
                [receipt_id],
                gate_receipt_from_row,
            )
            .optional()?
            .ok_or_else(|| not_found("gate receipt", receipt_id))
    }

    /// Lists gate receipts for one cycle in insertion order.
    pub fn list_gate_receipts(&self, cycle_id: &str) -> Result<Vec<GateReceipt>> {
        let mut statement = self.connection.prepare(
            "SELECT receipt_id, project_id, cycle_id, gate, evaluator, transition_id,
                    plan_hash, outcome, evidence, actor, command_id, frame_id, evaluated_at, seq
             FROM gate_receipts WHERE cycle_id = ?1 ORDER BY evaluated_at ASC",
        )?;
        let rows = statement.query_map([cycle_id], gate_receipt_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }
}

pub(crate) fn migrate(connection: &mut Connection) -> Result<()> {
    run_migrations(connection)
}

fn insert_cycle_on(connection: &Connection, cycle: &CycleRecord) -> Result<()> {
    connection.execute(
        "INSERT INTO cycles (
            cycle_id, project_id, workspace_id, status, phase, manifest_json,
            created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            cycle.manifest.cycle_id,
            cycle.manifest.project_id,
            cycle.manifest.workspace_id,
            enum_string(&cycle.manifest.status)?,
            enum_string(&cycle.manifest.phase)?,
            serde_json::to_string(&cycle.manifest)?,
            cycle.created_at,
            cycle.updated_at
        ],
    )?;
    Ok(())
}

fn project_optional_on(connection: &Connection, project_id: &str) -> Result<Option<ProjectRecord>> {
    Ok(connection
        .query_row(
            "SELECT project_id, display_name, remote_url, scope, created_at
             FROM projects WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(ProjectRecord {
                    project_id: row.get(0)?,
                    display_name: row.get(1)?,
                    remote_url: row.get(2)?,
                    scope: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()?)
}

fn workspace_optional_on(
    connection: &Connection,
    workspace_id: &str,
) -> Result<Option<WorkspaceRecord>> {
    Ok(connection
        .query_row(
            "SELECT workspace_id, project_id, canonical_path, created_at
             FROM workspaces WHERE workspace_id = ?1",
            [workspace_id],
            |row| {
                Ok(WorkspaceRecord {
                    workspace_id: row.get(0)?,
                    project_id: row.get(1)?,
                    canonical_path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .optional()?)
}

fn append_event_on(transaction: &Transaction<'_>, input: &LedgerEventInput) -> Result<LedgerEvent> {
    let previous = transaction
        .query_row(
            "SELECT sequence, event_hash FROM ledger_events ORDER BY sequence DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let (sequence, previous_hash) = previous
        .map(|(sequence, hash)| (sequence + 1, Some(hash)))
        .unwrap_or((1, None));
    let event_hash = hash_event(sequence, input, &previous_hash)?;
    transaction.execute(
        "INSERT INTO ledger_events (
            sequence, event_id, project_id, cycle_id, frame_id, command_id,
            actor, event_type, occurred_at, state_before_json,
            state_after_json, payload_json, previous_hash, event_hash
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            sequence,
            input.event_id,
            input.project_id,
            input.cycle_id,
            input.frame_id,
            input.command_id,
            input.actor,
            input.event_type,
            input.occurred_at,
            optional_json(&input.state_before)?,
            optional_json(&input.state_after)?,
            serde_json::to_string(&input.payload)?,
            previous_hash,
            event_hash
        ],
    )?;
    Ok(LedgerEvent {
        sequence,
        event_id: input.event_id.clone(),
        project_id: input.project_id.clone(),
        cycle_id: input.cycle_id.clone(),
        frame_id: input.frame_id.clone(),
        command_id: input.command_id.clone(),
        actor: input.actor.clone(),
        event_type: input.event_type.clone(),
        occurred_at: input.occurred_at.clone(),
        state_before: input.state_before.clone(),
        state_after: input.state_after.clone(),
        payload: input.payload.clone(),
        previous_hash,
        event_hash,
    })
}

#[derive(Serialize)]
struct EventHashMaterial<'a> {
    sequence: i64,
    event_id: &'a str,
    project_id: &'a str,
    cycle_id: &'a Option<String>,
    frame_id: &'a str,
    command_id: &'a str,
    actor: &'a str,
    event_type: &'a str,
    occurred_at: &'a str,
    state_before: &'a Option<Value>,
    state_after: &'a Option<Value>,
    payload: &'a Value,
    previous_hash: &'a Option<String>,
}

#[derive(Serialize)]
struct CapabilityRequestHashMaterial<'a> {
    cycle_id: &'a Option<String>,
    capability: &'a str,
    request: &'a Value,
}

fn hash_event(
    sequence: i64,
    input: &LedgerEventInput,
    previous_hash: &Option<String>,
) -> Result<String> {
    let material = EventHashMaterial {
        sequence,
        event_id: &input.event_id,
        project_id: &input.project_id,
        cycle_id: &input.cycle_id,
        frame_id: &input.frame_id,
        command_id: &input.command_id,
        actor: &input.actor,
        event_type: &input.event_type,
        occurred_at: &input.occurred_at,
        state_before: &input.state_before,
        state_after: &input.state_after,
        payload: &input.payload,
        previous_hash,
    };
    Ok(hash_bytes(&serde_json::to_vec(&material)?))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn hash_capability_request(input: &CapabilityReceiptInput) -> Result<String> {
    let material = CapabilityRequestHashMaterial {
        cycle_id: &input.cycle_id,
        capability: &input.capability,
        request: &input.request,
    };
    Ok(hash_bytes(&serde_json::to_vec(&material)?))
}

fn ensure_event_scope(manifest: &CycleManifest, event: &LedgerEventInput) -> Result<()> {
    if event.project_id != manifest.project_id
        || event.cycle_id.as_deref() != Some(manifest.cycle_id.as_str())
    {
        return Err(StorageError::EventScopeMismatch);
    }
    Ok(())
}

fn enum_string<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value)? {
        Value::String(value) => Ok(value),
        _ => unreachable!("serialized enum must be a string"),
    }
}

fn optional_json(value: &Option<Value>) -> Result<Option<String>> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(StorageError::from)
}

fn parse_optional_json(value: Option<String>) -> rusqlite::Result<Option<Value>> {
    value
        .map(|json| serde_json::from_str(&json).map_err(json_from_sql_error))
        .transpose()
}

fn cycle_from_row(row: &Row<'_>) -> rusqlite::Result<CycleRecord> {
    let manifest_json: String = row.get(0)?;
    Ok(CycleRecord {
        manifest: serde_json::from_str(&manifest_json).map_err(json_from_sql_error)?,
        created_at: row.get(1)?,
        updated_at: row.get(2)?,
    })
}

fn event_from_row(row: &Row<'_>) -> rusqlite::Result<LedgerEvent> {
    let payload_json: String = row.get(11)?;
    Ok(LedgerEvent {
        sequence: row.get(0)?,
        event_id: row.get(1)?,
        project_id: row.get(2)?,
        cycle_id: row.get(3)?,
        frame_id: row.get(4)?,
        command_id: row.get(5)?,
        actor: row.get(6)?,
        event_type: row.get(7)?,
        occurred_at: row.get(8)?,
        state_before: parse_optional_json(row.get(9)?)?,
        state_after: parse_optional_json(row.get(10)?)?,
        payload: serde_json::from_str(&payload_json).map_err(json_from_sql_error)?,
        previous_hash: row.get(12)?,
        event_hash: row.get(13)?,
    })
}

fn artifact_from_row(row: &Row<'_>) -> rusqlite::Result<ArtifactRecord> {
    let metadata_json: String = row.get(8)?;
    Ok(ArtifactRecord {
        artifact_id: row.get(0)?,
        project_id: row.get(1)?,
        cycle_id: row.get(2)?,
        kind: row.get(3)?,
        path: row.get(4)?,
        sha256: row.get(5)?,
        producer: row.get(6)?,
        created_at: row.get(7)?,
        metadata: serde_json::from_str(&metadata_json).map_err(json_from_sql_error)?,
    })
}

fn get_capability_receipt_on(
    connection: &Connection,
    receipt_id: &str,
) -> Result<CapabilityReceipt> {
    connection
        .query_row(
            "SELECT receipt_id, project_id, cycle_id, capability, request_hash,
                    request_json, status, result_json, started_at, completed_at,
                    agent_version_hash, behavior_version_hash
             FROM capability_receipts WHERE receipt_id = ?1",
            [receipt_id],
            capability_receipt_from_row,
        )
        .optional()?
        .ok_or_else(|| not_found("capability receipt", receipt_id))
}

fn capability_receipt_from_row(row: &Row<'_>) -> rusqlite::Result<CapabilityReceipt> {
    let request_json: String = row.get(5)?;
    let status: String = row.get(6)?;
    Ok(CapabilityReceipt {
        receipt_id: row.get(0)?,
        project_id: row.get(1)?,
        cycle_id: row.get(2)?,
        capability: row.get(3)?,
        request_hash: row.get(4)?,
        request: serde_json::from_str(&request_json).map_err(json_from_sql_error)?,
        status: serde_json::from_value(Value::String(status)).map_err(json_from_sql_error)?,
        result: parse_optional_json(row.get(7)?)?,
        started_at: row.get(8)?,
        completed_at: row.get(9)?,
        agent_version_hash: row.get(10)?,
        behavior_version_hash: row.get(11)?,
    })
}
fn get_cycle_lease_on(connection: &Connection, cycle_id: &str) -> rusqlite::Result<CycleLease> {
    connection.query_row(
        "SELECT cycle_id, owner, acquired_at_ms, expires_at_ms, fencing_token
         FROM cycle_leases WHERE cycle_id = ?1",
        [cycle_id],
        |row| {
            Ok(CycleLease {
                cycle_id: row.get(0)?,
                owner: row.get(1)?,
                acquired_at_ms: row.get(2)?,
                expires_at_ms: row.get(3)?,
                fencing_token: row.get(4)?,
            })
        },
    )
}

fn gate_receipt_from_row(row: &Row<'_>) -> rusqlite::Result<GateReceipt> {
    let outcome: String = row.get(7)?;
    let evidence_json: String = row.get(8)?;
    Ok(GateReceipt {
        receipt_id: row.get(0)?,
        project_id: row.get(1)?,
        cycle_id: row.get(2)?,
        gate: row.get(3)?,
        evaluator: row.get(4)?,
        transition_id: row.get(5)?,
        plan_hash: row.get(6)?,
        outcome: serde_json::from_value(Value::String(outcome)).map_err(json_from_sql_error)?,
        evidence: serde_json::from_str(&evidence_json).map_err(json_from_sql_error)?,
        actor: row.get(9)?,
        command_id: row.get(10)?,
        frame_id: row.get(11)?,
        evaluated_at: row.get(12)?,
        seq: row.get(13)?,
    })
}

fn json_from_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn not_found(entity: &'static str, id: &str) -> StorageError {
    StorageError::NotFound {
        entity,
        id: id.to_owned(),
    }
}

fn integrity_error(sequence: i64, reason: &str) -> StorageError {
    StorageError::LedgerIntegrity {
        sequence,
        reason: reason.to_owned(),
    }
}

impl sddk_domain::SddkErrorCode for StorageError {
    fn code(&self) -> &'static str {
        match self {
            Self::Database(..) => "STORAGE_DATABASE",
            Self::Serialization(..) => "STORAGE_SERIALIZATION",
            Self::Io(..) => "STORAGE_IO",
            Self::NotFound { .. } => "STORAGE_NOT_FOUND",
            Self::IdempotencyConflict { .. } => "STORAGE_IDEMPOTENCY_CONFLICT",
            Self::InvalidReceiptBegin => "STORAGE_INVALID_RECEIPT_BEGIN",
            Self::TerminalReceipt { .. } => "STORAGE_TERMINAL_RECEIPT",
            Self::LeaseConflict { .. } => "STORAGE_LEASE_CONFLICT",
            Self::LeaseExpired { .. } => "STORAGE_LEASE_EXPIRED",
            Self::LeaseNotRenewable { .. } => "STORAGE_LEASE_NOT_RENEWABLE",
            Self::InvalidLease => "STORAGE_INVALID_LEASE",
            Self::EventScopeMismatch => "STORAGE_EVENT_SCOPE_MISMATCH",
            Self::RegistrationConflict { .. } => "STORAGE_REGISTRATION_CONFLICT",
            Self::SchemaVersion { .. } => "STORAGE_SCHEMA_VERSION",
            Self::LedgerIntegrity { .. } => "STORAGE_LEDGER_INTEGRITY",
            Self::PlanHashTooShort { .. } => "STORAGE_PLAN_HASH_TOO_SHORT",
            Self::GateNameInvalid { .. } => "STORAGE_GATE_NAME_INVALID",
            Self::CycleProjectMismatch { .. } => "STORAGE_CYCLE_PROJECT_MISMATCH",
        }
    }

    fn recovery(&self) -> String {
        match self {
            Self::Database(..) => "retry after checking the SQLite database integrity".into(),
            Self::Serialization(..) => "fix the malformed JSON value before retrying".into(),
            Self::Io(..) => "check the filesystem path and permissions".into(),
            Self::NotFound { .. } => "create the record or fix the reference".into(),
            Self::IdempotencyConflict { .. } => {
                "use a fresh idempotency key or the original request".into()
            }
            Self::InvalidReceiptBegin => "begin capability receipts in the started status".into(),
            Self::TerminalReceipt { .. } => {
                "do not finalize a receipt that is already terminal".into()
            }
            Self::LeaseConflict { .. } => "wait for the lease to expire or release it first".into(),
            Self::LeaseExpired { .. } => {
                "re-acquire the lease with `acquire`; an expired lease cannot be renewed".into()
            }
            Self::LeaseNotRenewable { .. } => {
                "call `renew` with the exact (owner, fencing_token) returned by the prior \
                 `acquire` or `renew`; release and reacquire if you need a new token"
                    .into()
            }
            Self::InvalidLease => "provide an expiry later than the acquisition time".into(),
            Self::EventScopeMismatch => "match the event scope to the cycle or project".into(),
            Self::RegistrationConflict { .. } => {
                "keep the existing identity data consistent".into()
            }
            Self::SchemaVersion { .. } => {
                "migrate the database to the supported schema version".into()
            }
            Self::LedgerIntegrity { .. } => "restore the ledger from a verified backup".into(),
            Self::PlanHashTooShort { .. } => {
                "supply a plan_hash of at least 23 characters (sha256: prefix + 16 hex digits)"
                    .into()
            }
            Self::GateNameInvalid { .. } => "supply a gate name of 1..=128 characters".into(),
            Self::CycleProjectMismatch {
                cycle_project_id,
                expected_project_id,
                ..
            } => {
                let cp = cycle_project_id.as_str();
                let ep = expected_project_id.as_str();
                format!(
                    "cycle belongs to project {cp}; this workspace adopts project {ep}; \
                     pass a --cycle whose project prefix matches {ep}, or run 'sddk adopt status' to inspect identity"
                )
            }
        }
    }
}

/// Converts `sddk_storage::StorageError` → `sddk_domain::StorageError`.
/// Required so that `impl sddk_domain::Ledger for Storage` methods can use `?`
/// and Rust will apply the conversion automatically.
impl From<StorageError> for sddk_domain::StorageError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::NotFound { entity, id } => {
                sddk_domain::StorageError::NotFound { entity, id }
            }
            StorageError::Database(msg) => sddk_domain::StorageError::Database(msg.to_string()),
            StorageError::LeaseConflict {
                cycle_id, owner, ..
            } => sddk_domain::StorageError::LeaseConflict { cycle_id, owner },
            _ => sddk_domain::StorageError::Other(err.to_string()),
        }
    }
}

impl sddk_domain::Ledger for Storage {
    fn get_cycle(
        &self,
        cycle_id: &str,
    ) -> std::result::Result<CycleRecord, sddk_domain::StorageError> {
        Storage::get_cycle(self, cycle_id).map_err(|e| e.into())
    }

    fn list_cycle_events(
        &self,
        cycle_id: &str,
    ) -> std::result::Result<Vec<LedgerEvent>, sddk_domain::StorageError> {
        Storage::list_cycle_events(self, cycle_id).map_err(|e| e.into())
    }

    fn insert_cycle_with_event(
        &mut self,
        cycle: &CycleRecord,
        event: &LedgerEventInput,
    ) -> std::result::Result<LedgerEvent, sddk_domain::StorageError> {
        Storage::insert_cycle_with_event(self, cycle, event).map_err(|e| e.into())
    }

    fn update_cycle_with_event(
        &mut self,
        manifest: &crate::CycleManifest,
        updated_at: &str,
        event: &LedgerEventInput,
        release_lease_on_phase_change: bool,
    ) -> std::result::Result<LedgerEvent, sddk_domain::StorageError> {
        Storage::update_cycle_with_event(
            self,
            manifest,
            updated_at,
            event,
            release_lease_on_phase_change,
        )
        .map_err(|e| e.into())
    }

    fn acquire_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> std::result::Result<CycleLease, sddk_domain::StorageError> {
        Storage::acquire_cycle_lease(self, cycle_id, owner, now_ms, expires_at_ms)
            .map_err(|e| e.into())
    }

    fn release_cycle_lease(
        &mut self,
        project_id: &str,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        actor: &str,
        command_id: &str,
        occurred_at: &str,
    ) -> std::result::Result<bool, sddk_domain::StorageError> {
        Storage::release_cycle_lease(
            self,
            project_id,
            cycle_id,
            owner,
            fencing_token,
            actor,
            command_id,
            occurred_at,
        )
        .map_err(|e| e.into())
    }

    fn renew_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
        new_expires_at_ms: i64,
    ) -> std::result::Result<CycleLease, sddk_domain::StorageError> {
        Storage::renew_cycle_lease(
            self,
            cycle_id,
            owner,
            fencing_token,
            now_ms,
            new_expires_at_ms,
        )
        .map_err(|e| e.into())
    }

    fn get_cycle_lease(
        &self,
        cycle_id: &str,
    ) -> std::result::Result<CycleLease, sddk_domain::StorageError> {
        Storage::get_cycle_lease(self, cycle_id).map_err(|e| e.into())
    }

    fn verify_cycle_lease(
        &self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
    ) -> std::result::Result<CycleLease, sddk_domain::StorageError> {
        Storage::verify_cycle_lease(self, cycle_id, owner, fencing_token, now_ms)
            .map_err(|e| e.into())
    }

    fn get_gate_receipt(
        &self,
        receipt_id: &str,
    ) -> std::result::Result<GateReceipt, sddk_domain::StorageError> {
        Storage::get_gate_receipt(self, receipt_id).map_err(|e| e.into())
    }

    fn insert_gate_receipt_next_seq(
        &mut self,
        input: &GateReceiptNextSeqInput,
    ) -> std::result::Result<GateReceipt, sddk_domain::StorageError> {
        Storage::insert_gate_receipt_next_seq(self, input).map_err(|e| e.into())
    }

    fn get_project_optional(
        &self,
        project_id: &str,
    ) -> std::result::Result<Option<ProjectRecord>, sddk_domain::StorageError> {
        Storage::get_project_optional(self, project_id).map_err(|e| e.into())
    }

    fn get_workspace_optional(
        &self,
        workspace_id: &str,
    ) -> std::result::Result<Option<WorkspaceRecord>, sddk_domain::StorageError> {
        Storage::get_workspace_optional(self, workspace_id).map_err(|e| e.into())
    }

    fn has_projects(&self) -> std::result::Result<bool, sddk_domain::StorageError> {
        Storage::has_projects(self).map_err(|e| e.into())
    }

    fn register_project_workspace(
        &mut self,
        project: &ProjectRecord,
        workspace: &WorkspaceRecord,
    ) -> std::result::Result<(), sddk_domain::StorageError> {
        Storage::register_project_workspace(self, project, workspace).map_err(|e| e.into())
    }

    fn load_all_ledger_events(
        &self,
    ) -> std::result::Result<Vec<LedgerEvent>, sddk_domain::StorageError> {
        Storage::load_all_ledger_events(self).map_err(|e| e.into())
    }
}

/// `LedgerFactory` for the concrete SQLite-backed [`Storage`].
///
/// This implementation satisfies the [`sddk_domain::LedgerFactory`] port,
/// allowing the CLI composition root to create ledger instances without
/// a direct compile-time dependency on `sddk-storage` in production code
/// that only needs the trait.
///
/// # Example
///
/// ```ignore
/// use sddk_domain::LedgerFactory;
/// use sddk_storage::SqliteLedgerFactory;
///
/// let factory = SqliteLedgerFactory;
/// let ledger = factory.open_ledger(Path::new("/data/ledger.sqlite"))?;
/// ```
pub struct SqliteLedgerFactory;

impl LedgerFactory for SqliteLedgerFactory {
    type Ledger = Storage;

    fn open_ledger(
        &self,
        path: &std::path::Path,
    ) -> std::result::Result<Storage, sddk_domain::StorageError> {
        Storage::open(path).map_err(|e| e.into())
    }

    fn open_in_memory(&self) -> std::result::Result<Storage, sddk_domain::StorageError> {
        Storage::open_in_memory().map_err(|e| e.into())
    }
}

// ── ArtifactStore impl ────────────────────────────────────────────────────────

/// Satisfies the [`sddk_domain::ArtifactStore`] port for the concrete SQLite store.
#[allow(clippy::useless_conversion, clippy::only_used_in_recursion)]
impl ArtifactStore for Storage {
    fn insert_artifact(
        &mut self,
        artifact: &sddk_domain::ArtifactRecord,
    ) -> std::result::Result<(), sddk_domain::StorageError> {
        Storage::insert_artifact(self, artifact).map_err(sddk_domain::StorageError::from)
    }

    fn get_artifact(
        &self,
        artifact_id: &str,
    ) -> std::result::Result<Option<sddk_domain::ArtifactRecord>, sddk_domain::StorageError> {
        match Storage::get_artifact(self, artifact_id) {
            Ok(a) => Ok(Some(a)),
            Err(StorageError::NotFound { .. }) => Ok(None),
            Err(e) => Err(sddk_domain::StorageError::from(e)),
        }
    }

    fn list_project_artifacts(
        &self,
        project_id: &str,
    ) -> std::result::Result<Vec<sddk_domain::ArtifactRecord>, sddk_domain::StorageError> {
        Storage::list_project_artifacts(self, project_id).map_err(sddk_domain::StorageError::from)
    }
}

#[cfg(test)]
mod cycle_project_mismatch_tests {
    use super::*;
    use sddk_domain::SddkErrorCode;

    #[test]
    fn cycle_project_mismatch_code_is_stable() {
        let err = StorageError::CycleProjectMismatch {
            cycle_id: "p-B/foo".into(),
            cycle_project_id: "p-B".into(),
            expected_project_id: "p-A".into(),
        };
        assert_eq!(err.code(), "STORAGE_CYCLE_PROJECT_MISMATCH");
    }

    #[test]
    fn cycle_project_mismatch_recovery_names_both_projects() {
        let err = StorageError::CycleProjectMismatch {
            cycle_id: "p-B/foo".into(),
            cycle_project_id: "p-B".into(),
            expected_project_id: "p-A".into(),
        };
        let recovery = err.recovery();
        assert!(
            recovery.contains("p-B"),
            "recovery should name the cycle's project: {recovery}"
        );
        assert!(
            recovery.contains("p-A"),
            "recovery should name the expected project: {recovery}"
        );
        assert!(
            recovery.contains("sddk adopt status"),
            "recovery should mention 'sddk adopt status': {recovery}"
        );
    }
}
