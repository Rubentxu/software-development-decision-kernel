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
            Self::NotFound { .. } => "ensure the record exists before operating on it".into(),
            Self::Database(_) => "check the database is accessible and not corrupted".into(),
            Self::LeaseConflict { .. } => {
                "release the existing lease before acquiring a new one".into()
            }
            Self::Other(_) => "retry the operation; if the problem persists, check the logs".into(),
        }
    }
}
