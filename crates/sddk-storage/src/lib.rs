//! SQLite-backed operational persistence for SDDK.
//!
//! The crate stores project identity, workspaces, cycle snapshots, immutable
//! hash-linked ledger events, artifact metadata, capability receipts, and cycle
//! leases. Callers supply all timestamps; this crate never reads the system clock.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

pub mod backlog_store;
pub mod cas;
pub mod control_plane;
pub mod event_store;
pub mod fork_store;
pub mod graph_store;
mod migrations;
mod models;
pub mod projection_store;
pub mod rebuild;
pub mod schema_guard;
pub mod spine_import;
pub use backlog_store::{BacklogEvent, BacklogStore, SqliteBacklogStore, SqliteBacklogStoreOwned};
pub use cas::FilesystemCas;
pub use control_plane::{ProjectStatusRow, SCHEMA_V1, SqliteControlPlane};
pub use event_store::SqliteEventStore;
pub use fork_store::SqliteForkStore;
pub use graph_store::SqliteGraphStore;
pub use projection_store::SqliteProjectionStore;
pub use rebuild::rebuild;
pub use schema_guard::{
    COMPILED_SCHEMA_VERSION, GuardError, MIN_SUPPORTED_SCHEMA_VERSION, SchemaCompatibility,
    assert_compatible, check_compatibility, classify,
};

use std::path::Path;
use std::time::Duration;

use migrations::{LATEST_SCHEMA_VERSION, run_migrations};
pub use models::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, TransactionBehavior, params};
use sddk_domain::CycleManifest;
use sddk_domain::EventStore;
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
    /// A production write attempted to persist a RUNTIME-DERIVED cycle
    /// status as canonical Cycle truth (WU-C3 cutover, DELTA-CONF-004).
    /// These statuses are decode-only: wait/remediation/recovery detail
    /// belongs to Run/Authority facts (approval events, gate receipts,
    /// transition ledger), not to the `cycles` snapshot.
    #[error(
        "runtime-derived cycle status {status:?} cannot be written to the \
         cycle record (decode-only since the cycle/run lifecycle cutover, \
         DELTA-CONF-004); record the wait/remediation/recovery fact on the \
         ledger instead"
    )]
    RuntimeStatusWriteForbidden {
        /// The offending runtime-derived status.
        status: sddk_domain::CycleStatus,
    },
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
    /// An evidence attachment body is empty.
    #[error("evidence attachment body must be non-empty")]
    EmptyEvidenceBody,
    /// An evidence attachment was written without the universal `relation`
    /// tag (the production authority). The legacy `kind` column is read-compat
    /// only — callers MUST set `relation` from
    /// `resolve_planning_evidence_relation`. Pre-A5-EVIDENCE-ATTACHMENT-MIGRATION-V1
    /// this used to derive silently from the legacy discriminator, which is
    /// now forbidden on the WRITE path.
    #[error(
        "evidence attachment write requires the universal relation tag (set it from resolve_planning_evidence_relation)"
    )]
    MissingEvidenceRelation,
    /// A dependency edge has the same source and target work item.
    #[error("self-loop dependency edge rejected: from_id == to_id ({0})")]
    SelfLoop(String),
}

