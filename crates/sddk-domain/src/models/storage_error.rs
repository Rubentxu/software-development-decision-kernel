//! Storage error exposed via the Ledger trait. The concrete SQLite
//! implementation in `sddk_storage` wraps this via a From impl.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StorageError {
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("database error: {0}")]
    Database(String),
    #[error("lease conflict on {cycle_id} held by {owner}")]
    LeaseConflict { cycle_id: String, owner: String },
    /// Idempotency key conflict — attempt already recorded (safe no-op).
    #[error("idempotency conflict for attempt: {key}")]
    IdempotencyConflict {
        key: crate::workflow_run::IdempotencyKey,
    },
    #[error("storage error: {0}")]
    Other(String),
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
        status: crate::CycleStatus,
    },
    /// A cycle row was written without the `(project_id, workspace_id)`
    /// parent pair that its foreign key requires.
    ///
    /// `cycles` declares `FOREIGN KEY (project_id, workspace_id) REFERENCES
    /// workspaces(project_id, workspace_id) ON DELETE RESTRICT`, and
    /// `workspaces` in turn references `projects`. Adoption is what registers
    /// that pair, so a missing parent means **this project has not been
    /// adopted at this path** — not that the database is unreachable or
    /// corrupt.
    ///
    /// `missing` separates the two reachable cases because they are different
    /// situations with the same first half of the story, and a message that
    /// names the wrong one sends the operator to the wrong place:
    /// `project` when nothing was ever registered, `workspace` when a second
    /// clone of an already-adopted project resolves a different
    /// `workspace_id` for the same `project_id` (the id is derived from the
    /// canonical path, so a checkout anywhere else is a different workspace).
    ///
    /// MEDIDO (session-84 bis 7, `sddk 2.11.3`): `sddk cycle start` against a
    /// fresh state home failed 3/3 with
    /// `ENGINE_STORAGE: ... FOREIGN KEY constraint failed`, whose recovery
    /// line read `resolve the underlying storage error first`. That sends the
    /// operator to debug SQLite when the actual fix is one command
    /// (`sddk adopt apply`). `sddk cycle start --help` declared no
    /// prerequisite, so the ordering dependency was invisible from the
    /// surface. The FK is still the authoritative integrity guarantee; this
    /// variant exists so the same guarantee is *reported* as what it is
    /// instead of escaping as a generic storage fault.
    #[error("cannot store cycle {cycle_id}: this ledger has no {missing} row {parent_id}")]
    ProjectNotAdopted {
        /// The cycle that could not be stored.
        cycle_id: String,
        /// The project the cycle claims to belong to.
        project_id: String,
        /// Which parent row is absent: `"project"` or `"workspace"`.
        missing: &'static str,
        /// Identifier of the missing parent row.
        parent_id: String,
    },
}

impl crate::SddkErrorCode for StorageError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "STORAGE_NOT_FOUND",
            Self::Database(_) => "STORAGE_DATABASE_ERROR",
            Self::LeaseConflict { .. } => "STORAGE_LEASE_CONFLICT",
            Self::IdempotencyConflict { .. } => "STORAGE_IDEMPOTENCY_CONFLICT",
            Self::Other(_) => "STORAGE_ERROR",
            Self::RuntimeStatusWriteForbidden { .. } => "STORAGE_RUNTIME_STATUS_FORBIDDEN",
            Self::ProjectNotAdopted { .. } => "STORAGE_PROJECT_NOT_ADOPTED",
        }
    }
    fn recovery(&self) -> String {
        match self {
            Self::NotFound { entity, .. } if *entity == "cycle" => {
                "run `sddk cycle start --scope .` to create a new cycle, \
                 or `sddk cycle rebuild --cycle <id>` if the cycle exists in ledger events"
                    .into()
            }
            Self::NotFound { entity, .. } if *entity == "gate receipt" => {
                "evaluate the gate with `sddk cycle evaluate-gate` before the transition".into()
            }
            Self::NotFound { .. } => "ensure the record exists before operating on it".into(),
            Self::Database(_) => "check the database is accessible and not corrupted".into(),
            Self::LeaseConflict { cycle_id, .. } => {
                format!(
                    "run `sddk cycle lock inspect --cycle {}` to see the current lease, \
                     then `sddk cycle lock release --cycle {}` to release it",
                    cycle_id, cycle_id
                )
            }
            Self::IdempotencyConflict { .. } => {
                "the attempt was already recorded — this is a safe no-op, not an error".into()
            }
            Self::Other(_) => "retry the operation; if the problem persists, check the logs".into(),
            Self::RuntimeStatusWriteForbidden { status } => format!(
                "{status:?} is derived from ledger facts — do not write it to the cycle \
                 record; surface it via the derived cycle runtime summary instead"
            ),
            Self::ProjectNotAdopted { .. } => {
                "run `sddk adopt apply --scope .` at THIS checkout to register \
                 its project and workspace, then retry — a cycle row cannot \
                 exist before the pair its foreign key names does"
                    .into()
            }
        }
    }
}
