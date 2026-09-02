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
    #[error("storage error: {0}")]
    Other(String),
}

impl crate::SddkErrorCode for StorageError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "STORAGE_NOT_FOUND",
            Self::Database(_) => "STORAGE_DATABASE_ERROR",
            Self::LeaseConflict { .. } => "STORAGE_LEASE_CONFLICT",
            Self::Other(_) => "STORAGE_ERROR",
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
            Self::Other(_) => "retry the operation; if the problem persists, check the logs".into(),
        }
    }
}