/// SQLite-backed SDDK persistence.
pub struct Storage {
    connection: Connection,
    cas_root: std::path::PathBuf,
    /// Stable process-local UUID assigned at construction time.
    handle_id: String,
    /// Lazy-computed CAS root identity (SHA-256 of canonical CAS root path).
    cas_root_id_cache: std::sync::OnceLock<String>,
    /// False for read-only handles: the WU-C1.2 canonical redirect must not
    /// open a writable side-channel on a read-only storage.
    writable: bool,
    /// Backing tempdir for isolated ("in-memory") handles (WU-C1.2): keeps
    /// the private `ledger.sqlite` alive for the handle's lifetime and
    /// removes it on drop.
    isolation_dir: Option<tempfile::TempDir>,
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
        let cas_root = crate::cas::FilesystemCas::default_root();
        Self::from_connection(Connection::open(path)?, true, cas_root)
    }

    /// Opens an existing database without creating files or applying migrations.
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let cas_root = crate::cas::FilesystemCas::default_root();
        Self::from_connection(connection, false, cas_root)
    }

    /// Opens an isolated in-memory database and applies all migrations.
    ///
    /// Implementation note (WU-C1.2): the canonical event redirect requires a
    /// file-backed `events_v1` substrate (private-tempfile `ledger.sqlite`),
    /// so "in-memory" is behavioral isolation, not a literal `:memory:`
    /// handle. The backing file is removed when the returned [`TempDir`]
    /// guard is dropped.
    pub fn open_in_memory() -> Result<Self> {
        // Use a temporary directory for CAS in in-memory mode
        let cas_root = std::env::temp_dir().join("sddk_cas_inmemory");
        std::fs::create_dir_all(&cas_root).ok();
        Self::open_isolated()
    }

    /// Opens an in-memory database with a specific CAS root path.
    ///
    /// This is a test-only constructor for cross-storage testing scenarios
    /// where the CAS root path needs to be controlled.
    #[cfg(test)]
    pub fn open_in_memory_with_cas_root(cas_root: std::path::PathBuf) -> Result<Self> {
        let _ = cas_root;
        Self::open_isolated()
    }

    /// Shared backend for the isolated (in-memory) constructors: a private
    /// tempdir hosting a fresh `ledger.sqlite`. Canonical event emission
    /// opens this same file via `SqliteEventStore::open_path`, so the
    /// redirect substrate always exists. The tempdir guard lives inside the
    /// [`Storage`] handle and removes the directory on drop.
    fn open_isolated() -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("ledger.sqlite");
        let mut storage = Self::open(&path)?;
        storage.isolation_dir = Some(dir);
        Ok(storage)
    }

    fn from_connection(
        mut connection: Connection,
        writable: bool,
        cas_root: std::path::PathBuf,
    ) -> Result<Self> {
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
        let handle_id = uuid::Uuid::new_v4().to_string();
        Ok(Self {
            connection,
            cas_root,
            handle_id,
            cas_root_id_cache: std::sync::OnceLock::new(),
            writable,
            isolation_dir: None,
        })
    }

    /// Returns the filesystem path of the backing SQLite database.
    ///
    /// Used by the WU-C1.2 redirect to open the canonical
    /// [`SqliteEventStore`] over the same file. Errors on in-memory handles.
    fn database_path(&self) -> Result<std::path::PathBuf> {
        storage_database_path(&self.connection)
    }

    /// Test-only access to the raw connection (legacy-table seeding in
    /// integration tests). Not part of the public contract.
    #[doc(hidden)]
    pub fn connection_for_tests(&self) -> &Connection {
        &self.connection
    }

    /// Returns the stable CAS root identity string for this storage.
    ///
    /// The CAS root ID is the SHA-256 of the canonical absolute CAS root path.
    /// Two storage handles with the same CAS root path will return the same ID.
    pub fn cas_root_id(&self) -> String {
        self.cas_root_id_cache
            .get_or_init(|| {
                let canonical = self
                    .cas_root
                    .canonicalize()
                    .unwrap_or_else(|_| self.cas_root.clone());
                let path_str = canonical.to_string_lossy().into_owned();
                let digest = sha2::Sha256::digest(path_str.as_bytes());
                format!("{:x}", digest)
            })
            .clone()
    }

    /// Returns the stable handle identifier for this storage instance.
    ///
    /// The handle ID is a process-local UUID assigned at construction time.
    pub fn handle_id(&self) -> &str {
        &self.handle_id
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

    /// Reports whether a cycle row exists in the `cycles` table.
    ///
    /// This check is advisory — it guards the lease INSERT/UPDATE/DELETE paths
    /// to provide a typed `STORAGE_NOT_FOUND` error instead of a generic
    /// `STORAGE_DATABASE` FK violation. The FK constraint on `cycle_leases.cycle_id`
    /// remains the authoritative integrity guarantee.
    pub fn cycle_exists(&self, cycle_id: &str) -> Result<bool> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM cycles WHERE cycle_id = ?1)",
            [cycle_id],
            |row| row.get(0),
        )?)
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
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        self.with_busy_retry(|transaction| {
            let existing_project = project_optional_on(transaction, &project.project_id)?;
            match existing_project {
                Some(existing)
                    if existing.remote_url != project.remote_url
                        || existing.scope != project.scope =>
                {
                    return Err(StorageError::RegistrationConflict {
                        entity: "project",
                        id: project.project_id.clone(),
                    });
                }
                Some(_) => {}
                None => {
                    let has_other: bool = transaction.query_row(
                        "SELECT EXISTS(SELECT 1 FROM projects)",
                        [],
                        |row| row.get(0),
                    )?;
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
            let existing_workspace = workspace_optional_on(transaction, &workspace.workspace_id)?;
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
            Ok(())
        })
    }

    /// Inserts a cycle snapshot without a ledger event.
    ///
    /// Runtime code should normally prefer [`Storage::insert_cycle_with_event`]
    /// so the initial state and causal event are committed atomically.
    pub fn insert_cycle(&self, cycle: &CycleRecord) -> Result<()> {
        // WU-C3 cutover (DELTA-CONF-004): runtime-derived statuses are
        // decode-only; reject writes of derived state as canonical truth.
        if cycle.manifest.status.is_runtime_derived() {
            return Err(StorageError::RuntimeStatusWriteForbidden {
                status: cycle.manifest.status,
            });
        }
        insert_cycle_on(&self.connection, cycle)
    }

    /// Inserts a cycle snapshot and its initial event atomically (WU-C15-5
    /// de-deprecated trait body; the [`Ledger`] port is the write surface).
    ///
    /// The snapshot write stays transactional on the `cycles` table while
    /// the domain event is emitted through the canonical `events_v1` stream
    /// (`cycle:<cycle_id>`).
    fn insert_cycle_with_event(
        &mut self,
        cycle: &CycleRecord,
        event: &LedgerEventInput,
    ) -> Result<LedgerEvent> {
        ensure_event_scope(&cycle.manifest, event)?;
        // WU-C3 cutover (DELTA-CONF-004): runtime-derived statuses are
        // decode-only; reject writes of derived state as canonical truth.
        if cycle.manifest.status.is_runtime_derived() {
            return Err(StorageError::RuntimeStatusWriteForbidden {
                status: cycle.manifest.status,
            });
        }
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        self.with_busy_retry(|transaction| insert_cycle_on(transaction, cycle))?;
        self.emit_canonical_event(event)
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

    /// Replaces a cycle snapshot and appends its causal event (WU-C15-5
    /// de-deprecated trait body; the [`Ledger`] port is the write surface).
    ///
    /// When `release_lease_on_phase_change` is `true`, the method also
    /// deletes the `cycles_lease` row and emits a `lease.released` domain
    /// event. The caller (typically `Engine::apply_transition`) opts in only
    /// when the transition changes the cycle's `phase` and the outcome is
    /// `Succeeded`.
    ///
    /// The snapshot write and the lease release stay on the
    /// `cycles`/`cycle_leases` tables while the domain events are emitted
    /// through the canonical `events_v1` stream (`cycle:<cycle_id>`).
    fn update_cycle_with_event(
        &mut self,
        manifest: &CycleManifest,
        updated_at: &str,
        event: &LedgerEventInput,
        release_lease_on_phase_change: bool,
    ) -> Result<LedgerEvent> {
        ensure_event_scope(manifest, event)?;
        // WU-C3 cutover (DELTA-CONF-004): runtime-derived statuses are
        // decode-only; reject writes of derived state as canonical truth.
        if manifest.status.is_runtime_derived() {
            return Err(StorageError::RuntimeStatusWriteForbidden {
                status: manifest.status,
            });
        }
        // WU-C1.3: emit the causal event BEFORE mutating the snapshot. The
        // canonical append is the guard: a duplicate event_id with divergent
        // content fails here and leaves the `cycles` row untouched, which
        // restores the pre-redirect rollback semantics that
        // `duplicate_event_id_rolls_back_transition_snapshot` pins (the
        // pre-cutover legacy INSERT rejected inside the same transaction).
        // Design note: the snapshot remains a projection —
        // an event that landed without a snapshot is recoverable via
        // `Engine::rebuild_cycle`, but a snapshot moved without its event
        // would be unrecoverable divergence.
        let main_event = self.emit_canonical_event(event)?;
        let changed = self.connection.execute(
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
        if release_lease_on_phase_change {
            let deleted = self
                .connection
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
                    actor_ref: event.actor_ref.clone(),
                    event_type: "lease.released".to_owned(),
                    occurred_at: event.occurred_at.clone(),
                    state_before: None,
                    state_after: None,
                    payload: json!({
                        "cycle_id": manifest.cycle_id,
                        "released_at_ms": updated_at,
                    }),
                    causation_id: Some(main_event.event_id.clone()),
                    correlation_id: event.correlation_id.clone(),
                };
                self.emit_canonical_event(&release_event)?;
            }
        }
        Ok(main_event)
    }

    /// Appends one immutable event to the ledger via the canonical
    /// `events_v1` stream served by [`SqliteEventStore`] (WU-C15-5 pub seed
    /// path; `project:<project_id>` stream when the input carries no cycle).
    ///
    /// Persists an `EventEnvelopeV1` via the canonical path and returns the
    /// equivalent `LedgerEvent` view derived from the canonical receipt.
    ///
    /// Mapping decisions (C1.2 cutover envelope mapping):
    /// - `stream_id` = `cycle:<cycle_id>` when a cycle is set, otherwise
    ///   `project:<project_id>` (global projection replays per-project).
    /// - `sequence` = canonical per-stream sequence (legacy global seq drops).
    /// - `actor_ref` when present, else a `System` actor with `input.actor`.
    /// - `command_id`, `state_before`/`state_after` ride in `metadata`
    ///   (envelope has no first-class columns for them).
    /// - subjects: one `cycle:<id>` (or `project:<id>`) entity, matching the
    ///   legacy root-subject convention.
    pub fn emit_canonical_event(&self, input: &LedgerEventInput) -> Result<LedgerEvent> {
        use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1};
        let stream_id = match &input.cycle_id {
            Some(cycle_id) => format!("cycle:{cycle_id}"),
            None => format!("project:{}", input.project_id),
        };
        let subject_id = input
            .cycle_id
            .clone()
            .unwrap_or_else(|| input.project_id.clone());
        let actor = input.actor_ref.clone().unwrap_or_else(|| ActorRef {
            kind: ActorKind::System,
            id: input.actor.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: None,
        });
        let metadata = json!({
            // Redirect marker: distinguishes C1-redirected events from
            // third-party events_v1 writers (audit trail, WU-C1.2).
            "redirect": "sddk-c1",
            "command_id": input.command_id,
            "state_before": input.state_before,
            "state_after": input.state_after,
        });
        let mut envelope = EventEnvelopeV1 {
            event_id: input.event_id.clone(),
            event_type: input.event_type.clone(),
            schema_version: EventEnvelopeV1::SCHEMA_VERSION,
            stream_id: stream_id.clone(),
            // Placeholder; `append` validates and assigns the canonical
            // per-stream sequence.
            sequence: 0,
            project_id: input.project_id.clone(),
            occurred_at: input.occurred_at.clone(),
            recorded_at: input.occurred_at.clone(),
            actor,
            subjects: vec![EntityRef {
                kind: if input.cycle_id.is_some() {
                    "cycle".to_string()
                } else {
                    "project".to_string()
                },
                id: subject_id,
                version: None,
                content_hash: None,
            }],
            payload: input.payload.clone(),
            evidence_refs: vec![],
            // `compute_content_hash` blanks content_hash/sequence/recorded_at
            // before hashing, so the placeholder values do not affect the
            // digest.
            content_hash: String::new(),
            metadata: Some(metadata),
            causation_id: input.causation_id.clone(),
            correlation_id: Some(
                input
                    .correlation_id
                    .clone()
                    .unwrap_or_else(|| input.frame_id.clone()),
            ),
            cycle_id: input.cycle_id.clone(),
            frame_id: Some(input.frame_id.clone()),
            fork_id: None,
        };
        envelope.content_hash = envelope.compute_content_hash();
        if !self.writable {
            return Err(StorageError::LedgerIntegrity {
                sequence: 0,
                reason: "cannot append canonical events on a read-only storage".to_owned(),
            });
        }
        let mut store = SqliteEventStore::open_path(self.database_path()?)?;
        let receipt = store
            .append(&envelope)
            .map_err(|e| integrity_error(0, &format!("canonical event append failed: {e}")))?;
        Ok(LedgerEvent {
            sequence: receipt.sequence as i64,
            event_id: receipt.event_id,
            project_id: input.project_id.clone(),
            cycle_id: input.cycle_id.clone(),
            frame_id: input.frame_id.clone(),
            command_id: input.command_id.clone(),
            actor: input.actor.clone(),
            actor_ref: input.actor_ref.clone(),
            event_type: input.event_type.clone(),
            occurred_at: input.occurred_at.clone(),
            state_before: input.state_before.clone(),
            state_after: input.state_after.clone(),
            payload: input.payload.clone(),
            causation_id: input.causation_id.clone(),
            correlation_id: input.correlation_id.clone(),
            // Parity contract: `event_hash` mirrors the canonical content
            // hash so downstream receipt comparisons keep working unchanged.
            event_hash: receipt.content_hash,
            // Parity contract: legacy `previous_hash` carried the chain link,
            // so the canonical `chain_hash` rides in the same field.
            previous_hash: Some(receipt.chain_hash),
        })
    }

    /// Lists all ledger events in ascending sequence order (canonical
    /// `events_v1` view only, C1.5 single read authority).
    pub fn list_events(&self) -> Result<Vec<LedgerEvent>> {
        let mut canonical = self.canonical_events()?;
        canonical.sort_by_key(|event| event.sequence);
        Ok(canonical)
    }

    /// Loads every canonical `events_v1` event mapped into the `LedgerEvent`
    /// view (C1-READ-2).
    fn canonical_events(&self) -> Result<Vec<LedgerEvent>> {
        let store = SqliteEventStore::open_path(self.database_path()?)?;
        let mut out = Vec::new();
        for stream in store.list_streams()? {
            let stream_events = store.load_stream(&stream, None, u32::MAX)?;
            out.extend(stream_events.iter().map(canonical_event_to_ledger));
        }
        Ok(out)
    }

    /// Lists ledger events for one cycle in ascending global sequence order.
    pub fn list_cycle_events(&self, cycle_id: &str) -> Result<Vec<LedgerEvent>> {
        let mut events: Vec<LedgerEvent> = self
            .canonical_events()?
            .into_iter()
            .filter(|event| event.cycle_id.as_deref() == Some(cycle_id))
            .collect();
        events.sort_by_key(|event| event.sequence);
        Ok(events)
    }

    /// Lists ledger events sharing one command frame in ascending sequence order.
    pub fn list_frame_events(&self, frame_id: &str) -> Result<Vec<LedgerEvent>> {
        let mut events: Vec<LedgerEvent> = self
            .canonical_events()?
            .into_iter()
            .filter(|event| event.frame_id == frame_id)
            .collect();
        events.sort_by_key(|event| event.sequence);
        Ok(events)
    }

    /// Lists ledger events strictly after `after_sequence`, ascending order,
    /// capped at `limit` rows. M9.5 live-mode streaming foundation
    /// (`sddk ledger watch`).
    ///
    /// C1.5: the cursor domain is the canonical per-stream sequence. Fresh
    /// repositories stream canonical events seamlessly; the pre-cutover
    /// legacy corpus is gone (one-time cursor jump documented in the release
    /// notes).
    pub fn list_events_after(&self, after_sequence: i64, limit: i64) -> Result<Vec<LedgerEvent>> {
        let mut canonical = self.canonical_events()?;
        canonical.retain(|event| event.sequence > after_sequence);
        canonical.sort_by_key(|event| event.sequence);
        Ok(canonical.into_iter().take(limit.max(0) as usize).collect())
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
    ///
    /// C1.5: every canonical `events_v1` stream is verified with the event
    /// store's chain verifier (single read authority).
    pub fn verify_ledger(&self) -> Result<LedgerVerification> {
        let canonical = self.canonical_events()?;
        let store = SqliteEventStore::open_path(self.database_path()?)?;
        for stream in store.list_streams()? {
            if let Err(error) = store.verify_stream_chain(&stream) {
                return Err(integrity_error(
                    -1,
                    &format!("canonical stream {stream}: {error}"),
                ));
            }
            if let Err(error) = store.verify_chain_integrity(&stream) {
                return Err(integrity_error(
                    -1,
                    &format!("canonical stream {stream} chain: {error}"),
                ));
            }
        }
        Ok(LedgerVerification {
            event_count: canonical.len(),
            last_hash: canonical.last().map(|event| event.event_hash.clone()),
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
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        self.with_busy_retry(|transaction| {
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
                return get_capability_receipt_on(transaction, &receipt_id);
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
            let receipt = get_capability_receipt_on(transaction, &input.receipt_id)?;
            Ok(receipt)
        })
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

    /// Finalizes a capability receipt from the started state with optional
    /// version-hash updates.
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
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        self.with_busy_retry(|transaction| {
            let current = get_capability_receipt_on(transaction, receipt_id)?;
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
            let receipt = get_capability_receipt_on(transaction, receipt_id)?;
            Ok(receipt)
        })
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
        // REQ-DEBT017-2: fail-fast with typed error when cycle does not exist
        if !self.cycle_exists(cycle_id)? {
            return Err(not_found("cycle", cycle_id));
        }
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        self.with_busy_retry(|transaction| {
            let existing = get_cycle_lease_on(transaction, cycle_id).optional()?;
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
            Ok(CycleLease {
                cycle_id: cycle_id.to_owned(),
                owner: owner.to_owned(),
                acquired_at_ms: now_ms,
                expires_at_ms,
                fencing_token,
            })
        })
    }

    /// Loads the current cycle lease.
    ///
    /// Returns `NotFound` if the cycle row does not exist in `cycles`
    /// (REQ-DEBT017-5: typed error instead of silent `lease: none`).
    /// Returns `NotFound` with entity `"cycle lease"` if the cycle exists
    /// but has no active lease.
    pub fn get_cycle_lease(&self, cycle_id: &str) -> Result<CycleLease> {
        // REQ-DEBT017-5: cycle must exist in `cycles` table — not found is
        // a typed STORAGE_NOT_FOUND, not a silent `lease: none`
        if !self.cycle_exists(cycle_id)? {
            return Err(not_found("cycle", cycle_id));
        }
        self.get_cycle_lease_on_optional(cycle_id)?
            .ok_or_else(|| not_found("cycle lease", cycle_id))
    }

    fn get_cycle_lease_on_optional(&self, cycle_id: &str) -> Result<Option<CycleLease>> {
        Ok(get_cycle_lease_on(&self.connection, cycle_id).optional()?)
    }

    /// Lists all active (unexpired) leases for cycles belonging to a project.
    /// Used by the cycle inference layer to resolve `--cycle` when zero args
    /// are passed (S1/S3).
    pub fn list_active_cycle_leases_for_project(
        &self,
        project_id: &str,
        now_ms: i64,
    ) -> Result<Vec<CycleLease>> {
        let mut stmt = self.connection.prepare(
            "SELECT cl.cycle_id, cl.owner, cl.acquired_at_ms, cl.expires_at_ms, cl.fencing_token
             FROM cycle_leases cl
             INNER JOIN cycles c ON cl.cycle_id = c.cycle_id
             WHERE c.project_id = ?1 AND cl.expires_at_ms > ?2
             ORDER BY cl.acquired_at_ms DESC",
        )?;
        let leases = stmt.query_map(params![project_id, now_ms], |row| {
            Ok(CycleLease {
                cycle_id: row.get(0)?,
                owner: row.get(1)?,
                acquired_at_ms: row.get(2)?,
                expires_at_ms: row.get(3)?,
                fencing_token: row.get(4)?,
            })
        })?;
        leases
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
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
        // REQ-DEBT017-3: fail-fast with typed error when cycle does not exist
        if !self.cycle_exists(cycle_id)? {
            return Err(not_found("cycle", cycle_id));
        }
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        self.with_busy_retry(|transaction| {
            let existing = match get_cycle_lease_on(transaction, cycle_id).optional()? {
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
            Ok(CycleLease {
                cycle_id: cycle_id.to_owned(),
                owner: owner.to_owned(),
                acquired_at_ms: existing.acquired_at_ms,
                expires_at_ms: new_expires_at_ms,
                fencing_token,
            })
        })
    }

    /// Releases a cycle lease only when owner and fencing token still match,
    /// emitting the `lease.released` audit event on the canonical `events_v1`
    /// stream (`cycle:<cycle_id>`).
    ///
    /// When the delete removes one row, appends a `lease.released` ledger
    /// event in the same transaction. Returns `true` iff the event was
    /// appended.
    #[allow(clippy::too_many_arguments)]
    pub fn release_lease_with_event(
        &mut self,
        project_id: &str,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        actor: &str,
        command_id: &str,
        occurred_at: &str,
    ) -> Result<bool> {
        // REQ-DEBT017-4: fail-fast with typed error when cycle does not exist
        if !self.cycle_exists(cycle_id)? {
            return Err(not_found("cycle", cycle_id));
        }
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        // Returns Some(LedgerEventInput) when the delete removed a row; the
        // event emission happens outside the retry (its own transaction).
        let released = self.with_busy_retry(|transaction| {
            let changes = transaction.execute(
                "DELETE FROM cycle_leases
                 WHERE cycle_id = ?1 AND owner = ?2 AND fencing_token = ?3",
                params![cycle_id, owner, fencing_token],
            )?;
            if changes == 0 {
                return Ok(None);
            }
            Ok(Some(serde_json::json!({
                "cycle_id": cycle_id,
                "owner": owner,
                "fencing_token": fencing_token,
                "actor": actor,
            })))
        })?;
        let payload = match released {
            Some(payload) => payload,
            None => return Ok(false),
        };
        let event = LedgerEventInput {
            event_id: format!("evt-{}", uuid::Uuid::new_v4().hyphenated()),
            project_id: project_id.to_owned(),
            cycle_id: Some(cycle_id.to_owned()),
            frame_id: format!("frame:{command_id}"),
            command_id: command_id.to_owned(),
            actor: actor.to_owned(),
            actor_ref: None,
            event_type: "lease.released".to_owned(),
            occurred_at: occurred_at.to_owned(),
            state_before: None,
            state_after: None,
            payload,
            causation_id: None,
            correlation_id: None,
        };
        // WU-C1.2 redirect (C1-REDIRECT-4): the lease row delete is committed
        // on the kernel tables while the `lease.released` event goes to the
        // canonical `cycle:<cycle_id>` events_v1 stream.
        self.emit_canonical_event(&event)?;
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

    /// Executes a closure inside an IMMEDIATE transaction with bounded retry on
    /// `DatabaseBusy`.
    ///
    /// SQLite's write lock serializes concurrent IMMEDIATE transactions: the
    /// second writer gets `SQLITE_BUSY` (extended code 5). This helper retries
    /// with exponential backoff (100ms base, max 5 attempts) inside the
    /// existing 5s `busy_timeout` budget. All other errors propagate
    /// immediately. Zero new public API — internal helper only.
    fn with_busy_retry<T, F>(&mut self, mut op: F) -> Result<T>
    where
        F: FnMut(&Connection) -> Result<T>,
    {
        const MAX_RETRIES: u32 = 5;
        const BASE_DELAY_MS: u64 = 100;
        let mut last_busy: Option<rusqlite::Error> = None;
        for attempt in 0..=MAX_RETRIES {
            match self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
            {
                Ok(transaction) => match op(&transaction) {
                    Ok(value) => {
                        transaction.commit()?;
                        return Ok(value);
                    }
                    Err(e) => {
                        let _ = transaction.rollback();
                        return Err(e);
                    }
                },
                Err(rusqlite::Error::SqliteFailure(code, ref ext))
                    if code.code == rusqlite::ErrorCode::DatabaseBusy =>
                {
                    last_busy = Some(rusqlite::Error::SqliteFailure(code, ext.clone()));
                    if attempt < MAX_RETRIES {
                        let delay_ms = BASE_DELAY_MS * (1u64 << attempt).min(500);
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        continue;
                    }
                }
                Err(e) => return Err(StorageError::from(e)),
            }
        }
        Err(StorageError::Database(last_busy.take().unwrap_or_else(
            || {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    Some(String::new()),
                )
            },
        )))
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
        self.with_busy_retry(|transaction| {
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
                actor_ref: input.actor_ref.clone(),
                command_id: input.command_id.clone(),
                frame_id: input.frame_id.clone(),
                evaluated_at: input.evaluated_at.clone(),
                seq,
                causation_id: input.causation_id.clone(),
                correlation_id: input.correlation_id.clone(),
            })
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
        // SQLITE-BUSY-R2: bounded retry on DatabaseBusy under concurrent writers.
        self.with_busy_retry(|transaction| {
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
                actor_ref: input.actor_ref.clone(),
                command_id: input.command_id.clone(),
                frame_id: input.frame_id.clone(),
                evaluated_at: input.evaluated_at.clone(),
                seq: input.seq,
                causation_id: input.causation_id.clone(),
                correlation_id: input.correlation_id.clone(),
            })
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

    /// Lists gate receipts for one cycle and transition that match the given plan hash.
    ///
    /// Returns receipts ordered by `evaluated_at ASC`. An empty list means no
    /// fresh gate receipt exists for this state.
    pub fn list_gate_receipts_for(
        &self,
        cycle_id: &str,
        transition_id: &str,
        plan_hash: &str,
    ) -> Result<Vec<GateReceipt>> {
        let mut statement = self.connection.prepare(
            "SELECT receipt_id, project_id, cycle_id, gate, evaluator, transition_id,
                    plan_hash, outcome, evidence, actor, command_id, frame_id, evaluated_at, seq
             FROM gate_receipts
             WHERE cycle_id = ?1 AND transition_id = ?2 AND plan_hash = ?3
             ORDER BY evaluated_at ASC",
        )?;
        let rows =
            statement.query_map([cycle_id, transition_id, plan_hash], gate_receipt_from_row)?;
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

#[derive(Serialize)]
struct CapabilityRequestHashMaterial<'a> {
    cycle_id: &'a Option<String>,
    capability: &'a str,
    request: &'a Value,
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
        // actor_ref / causation_id / correlation_id columns added in MIGRATION_11+; placeholder None for existing rows
        actor_ref: None,
        command_id: row.get(10)?,
        frame_id: row.get(11)?,
        evaluated_at: row.get(12)?,
        seq: row.get(13)?,
        causation_id: None,
        correlation_id: None,
    })
}

fn json_from_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

/// Maps a canonical `EventEnvelopeV1` into the legacy `LedgerEvent` view
/// (inverse of the redirect mapping in
/// [`Storage::emit_canonical_event`]).
///
/// - `sequence`: envelope per-stream sequence (canonical authority).
/// - `previous_hash`: `None` (linkage lives in `chain_hash`; the legacy
///   hash-link semantics do not apply to canonical events).
/// - `command_id` / `state_before` / `state_after`: recovered from
///   `metadata` when present.
/// - `actor`: `actor_ref.id` (the legacy string mirrors the canonical actor).
fn canonical_event_to_ledger(envelope: &sddk_domain::EventEnvelopeV1) -> LedgerEvent {
    let metadata = envelope.metadata.as_ref();
    let command_id = metadata
        .and_then(|m| m.get("command_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    let state_before = metadata
        .and_then(|m| m.get("state_before"))
        .filter(|v| !v.is_null())
        .cloned();
    let state_after = metadata
        .and_then(|m| m.get("state_after"))
        .filter(|v| !v.is_null())
        .cloned();
    LedgerEvent {
        sequence: envelope.sequence as i64,
        event_id: envelope.event_id.clone(),
        project_id: envelope.project_id.clone(),
        cycle_id: envelope.cycle_id.clone(),
        frame_id: envelope.frame_id.clone().unwrap_or_default(),
        command_id,
        actor: envelope.actor.id.clone(),
        actor_ref: Some(envelope.actor.clone()),
        event_type: envelope.event_type.clone(),
        occurred_at: envelope.occurred_at.clone(),
        state_before,
        state_after,
        payload: envelope.payload.clone(),
        causation_id: envelope.causation_id.clone(),
        correlation_id: envelope.correlation_id.clone(),
        event_hash: envelope.content_hash.clone(),
        previous_hash: None,
    }
}

/// Returns the filesystem path of the SQLite database backing a connection.
///
/// Backed by `PRAGMA database_list` so it works for file-backed handles in
/// both read-write and read-only mode. In-memory handles fail closed: they
/// have no canonical-stream substrate to redirect to.
fn storage_database_path(connection: &Connection) -> Result<std::path::PathBuf> {
    let path: String = connection.query_row(
        "SELECT file FROM pragma_database_list() WHERE name = 'main'",
        [],
        |row| row.get(0),
    )?;
    if path.is_empty() {
        return Err(StorageError::LedgerIntegrity {
            sequence: 0,
            reason: "canonical event redirect requires a file-backed database".to_owned(),
        });
    }
    Ok(std::path::PathBuf::from(path))
}

/// Converts `sddk_domain::StorageError` (canonical event store boundary) →
/// `sddk_storage::StorageError` (WU-C1.2 redirect).
impl From<sddk_domain::StorageError> for StorageError {
    fn from(err: sddk_domain::StorageError) -> Self {
        match err {
            sddk_domain::StorageError::NotFound { entity, id } => {
                StorageError::NotFound { entity, id }
            }
            sddk_domain::StorageError::LeaseConflict { cycle_id, owner } => {
                StorageError::LeaseConflict {
                    cycle_id,
                    owner,
                    expires_at_ms: 0,
                }
            }
            other => integrity_error(0, &other.to_string()),
        }
    }
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
            Self::RuntimeStatusWriteForbidden { .. } => "STORAGE_RUNTIME_STATUS_FORBIDDEN",
            Self::RegistrationConflict { .. } => "STORAGE_REGISTRATION_CONFLICT",
            Self::SchemaVersion { .. } => "STORAGE_SCHEMA_VERSION",
            Self::LedgerIntegrity { .. } => "STORAGE_LEDGER_INTEGRITY",
            Self::PlanHashTooShort { .. } => "STORAGE_PLAN_HASH_TOO_SHORT",
            Self::GateNameInvalid { .. } => "STORAGE_GATE_NAME_INVALID",
            Self::CycleProjectMismatch { .. } => "STORAGE_CYCLE_PROJECT_MISMATCH",
            Self::EmptyEvidenceBody => "STORAGE_EMPTY_EVIDENCE_BODY",
            Self::MissingEvidenceRelation => "STORAGE_MISSING_EVIDENCE_RELATION",
            Self::SelfLoop(_) => "STORAGE_SELF_LOOP",
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
            Self::RuntimeStatusWriteForbidden { status } => format!(
                "{status:?} is derived from ledger facts — do not write it to the cycle \
                 record; surface it via the derived cycle runtime summary instead"
            ),
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
            Self::EmptyEvidenceBody => "supply a non-empty evidence body".into(),
            Self::MissingEvidenceRelation => {
                "set EvidenceAttachmentRecord::relation from resolve_planning_evidence_relation \
                 (universal substrate); the legacy kind field is read-compat only and must not \
                 be used to derive relation on write"
                    .into()
            }
            Self::SelfLoop(_) => {
                "remove the self-loop: a dependency edge cannot have the same source and target work item"
                    .into()
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
            StorageError::IdempotencyConflict { key } => {
                // Parse "project_id:run_id:node_id:attempt_seq" back to IdempotencyKey.
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 4 {
                    let attempt_seq = parts[3].parse().unwrap_or(0);
                    sddk_domain::StorageError::IdempotencyConflict {
                        key: sddk_domain::workflow_run::IdempotencyKey {
                            project_id: parts[0].to_string(),
                            run_id: sddk_domain::workflow_ir::RunId(parts[1].to_string()),
                            node_id: sddk_domain::workflow_ir::NodeId(parts[2].to_string()),
                            attempt_seq,
                        },
                    }
                } else {
                    // Fallback for legacy/unexpected format
                    sddk_domain::StorageError::IdempotencyConflict {
                        key: sddk_domain::workflow_run::IdempotencyKey {
                            project_id: key.clone(),
                            run_id: sddk_domain::workflow_ir::RunId(String::new()),
                            node_id: sddk_domain::workflow_ir::NodeId(String::new()),
                            attempt_seq: 0,
                        },
                    }
                }
            }
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

    fn cycle_exists(&self, cycle_id: &str) -> std::result::Result<bool, sddk_domain::StorageError> {
        Storage::cycle_exists(self, cycle_id).map_err(|e| e.into())
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

    fn release_lease_with_event(
        &mut self,
        project_id: &str,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        actor: &str,
        command_id: &str,
        occurred_at: &str,
    ) -> std::result::Result<bool, sddk_domain::StorageError> {
        Storage::release_lease_with_event(
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

    fn list_active_cycle_leases_for_project(
        &self,
        project_id: &str,
        now_ms: i64,
    ) -> std::result::Result<Vec<CycleLease>, sddk_domain::StorageError> {
        Storage::list_active_cycle_leases_for_project(self, project_id, now_ms)
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

    fn list_gate_receipts_for(
        &self,
        cycle_id: &str,
        transition_id: &str,
        plan_hash: &str,
    ) -> std::result::Result<Vec<GateReceipt>, sddk_domain::StorageError> {
        Storage::list_gate_receipts_for(self, cycle_id, transition_id, plan_hash)
            .map_err(|e| e.into())
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

    fn list_events_after(
        &self,
        after_sequence: i64,
        limit: i64,
    ) -> std::result::Result<Vec<LedgerEvent>, sddk_domain::StorageError> {
        Storage::list_events_after(self, after_sequence, limit).map_err(|e| e.into())
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

// ── Planning CRUD ──────────────────────────────────────────────────────────

impl Storage {
    // ── Planning WorkItem CRUD ─────────────────────────────────────────────

    /// Inserts a new WorkItem.
    ///
    /// Fails with `IdempotencyConflict` if the id already exists.
    pub fn insert_work_item(&self, item: &sddk_domain::WorkItemRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO work_items_v1 (
                id, cycle_id, title, description, status,
                actor_ref_kind, actor_ref_id, actor_ref_label,
                created_at, schema_version,
                spine_order, spine_horizon, spine_status, exit_gate
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                item.id,
                item.cycle_id,
                item.title,
                item.description,
                serde_json::to_string(&item.status).unwrap(),
                item.actor_ref_kind,
                item.actor_ref_id,
                item.actor_ref_label,
                item.created_at,
                item.schema_version,
                item.spine_order,
                item.spine_horizon,
                item.spine_status,
                item.exit_gate,
            ],
        )?;
        Ok(())
    }

    /// Loads a WorkItem by id.
    pub fn get_work_item(&self, id: &str) -> Result<Option<sddk_domain::WorkItemRecord>> {
        let result = self.connection.query_row(
            "SELECT id, cycle_id, title, description, status,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    created_at, schema_version,
                    spine_order, spine_horizon, spine_status, exit_gate
             FROM work_items_v1 WHERE id = ?1",
            [id],
            |row| {
                let status_str: String = row.get(4)?;
                let status = serde_json::from_str(&status_str).unwrap();
                Ok(sddk_domain::WorkItemRecord {
                    id: row.get(0)?,
                    cycle_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status,
                    actor_ref_kind: row.get(5)?,
                    actor_ref_id: row.get(6)?,
                    actor_ref_label: row.get(7)?,
                    created_at: row.get(8)?,
                    schema_version: row.get(9)?,
                    spine_order: row.get(10)?,
                    spine_horizon: row.get(11)?,
                    spine_status: row.get(12)?,
                    exit_gate: row.get(13)?,
                })
            },
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    /// Lists all WorkItems for a cycle.
    pub fn list_work_items_by_cycle(
        &self,
        cycle_id: &str,
    ) -> Result<Vec<sddk_domain::WorkItemRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, cycle_id, title, description, status,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    created_at, schema_version,
                    spine_order, spine_horizon, spine_status, exit_gate
             FROM work_items_v1 WHERE cycle_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([cycle_id], |row| {
            let status_str: String = row.get(4)?;
            let status = serde_json::from_str(&status_str).unwrap();
            Ok(sddk_domain::WorkItemRecord {
                id: row.get(0)?,
                cycle_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                status,
                actor_ref_kind: row.get(5)?,
                actor_ref_id: row.get(6)?,
                actor_ref_label: row.get(7)?,
                created_at: row.get(8)?,
                schema_version: row.get(9)?,
                spine_order: row.get(10)?,
                spine_horizon: row.get(11)?,
                spine_status: row.get(12)?,
                exit_gate: row.get(13)?,
            })
        })?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Lists WorkItems for a cycle filtered by status.
    pub fn list_work_items_by_status(
        &self,
        cycle_id: &str,
        status: &sddk_domain::WorkItemStatus,
    ) -> Result<Vec<sddk_domain::WorkItemRecord>> {
        let status_str = serde_json::to_string(status).unwrap();
        let mut stmt = self.connection.prepare(
            "SELECT id, cycle_id, title, description, status,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    created_at, schema_version,
                    spine_order, spine_horizon, spine_status, exit_gate
             FROM work_items_v1 WHERE cycle_id = ?1 AND status = ?2 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![cycle_id, status_str], |row| {
            let status_str: String = row.get(4)?;
            let status = serde_json::from_str(&status_str).unwrap();
            Ok(sddk_domain::WorkItemRecord {
                id: row.get(0)?,
                cycle_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                status,
                actor_ref_kind: row.get(5)?,
                actor_ref_id: row.get(6)?,
                actor_ref_label: row.get(7)?,
                created_at: row.get(8)?,
                schema_version: row.get(9)?,
                spine_order: row.get(10)?,
                spine_horizon: row.get(11)?,
                spine_status: row.get(12)?,
                exit_gate: row.get(13)?,
            })
        })?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Updates a WorkItem's status atomically.
    pub fn update_work_item_status(
        &mut self,
        id: &str,
        new_status: sddk_domain::WorkItemStatus,
        _actor_ref: Option<sddk_domain::ActorRef>,
    ) -> Result<()> {
        let status_str = serde_json::to_string(&new_status).unwrap();
        let rows = self.connection.execute(
            "UPDATE work_items_v1 SET status = ?1 WHERE id = ?2",
            params![status_str, id],
        )?;
        if rows == 0 {
            return Err(StorageError::NotFound {
                entity: "work_item",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Backfills NULL spine metadata columns on an existing work item.
    ///
    /// Used during re-import when a spine row exists but the spine columns are NULL.
    /// Caller must verify no conflict before calling (SpineImportError::ImportConflict
    /// if any column is already populated with different values).
    ///
    /// Used by `import_spine` when the re-import backfill rule applies (PLN-LEDGER-004 AC-PLN4-04).
    pub fn backfill_spine_columns(
        &self,
        work_item_id: &str,
        spine_order: i32,
        spine_horizon: &str,
        spine_status: &str,
        exit_gate: &str,
    ) -> Result<()> {
        self.connection.execute(
            "UPDATE work_items_v1 SET
                spine_order = ?1,
                spine_horizon = ?2,
                spine_status = ?3,
                exit_gate = ?4
             WHERE id = ?5",
            params![
                spine_order,
                spine_horizon,
                spine_status,
                exit_gate,
                work_item_id
            ],
        )?;
        Ok(())
    }

    // ── Planning DependencyEdge CRUD ───────────────────────────────────────

    /// Inserts a DependencyEdge with idempotent composite PK.
    ///
    /// Uses `INSERT OR IGNORE` so duplicate (from_id, to_id, kind) is a no-op.
    /// Rejects self-loops (from_id == to_id) at insert time per AC-PLN2-02 / spec line 90.
    pub fn insert_dependency_edge(&self, edge: &sddk_domain::DependencyEdgeRecord) -> Result<()> {
        if edge.from_id == edge.to_id {
            return Err(StorageError::SelfLoop(edge.from_id.clone()));
        }
        self.connection.execute(
            "INSERT OR IGNORE INTO work_item_dependencies_v1 (
                from_id, to_id, kind,
                actor_ref_kind, actor_ref_id, actor_ref_label,
                schema_version
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                edge.from_id,
                edge.to_id,
                serde_json::to_string(&edge.kind).unwrap(),
                edge.actor_ref_kind,
                edge.actor_ref_id,
                edge.actor_ref_label,
                edge.schema_version,
            ],
        )?;
        Ok(())
    }

    /// Lists outgoing edges from a WorkItem.
    pub fn get_dependency_edges_from(
        &self,
        from_id: &str,
    ) -> Result<Vec<sddk_domain::DependencyEdgeRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT from_id, to_id, kind,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    schema_version
             FROM work_item_dependencies_v1 WHERE from_id = ?1 ORDER BY to_id ASC",
        )?;
        let rows = stmt.query_map([from_id], |row| {
            let kind_str: String = row.get(2)?;
            let kind = serde_json::from_str(&kind_str).unwrap();
            Ok(sddk_domain::DependencyEdgeRecord {
                from_id: row.get(0)?,
                to_id: row.get(1)?,
                kind,
                actor_ref_kind: row.get(3)?,
                actor_ref_id: row.get(4)?,
                actor_ref_label: row.get(5)?,
                schema_version: row.get(6)?,
            })
        })?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Lists incoming edges to a WorkItem.
    pub fn get_dependency_edges_to(
        &self,
        to_id: &str,
    ) -> Result<Vec<sddk_domain::DependencyEdgeRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT from_id, to_id, kind,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    schema_version
             FROM work_item_dependencies_v1 WHERE to_id = ?1 ORDER BY from_id ASC",
        )?;
        let rows = stmt.query_map([to_id], |row| {
            let kind_str: String = row.get(2)?;
            let kind = serde_json::from_str(&kind_str).unwrap();
            Ok(sddk_domain::DependencyEdgeRecord {
                from_id: row.get(0)?,
                to_id: row.get(1)?,
                kind,
                actor_ref_kind: row.get(3)?,
                actor_ref_id: row.get(4)?,
                actor_ref_label: row.get(5)?,
                schema_version: row.get(6)?,
            })
        })?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Lists all DependencyEdges for a cycle.
    pub fn list_dependency_edges_by_cycle(
        &self,
        cycle_id: &str,
    ) -> Result<Vec<sddk_domain::DependencyEdgeRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT d.from_id, d.to_id, d.kind,
                    d.actor_ref_kind, d.actor_ref_id, d.actor_ref_label,
                    d.schema_version
             FROM work_item_dependencies_v1 d
             JOIN work_items_v1 w ON d.from_id = w.id
             WHERE w.cycle_id = ?1
             ORDER BY d.from_id ASC, d.to_id ASC",
        )?;
        let rows = stmt.query_map([cycle_id], |row| {
            let kind_str: String = row.get(2)?;
            let kind = serde_json::from_str(&kind_str).unwrap();
            Ok(sddk_domain::DependencyEdgeRecord {
                from_id: row.get(0)?,
                to_id: row.get(1)?,
                kind,
                actor_ref_kind: row.get(3)?,
                actor_ref_id: row.get(4)?,
                actor_ref_label: row.get(5)?,
                schema_version: row.get(6)?,
            })
        })?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    // ── Planning EvidenceAttachment CRUD ─────────────────────────────────

    /// Inserts an EvidenceAttachment: writes body to CAS, stores metadata in SQL.
    ///
    /// **CAS-ORACLE contract (INV-1):** UNIQUE-id collision MUST NOT leave an
    /// orphan CAS file. This is guaranteed by reordering: SQL INSERT executes
    /// first. CAS is written only after INSERT succeeds. If INSERT fails
    /// (UNIQUE or other), CAS is never touched — orphan class is impossible
    /// by construction.
    ///
    /// Fails with `StorageError::EmptyEvidenceBody` if body is empty.
    /// Fails with `StorageError::MissingEvidenceRelation` if the universal
    /// `relation` tag is not set.
    ///
    /// Post-A5-EVIDENCE-ATTACHMENT-MIGRATION-V1 §M2: the write path is
    /// FAIL-CLOSED on the universal substrate. The legacy `kind` column is
    /// read-compat only and is NEVER used to derive `relation` on write
    /// (that would be a silent default to the legacy authority, forbidden
    /// by ADR-0100 + SPEC-CONF-003). Producers MUST build the record via
    /// `EvidenceAttachmentRecord::from_universal_relation` (which sets
    /// `relation` from `resolve_planning_evidence_relation`).
    ///
    /// Reads (legacy NULL-relation rows from pre-MIGRATION_19 storage)
    /// still derive `relation` from `kind` — the asymmetry is intentional:
    /// legacy compatibility belongs to READ, not WRITE.
    pub fn insert_evidence_attachment(
        &mut self,
        attachment: &sddk_domain::EvidenceAttachmentRecord,
        body: &[u8],
    ) -> Result<()> {
        if body.is_empty() {
            return Err(StorageError::EmptyEvidenceBody);
        }
        // Write-path fail-closed: the universal `relation` tag is the
        // production authority. Refuse to derive it from the legacy
        // discriminator — that would silently re-promote the legacy
        // authority to productive status on writes.
        let relation = attachment
            .relation
            .clone()
            .ok_or(StorageError::MissingEvidenceRelation)?;

        // Compute CAS hash before any filesystem or database write.
        // This lets us try the INSERT first; CAS is written only after
        // INSERT succeeds — making the orphan class impossible by construction.
        use sha2::{Digest, Sha256};
        let hash_digest = Sha256::digest(body);
        let hash_hex = format!("{:x}", hash_digest);
        let cas_hash = format!("sha256:{}", hash_hex);

        // Try INSERT first. On UNIQUE collision the error propagates
        // immediately and CAS is never written.
        self.connection.execute(
            "INSERT INTO evidence_attachments_v1 (
                id, work_item_id, kind, body_ref, relation,
                actor_ref_kind, actor_ref_id, actor_ref_label,
                schema_version
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                attachment.id,
                attachment.work_item_id,
                serde_json::to_string(&attachment.kind).unwrap(),
                cas_hash,
                relation,
                attachment.actor_ref_kind,
                attachment.actor_ref_id,
                attachment.actor_ref_label,
                attachment.schema_version,
            ],
        )?;

        // INSERT succeeded. Now write the body to CAS. This is idempotent
        // (cas_put skips the write if the file already exists), so concurrent
        // calls with the same body never create duplicate CAS files.
        self.cas_put(body)?;
        Ok(())
    }

    /// Loads an EvidenceAttachment and its body from CAS.
    ///
    /// WU-C2: decodes the universal `relation` tag; legacy rows predating
    /// MIGRATION_19 (NULL relation) derive it from the legacy `kind` via the
    /// ADR-0100 table (read-compat decoder path).
    pub fn get_evidence_attachment(
        &self,
        id: &str,
    ) -> Result<Option<(sddk_domain::EvidenceAttachmentRecord, Vec<u8>)>> {
        let result = self.connection.query_row(
            "SELECT id, work_item_id, kind, body_ref, relation,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    schema_version
             FROM evidence_attachments_v1 WHERE id = ?1",
            [id],
            |row| {
                let kind_str: String = row.get(2)?;
                let relation_col: Option<String> = row.get(4)?;
                let record = sddk_domain::EvidenceAttachmentRecord::from_legacy_kind_tag(
                    row.get(0)?,
                    row.get(1)?,
                    kind_str.trim_matches('"'),
                    row.get(3)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                )
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                let record = sddk_domain::EvidenceAttachmentRecord {
                    relation: relation_col.or(record.relation),
                    ..record
                };
                Ok((
                    record,
                    row.get::<_, String>(3)?, // body_ref for CAS lookup
                ))
            },
        );
        match result {
            Ok((rec, body_ref)) => {
                // Load body from CAS
                let body = self.cas_get(&body_ref)?;
                Ok(Some((rec, body)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    /// Lists evidence attachments for a WorkItem.
    ///
    /// WU-C2: decodes the universal `relation` tag; legacy rows predating
    /// MIGRATION_19 derive it from the legacy `kind` (read-compat decoder).
    pub fn list_evidence_attachments_by_work_item(
        &self,
        work_item_id: &str,
    ) -> Result<Vec<sddk_domain::EvidenceAttachmentRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, work_item_id, kind, body_ref, relation,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    schema_version
             FROM evidence_attachments_v1 WHERE work_item_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([work_item_id], |row| {
            let kind_str: String = row.get(2)?;
            let relation_col: Option<String> = row.get(4)?;
            let record = sddk_domain::EvidenceAttachmentRecord::from_legacy_kind_tag(
                row.get(0)?,
                row.get(1)?,
                kind_str.trim_matches('"'),
                row.get(3)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            )
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(sddk_domain::EvidenceAttachmentRecord {
                relation: relation_col.or(record.relation),
                ..record
            })
        })?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    // ── Planning DecisionRecord CRUD ──────────────────────────────────────

    /// Inserts a DecisionRecord.
    ///
    /// The domain layer should have already validated that rationale is non-empty.
    pub fn insert_decision_record(&self, record: &sddk_domain::DecisionRecordRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO decision_records_v1 (
                id, work_item_id, kind, rationale,
                actor_ref_kind, actor_ref_id, actor_ref_label,
                schema_version
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.work_item_id,
                serde_json::to_string(&record.kind).unwrap(),
                record.rationale,
                record.actor_ref_kind,
                record.actor_ref_id,
                record.actor_ref_label,
                record.schema_version,
            ],
        )?;
        Ok(())
    }

    /// Loads a DecisionRecord by id.
    pub fn get_decision_record(
        &self,
        id: &str,
    ) -> Result<Option<sddk_domain::DecisionRecordRecord>> {
        let result = self.connection.query_row(
            "SELECT id, work_item_id, kind, rationale,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    schema_version
             FROM decision_records_v1 WHERE id = ?1",
            [id],
            |row| {
                let kind_str: String = row.get(2)?;
                let kind = serde_json::from_str(&kind_str).unwrap();
                Ok(sddk_domain::DecisionRecordRecord {
                    id: row.get(0)?,
                    work_item_id: row.get(1)?,
                    kind,
                    rationale: row.get(3)?,
                    actor_ref_kind: row.get(4)?,
                    actor_ref_id: row.get(5)?,
                    actor_ref_label: row.get(6)?,
                    schema_version: row.get(7)?,
                })
            },
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    /// Lists decision records for a WorkItem.
    pub fn list_decision_records_by_work_item(
        &self,
        work_item_id: &str,
    ) -> Result<Vec<sddk_domain::DecisionRecordRecord>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, work_item_id, kind, rationale,
                    actor_ref_kind, actor_ref_id, actor_ref_label,
                    schema_version
             FROM decision_records_v1 WHERE work_item_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([work_item_id], |row| {
            let kind_str: String = row.get(2)?;
            let kind = serde_json::from_str(&kind_str).unwrap();
            Ok(sddk_domain::DecisionRecordRecord {
                id: row.get(0)?,
                work_item_id: row.get(1)?,
                kind,
                rationale: row.get(3)?,
                actor_ref_kind: row.get(4)?,
                actor_ref_id: row.get(5)?,
                actor_ref_label: row.get(6)?,
                schema_version: row.get(7)?,
            })
        })?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    // ── Planning Provenance Chain ─────────────────────────────────────────

    /// Builds a provenance chain for a cycle from current storage state.
    ///
    /// Aggregates all WorkItems, edges, evidence, and decisions for the cycle.
    pub fn build_provenance_chain(
        &self,
        cycle_id: &str,
    ) -> Result<sddk_domain::PlanningProvenanceChainV1> {
        self.collect_provenance_chain(cycle_id, None)
    }

    /// Builds a provenance chain for a cycle with schema version 2 producer metadata.
    ///
    /// Same as `build_provenance_chain`, but stamps the chain with
    /// `producer_cas_root_id = self.cas_root_id()` and `schema_version = 2`.
    ///
    /// Used for cross-storage verification scenarios.
    pub fn build_provenance_chain_v2(
        &self,
        cycle_id: &str,
    ) -> Result<sddk_domain::PlanningProvenanceChainV1> {
        self.collect_provenance_chain(cycle_id, Some(self.cas_root_id()))
    }

    /// Private helper: collects work items, evidence refs, and decision refs for a cycle,
    /// then constructs a `PlanningProvenanceChainV1` using `new` (schema v1) when
    /// `producer_cas_root_id` is `None`, or `new_v2` (schema v2) when `Some`.
    fn collect_provenance_chain(
        &self,
        cycle_id: &str,
        producer_cas_root_id: Option<String>,
    ) -> Result<sddk_domain::PlanningProvenanceChainV1> {
        let work_items = self.list_work_items_by_cycle(cycle_id)?;
        let work_item_ids: Vec<_> = work_items.iter().map(|w| w.id.clone()).collect();

        let _edges = self.list_dependency_edges_by_cycle(cycle_id)?;

        // Collect evidence refs from all work items in cycle
        let mut all_evidence_refs: Vec<sddk_domain::CasHash> = Vec::new();
        for wi in &work_item_ids {
            let evidence = self.list_evidence_attachments_by_work_item(wi)?;
            all_evidence_refs.extend(evidence.iter().map(|e| e.body_ref.clone()));
        }

        // Collect decision refs from all work items in cycle
        let mut all_decision_refs: Vec<_> = Vec::new();
        for wi in &work_item_ids {
            let decisions = self.list_decision_records_by_work_item(wi)?;
            all_decision_refs.extend(decisions.iter().map(|d| d.id.clone()));
        }

        match producer_cas_root_id {
            Some(cas_root_id) => Ok(sddk_domain::PlanningProvenanceChainV1::new_v2(
                cycle_id.to_string(),
                work_item_ids,
                all_evidence_refs,
                all_decision_refs,
                cas_root_id,
            )),
            None => Ok(sddk_domain::PlanningProvenanceChainV1::new(
                cycle_id.to_string(),
                work_item_ids,
                all_evidence_refs,
                all_decision_refs,
            )),
        }
    }

    // ── CAS helpers ─────────────────────────────────────────────────────

    fn cas_put(&mut self, body: &[u8]) -> Result<sddk_domain::CasHash> {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(body);
        let hash_hex = format!("{:x}", hash);
        let cas_hash = format!("sha256:{}", hash_hex);

        // Create directory structure
        let first = &hash_hex[0..2];
        let second = &hash_hex[2..4];
        let path = self.cas_root.join(first).join(second).join(&hash_hex);
        if !path.exists() {
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, body)?;
        }
        Ok(cas_hash)
    }

    fn cas_get(&self, cas_hash: &str) -> Result<Vec<u8>> {
        let hash_hex = cas_hash.trim_start_matches("sha256:");
        let first = &hash_hex[0..2];
        let second = &hash_hex[2..4];
        let path = self.cas_root.join(first).join(second).join(hash_hex);
        std::fs::read(&path).map_err(StorageError::from)
    }
}

// ── PlanningGraphRead port implementation ─────────────────────────────────────

/// Implements the domain's `PlanningGraphRead` port for the concrete `Storage` type.
impl sddk_domain::PlanningGraphRead for Storage {
    fn list_work_items_by_cycle(
        &self,
        cycle_id: &str,
    ) -> std::result::Result<Vec<sddk_domain::WorkItemRecord>, sddk_domain::StorageError> {
        Storage::list_work_items_by_cycle(self, cycle_id).map_err(sddk_domain::StorageError::from)
    }

    fn list_dependency_edges_by_cycle(
        &self,
        cycle_id: &str,
    ) -> std::result::Result<Vec<sddk_domain::DependencyEdgeRecord>, sddk_domain::StorageError>
    {
        Storage::list_dependency_edges_by_cycle(self, cycle_id)
            .map_err(sddk_domain::StorageError::from)
    }

    fn list_evidence_attachments_by_work_item(
        &self,
        work_item_id: &str,
    ) -> std::result::Result<Vec<sddk_domain::EvidenceAttachmentRecord>, sddk_domain::StorageError>
    {
        Storage::list_evidence_attachments_by_work_item(self, work_item_id)
            .map_err(sddk_domain::StorageError::from)
    }

    fn list_decision_records_by_work_item(
        &self,
        work_item_id: &str,
    ) -> std::result::Result<Vec<sddk_domain::DecisionRecordRecord>, sddk_domain::StorageError>
    {
        Storage::list_decision_records_by_work_item(self, work_item_id)
            .map_err(sddk_domain::StorageError::from)
    }

    fn cas_root_id(&self) -> String {
        Storage::cas_root_id(self)
    }

    fn handle_id(&self) -> String {
        Storage::handle_id(self).to_string()
    }
}

// ── RoadmapGraphRead port implementation ─────────────────────────────────────

/// Implements the domain's `RoadmapGraphRead` port for the concrete `Storage` type.
impl sddk_domain::planning::roadmap_read::RoadmapGraphRead for Storage {
    fn list_work_items_roadmap(
        &self,
    ) -> std::result::Result<
        Vec<sddk_domain::planning::projections::WorkItemSnapshot>,
        sddk_domain::StorageError,
    > {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, cycle_id, title, description, status,
                        spine_order, spine_horizon, spine_status, exit_gate
                 FROM work_items_v1 ORDER BY spine_order ASC",
            )
            .map_err(StorageError::from)?;
        let mapped_rows = stmt.query_map(
            [],
            |row| -> std::result::Result<
                sddk_domain::planning::projections::WorkItemSnapshot,
                rusqlite::Error,
            > {
                let status_str: String = row.get(4)?;
                let status = serde_json::from_str(&status_str).unwrap();
                Ok(sddk_domain::planning::projections::WorkItemSnapshot {
                    id: row.get(0)?,
                    cycle_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status,
                    spine_order: row.get(5)?,
                    spine_horizon: row.get(6)?,
                    spine_status: row.get(7)?,
                    exit_gate: row.get(8)?,
                    blocks: Vec::new(),
                })
            },
        );
        let items: std::result::Result<Vec<_>, StorageError> = mapped_rows
            .map_err(StorageError::from)?
            .map(|row| row.map_err(StorageError::from))
            .collect();
        items.map_err(sddk_domain::StorageError::from)
    }

    fn list_dependency_edges_roadmap(
        &self,
    ) -> std::result::Result<
        Vec<sddk_domain::planning::projections::DependencyEdgeSnapshot>,
        sddk_domain::StorageError,
    > {
        let mut stmt = self
            .connection
            .prepare("SELECT from_id, to_id, kind FROM work_item_dependencies_v1")
            .map_err(StorageError::from)?;
        let mapped_rows = stmt.query_map(
            [],
            |row| -> std::result::Result<sddk_domain::planning::projections::DependencyEdgeSnapshot, rusqlite::Error> {
                let kind_str: String = row.get(2)?;
                let kind = match kind_str.as_str() {
                    "blocks" => {
                        sddk_domain::planning::projections::DependencyEdgeKindSnapshot::Blocks
                    }
                    "blocks_on_closure" => {
                        sddk_domain::planning::projections::DependencyEdgeKindSnapshot::BlocksOnClosure
                    }
                    _ => sddk_domain::planning::projections::DependencyEdgeKindSnapshot::Blocks,
                };
                Ok(sddk_domain::planning::projections::DependencyEdgeSnapshot {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    kind,
                })
            },
        );
        let items: std::result::Result<Vec<_>, StorageError> = mapped_rows
            .map_err(StorageError::from)?
            .map(|row| row.map_err(StorageError::from))
            .collect();
        items.map_err(sddk_domain::StorageError::from)
    }

    fn list_incoming_edges_for(
        &self,
        work_item_id: &str,
    ) -> std::result::Result<
        Vec<sddk_domain::planning::projections::DependencyEdgeSnapshot>,
        sddk_domain::StorageError,
    > {
        let mut stmt = self
            .connection
            .prepare("SELECT from_id, to_id, kind FROM work_item_dependencies_v1 WHERE to_id = ?1")
            .map_err(StorageError::from)?;
        let mapped_rows = stmt.query_map(
            [work_item_id],
            |row| -> std::result::Result<sddk_domain::planning::projections::DependencyEdgeSnapshot, rusqlite::Error> {
                let kind_str: String = row.get(2)?;
                let kind = match kind_str.as_str() {
                    "blocks" => {
                        sddk_domain::planning::projections::DependencyEdgeKindSnapshot::Blocks
                    }
                    "blocks_on_closure" => {
                        sddk_domain::planning::projections::DependencyEdgeKindSnapshot::BlocksOnClosure
                    }
                    _ => sddk_domain::planning::projections::DependencyEdgeKindSnapshot::Blocks,
                };
                Ok(sddk_domain::planning::projections::DependencyEdgeSnapshot {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    kind,
                })
            },
        );
        let items: std::result::Result<Vec<_>, StorageError> = mapped_rows
            .map_err(StorageError::from)?
            .map(|row| row.map_err(StorageError::from))
            .collect();
        items.map_err(sddk_domain::StorageError::from)
    }

    fn get_work_item_with_spine_metadata(
        &self,
        work_item_id: &str,
    ) -> std::result::Result<
        Option<sddk_domain::planning::projections::WorkItemSnapshot>,
        sddk_domain::StorageError,
    > {
        let result = self.connection.query_row(
            "SELECT id, cycle_id, title, description, status,
                    spine_order, spine_horizon, spine_status, exit_gate
             FROM work_items_v1 WHERE id = ?1",
            [work_item_id],
            |row| {
                let status_str: String = row.get(4)?;
                let status = serde_json::from_str(&status_str).unwrap();
                Ok(sddk_domain::planning::projections::WorkItemSnapshot {
                    id: row.get(0)?,
                    cycle_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status,
                    spine_order: row.get(5)?,
                    spine_horizon: row.get(6)?,
                    spine_status: row.get(7)?,
                    exit_gate: row.get(8)?,
                    blocks: Vec::new(),
                })
            },
        );
        match result {
            Ok(wi) => Ok(Some(wi)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e).into()),
        }
    }

    fn is_ledger_imported(&self) -> std::result::Result<bool, sddk_domain::StorageError> {
        let count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM work_items_v1 WHERE spine_status IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .map_err(StorageError::from)?;
        Ok(count > 0)
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

#[cfg(test)]
mod list_events_after_tests {
    use super::*;
    use serde_json::json;

    /// Helper to build a minimal `LedgerEventInput` for tests.
    fn make_input(project_id: &str, event_type: &str) -> LedgerEventInput {
        LedgerEventInput {
            event_id: format!("e-{event_type}"),
            project_id: project_id.to_string(),
            cycle_id: None,
            frame_id: "f-1".to_string(),
            command_id: "cmd-1".to_string(),
            actor: "test".to_string(),
            actor_ref: None,
            event_type: event_type.to_string(),
            occurred_at: "2026-09-11T10:00:00Z".to_string(),
            state_before: None,
            state_after: None,
            payload: json!({}),
            causation_id: None,
            correlation_id: None,
        }
    }

    /// `list_events_after(after, limit)` returns rows with sequence > after
    /// ordered ascending, truncated to `limit`.
    #[test]
    fn list_events_after_returns_strictly_newer_events_ascending() {
        // Storage::open takes a SQLite *file* path (not a directory) and
        // creates parent directories if needed. Use a file path inside a
        // fresh tempdir.
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ledger.sqlite");
        let store = Storage::open(&db_path).unwrap();

        // Project row needed for FK.
        store
            .insert_project(&sddk_domain::ProjectRecord {
                project_id: "p-test".into(),
                display_name: "test".into(),
                remote_url: None,
                scope: ".".into(),
                created_at: "2026-09-11T10:00:00Z".into(),
            })
            .unwrap();

        let mut last_seq = 0;
        for event_type in ["a", "b", "c", "d"] {
            let appended = store
                .emit_canonical_event(&make_input("p-test", event_type))
                .unwrap();
            last_seq = appended.sequence;
        }
        assert_eq!(last_seq, 4);

        // after_sequence = 1 → expect events 2, 3, 4.
        let events = Storage::list_events_after(&store, 1, 100).unwrap();
        let sequences: Vec<i64> = events.iter().map(|e| e.sequence).collect();
        assert_eq!(sequences, vec![2, 3, 4]);

        // limit = 2 → expect only the first 2 of the strictly-newer set.
        let limited = Storage::list_events_after(&store, 1, 2).unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].sequence, 2);
        assert_eq!(limited[1].sequence, 3);

        // after_sequence past the head → empty result.
        let none = Storage::list_events_after(&store, 99, 100).unwrap();
        assert!(none.is_empty());

        // after_sequence = 0 → all events from the start.
        let all = Storage::list_events_after(&store, 0, 100).unwrap();
        assert_eq!(all.len(), 4);
    }
}

// ── C3c (session-11) — T23+T24 Storage Security canarios ───────────────────

#[cfg(test)]
mod capability_receipt_security_tests {
    //! T23 — fail-closed contracts on `begin_capability_receipt` and
    //! `finalize_capability_receipt`. These guards are security-critical
    //! because they are the only thing standing between a caller and
    //! rewriting a capability execution receipt's lifecycle state.

    use super::*;
    use sddk_domain::models::capability::{CapabilityReceiptInput, CapabilityStatus};
    use serde_json::json;

    fn base_input() -> CapabilityReceiptInput {
        CapabilityReceiptInput {
            receipt_id: "rec-001".to_string(),
            project_id: "p-c3c".to_string(),
            cycle_id: Some("c-cap".to_string()),
            capability: "test.capability".to_string(),
            idempotency_key: "key-001".to_string(),
            request: json!({ "hello": "world" }),
            status: CapabilityStatus::Started,
            result: None,
            started_at: "2026-09-22T00:00:00Z".to_string(),
            completed_at: None,
            agent_version_hash: None,
            behavior_version_hash: None,
        }
    }

    /// `capability_receipts.cycle_id` has a composite FK on `(project_id,
    /// cycle_id)` → `cycles(project_id, cycle_id)`. Seed the project +
    /// workspace + cycle so all FK constraints are satisfied before testing
    /// the receipt guards.
    fn storage_with_capability_cycle() -> Storage {
        use sddk_domain::cycle::{CycleManifest, CyclePath, CycleStatus, Phase};
        use sddk_domain::models::identity::{CycleRecord, ProjectRecord, WorkspaceRecord};
        use std::collections::HashMap;

        let store = Storage::open_in_memory().expect("open");
        store
            .insert_project(&ProjectRecord {
                project_id: "p-c3c".to_string(),
                display_name: "p-c3c".to_string(),
                remote_url: None,
                scope: ".".to_string(),
                created_at: "2026-09-22T00:00:00Z".to_string(),
            })
            .expect("project");
        store
            .insert_workspace(&WorkspaceRecord {
                workspace_id: "w-c3c".to_string(),
                project_id: "p-c3c".to_string(),
                canonical_path: "/work/c3c".to_string(),
                created_at: "2026-09-22T00:00:00Z".to_string(),
            })
            .expect("workspace");
        store
            .insert_cycle(&CycleRecord {
                manifest: CycleManifest {
                    schema_version: 1,
                    project_id: "p-c3c".to_string(),
                    workspace_id: "w-c3c".to_string(),
                    cycle_id: "c-cap".to_string(),
                    display_name: "c-cap".to_string(),
                    status: CycleStatus::Open,
                    phase: Phase::Build,
                    path: CyclePath::ALite,
                    branch: "feat/cap".to_string(),
                    base: "abc".to_string(),
                    head: None,
                    artifacts: HashMap::new(),
                    release: None,
                    delivery_kind: None,
                    remediation_round: 0,
                    remote_url: None,
                    scope: None,
                    pause_at: None,
                    review_at: None,
                    last_pause_reason: None,
                    replan_count: 0,
                },
                created_at: "2026-09-22T00:00:00Z".to_string(),
                updated_at: "2026-09-22T00:00:00Z".to_string(),
            })
            .expect("cycle");
        store
    }

    /// T23-1 — `begin_capability_receipt` MUST reject non-Started statuses
    /// (the lifecycle only allows begin → terminal, never begin → terminal
    /// directly). The guard returns `InvalidReceiptBegin` (typed).
    #[test]
    fn begin_with_terminal_status_is_rejected() {
        let mut store = Storage::open_in_memory().expect("open");
        let mut input = base_input();
        input.status = CapabilityStatus::Succeeded;
        let result = store.begin_capability_receipt(&input);
        assert!(
            matches!(result, Err(StorageError::InvalidReceiptBegin)),
            "expected InvalidReceiptBegin, got {:?}",
            result
        );
    }

    /// T23-2 — `finalize_capability_receipt` MUST reject `Started` as a
    /// target status. The transition is begin → terminal, never begin →
    /// begin-via-finalize. The guard returns `InvalidReceiptBegin` (typed).
    #[test]
    fn finalize_with_started_status_is_rejected() {
        let mut store = storage_with_capability_cycle();
        let input = base_input();
        let receipt = store.begin_capability_receipt(&input).expect("begin ok");
        // Now try to finalize with Started (forbidden).
        let result = store.finalize_capability_receipt(
            &receipt.receipt_id,
            CapabilityStatus::Started,
            None,
            "2026-09-22T00:01:00Z",
        );
        assert!(
            matches!(result, Err(StorageError::InvalidReceiptBegin)),
            "expected InvalidReceiptBegin on finalize(Started), got {:?}",
            result
        );
    }

    /// T23-3 — A terminal receipt MUST NOT be re-finalized. The guard
    /// returns `TerminalReceipt { .. }` (typed, not a generic DB error).
    /// This is the canary for "no double-finalize, no status rewind".
    #[test]
    fn finalize_already_terminal_rejected_with_typed_guard() {
        let mut store = storage_with_capability_cycle();
        let input = base_input();
        let receipt = store.begin_capability_receipt(&input).expect("begin ok");
        store
            .finalize_capability_receipt(
                &receipt.receipt_id,
                CapabilityStatus::Succeeded,
                Some(json!({"ok": true})),
                "2026-09-22T00:01:00Z",
            )
            .expect("first finalize ok");
        // Second finalize — different terminal status — must fail.
        let result = store.finalize_capability_receipt(
            &receipt.receipt_id,
            CapabilityStatus::Failed,
            Some(json!({"ok": false})),
            "2026-09-22T00:02:00Z",
        );
        match result {
            Err(StorageError::TerminalReceipt { receipt_id }) => {
                assert_eq!(receipt_id, "rec-001");
            }
            other => panic!("expected TerminalReceipt, got {:?}", other),
        }
    }

    /// T23-4 — Idempotency: same key + same request → returns the original
    /// receipt without inserting a duplicate row. This is the "safe no-op"
    /// contract documented at lib.rs:1026-1029.
    #[test]
    fn idempotent_retry_with_same_request_returns_existing_receipt() {
        let mut store = storage_with_capability_cycle();
        let input = base_input();
        let first = store.begin_capability_receipt(&input).expect("first begin");
        let second = store
            .begin_capability_receipt(&input)
            .expect("second begin");
        // Same receipt_id, same request_hash, same started_at — the contract.
        assert_eq!(first.receipt_id, second.receipt_id);
        assert_eq!(first.request_hash, second.request_hash);
        assert_eq!(first.started_at, second.started_at);
        // And only ONE row in the table.
        let all = store.list_capability_receipts("p-c3c").expect("list");
        assert_eq!(all.len(), 1, "idempotent retry must not duplicate");
    }

    /// T23-5 — Idempotency conflict: same key + DIFFERENT request →
    /// `IdempotencyConflict { key }` (typed guard, NOT silent overwrite).
    /// This protects against replay attacks where a different payload is
    /// smuggled under the same idempotency key.
    #[test]
    fn idempotent_retry_with_different_request_returns_conflict() {
        let mut store = storage_with_capability_cycle();
        let input = base_input();
        store.begin_capability_receipt(&input).expect("first begin");
        let mut tampered = base_input();
        tampered.request = json!({ "hello": "WORLD", "tampered": true });
        // The request_hash is automatically recomputed inside
        // `begin_capability_receipt` from the request payload (see
        // `hash_capability_request`); the caller does NOT pre-fill it.
        let result = store.begin_capability_receipt(&tampered);
        match result {
            Err(StorageError::IdempotencyConflict { key }) => {
                assert_eq!(key, "key-001");
            }
            other => panic!("expected IdempotencyConflict, got {:?}", other),
        }
        // And still only one row.
        let all = store.list_capability_receipts("p-c3c").expect("list");
        assert_eq!(all.len(), 1);
    }
}

#[cfg(test)]
mod cycle_lease_security_tests {
    //! T24 — fail-closed contracts on `acquire_cycle_lease` and related
    //! lifecycle operations. The cycle lease is the only authority for
    //! "which runtime currently owns cycle X"; mis-handling it would let
    //! two runtimes advance the same cycle in parallel.

    use super::*;
    use sddk_domain::cycle::{CycleManifest, CyclePath, CycleStatus, Phase};
    use sddk_domain::models::identity::{CycleRecord, ProjectRecord, WorkspaceRecord};
    use std::collections::HashMap;

    const TIMESTAMP: &str = "2026-09-22T00:00:00Z";

    fn manifest(cycle_id: &str) -> CycleManifest {
        CycleManifest {
            schema_version: 1,
            project_id: "p-c3c".to_string(),
            workspace_id: "w-c3c".to_string(),
            cycle_id: cycle_id.to_string(),
            display_name: "c3c canary".to_string(),
            status: CycleStatus::Open,
            phase: Phase::Build,
            path: CyclePath::ALite,
            branch: "feat/c3c".to_string(),
            base: "abc123".to_string(),
            head: None,
            artifacts: HashMap::new(),
            release: None,
            delivery_kind: None,
            remediation_round: 0,
            remote_url: None,
            scope: None,
            pause_at: None,
            review_at: None,
            last_pause_reason: None,
            replan_count: 0,
        }
    }

    fn storage_with_cycle(cycle_id: &str) -> Storage {
        let store = Storage::open_in_memory().expect("open");
        store
            .insert_project(&ProjectRecord {
                project_id: "p-c3c".to_string(),
                display_name: "p-c3c".to_string(),
                remote_url: None,
                scope: ".".to_string(),
                created_at: TIMESTAMP.to_string(),
            })
            .expect("project");
        store
            .insert_workspace(&WorkspaceRecord {
                workspace_id: "w-c3c".to_string(),
                project_id: "p-c3c".to_string(),
                canonical_path: "/work/c3c".to_string(),
                created_at: TIMESTAMP.to_string(),
            })
            .expect("workspace");
        store
            .insert_cycle(&CycleRecord {
                manifest: manifest(cycle_id),
                created_at: TIMESTAMP.to_string(),
                updated_at: TIMESTAMP.to_string(),
            })
            .expect("cycle");
        store
    }

    /// T24-1 — `acquire_cycle_lease` MUST reject negative `now_ms`.
    /// The guard returns `InvalidLease` (typed).
    #[test]
    fn acquire_with_negative_now_ms_is_rejected() {
        let mut store = storage_with_cycle("c-neg");
        let result = store.acquire_cycle_lease("c-neg", "owner-a", -1, 1000);
        assert!(
            matches!(result, Err(StorageError::InvalidLease)),
            "expected InvalidLease on negative now_ms, got {:?}",
            result
        );
    }

    /// T24-2 — `acquire_cycle_lease` MUST reject `expires_at_ms <= now_ms`.
    /// The lease must define a strictly positive interval.
    #[test]
    fn acquire_with_expires_at_or_before_now_is_rejected() {
        let mut store = storage_with_cycle("c-zero");
        let r1 = store.acquire_cycle_lease("c-zero", "owner-a", 100, 100);
        assert!(
            matches!(r1, Err(StorageError::InvalidLease)),
            "expires == now must reject, got {:?}",
            r1
        );
        let r2 = store.acquire_cycle_lease("c-zero", "owner-a", 100, 50);
        assert!(
            matches!(r2, Err(StorageError::InvalidLease)),
            "expires < now must reject, got {:?}",
            r2
        );
    }

    /// T24-3 — `acquire_cycle_lease` MUST fail-closed when the cycle does
    /// not exist. This prevents lease rows from being created against
    /// orphan IDs.
    #[test]
    fn acquire_on_missing_cycle_returns_not_found() {
        let mut store = Storage::open_in_memory().expect("open");
        let result = store.acquire_cycle_lease("c-ghost", "owner-a", 1000, 2000);
        match result {
            Err(StorageError::NotFound { entity, id }) => {
                assert_eq!(entity, "cycle");
                assert_eq!(id, "c-ghost");
            }
            other => panic!("expected NotFound(cycle), got {:?}", other),
        }
    }

    /// T24-4 — `acquire_cycle_lease` MUST reject a second acquire while a
    /// previous lease is still active. The guard returns `LeaseConflict`
    /// (typed, with the current owner and expiry).
    #[test]
    fn acquire_with_active_lease_returns_typed_conflict() {
        let mut store = storage_with_cycle("c-conflict");
        store
            .acquire_cycle_lease("c-conflict", "owner-a", 1000, 5000)
            .expect("first acquire ok");
        let result = store.acquire_cycle_lease("c-conflict", "owner-b", 2000, 6000);
        match result {
            Err(StorageError::LeaseConflict {
                cycle_id,
                owner,
                expires_at_ms,
            }) => {
                assert_eq!(cycle_id, "c-conflict");
                assert_eq!(owner, "owner-a");
                assert_eq!(expires_at_ms, 5000);
            }
            other => panic!("expected LeaseConflict, got {:?}", other),
        }
    }

    /// T24-5 — After a lease expires, re-acquiring increments the fencing
    /// token (1 → 2). This is the safety mechanism that invalidates stale
    /// holders' authority even after expiry.
    #[test]
    fn expired_lease_reacquire_increments_fencing_token() {
        let mut store = storage_with_cycle("c-expire");
        let first = store
            .acquire_cycle_lease("c-expire", "owner-a", 1000, 2000)
            .expect("first acquire");
        assert_eq!(first.fencing_token, 1);
        // Re-acquire AFTER expiry (`now_ms=3000 > expires_at_ms=2000`).
        let second = store
            .acquire_cycle_lease("c-expire", "owner-b", 3000, 4000)
            .expect("expired re-acquire");
        assert_eq!(
            second.fencing_token, 2,
            "fencing token must increment after expiry"
        );
        assert_eq!(second.owner, "owner-b");
        assert_eq!(second.expires_at_ms, 4000);
    }
}
