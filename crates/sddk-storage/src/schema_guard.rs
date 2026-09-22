//! AIW-S8 X06 — Schema version guard.
//!
//! Fails closed when a binary compiled against one schema version encounters
//! on-disk data it cannot honestly reconcile. Never interprets a mismatched
//! schema as if it were the expected one.

use crate::migrations::LATEST_SCHEMA_VERSION;
use crate::{Result, Storage, StorageError};

/// Current schema version this crate was compiled against.
pub const COMPILED_SCHEMA_VERSION: i32 = LATEST_SCHEMA_VERSION;

/// Oldest on-disk schema version this binary can still migrate forward.
pub const MIN_SUPPORTED_SCHEMA_VERSION: i32 = 1;

/// Compatibility verdict between on-disk schema and the compiled binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaCompatibility {
    /// The on-disk schema is the same version as the compiled binary.
    Exact,
    /// The on-disk schema is older but the migration path is available.
    Migratable {
        /// Version found on disk.
        from: i32,
        /// Version this binary migrates to.
        to: i32,
    },
    /// The on-disk schema is newer than the binary supports.
    NewerThanSupported {
        /// Version found on disk.
        on_disk: i32,
        /// Maximum version this binary understands.
        binary_max: i32,
    },
    /// The on-disk schema is older than the binary's minimum supported
    /// version (no migration path).
    TooOld {
        /// Version found on disk.
        on_disk: i32,
        /// Minimum version this binary supports.
        binary_min: i32,
    },
}

/// Classifies an on-disk schema version against this binary's compiled
/// schema version. Exposed for deterministic unit testing of mismatch
/// branches that a migrated `Storage` handle cannot produce in practice.
pub fn classify(on_disk: i32) -> SchemaCompatibility {
    if on_disk == COMPILED_SCHEMA_VERSION {
        SchemaCompatibility::Exact
    } else if on_disk > COMPILED_SCHEMA_VERSION {
        SchemaCompatibility::NewerThanSupported {
            on_disk,
            binary_max: COMPILED_SCHEMA_VERSION,
        }
    } else if on_disk < MIN_SUPPORTED_SCHEMA_VERSION {
        SchemaCompatibility::TooOld {
            on_disk,
            binary_min: MIN_SUPPORTED_SCHEMA_VERSION,
        }
    } else {
        SchemaCompatibility::Migratable {
            from: on_disk,
            to: COMPILED_SCHEMA_VERSION,
        }
    }
}

/// Reads the on-disk schema version and classifies it against the compiled
/// binary.
pub fn check_compatibility(storage: &Storage) -> Result<SchemaCompatibility> {
    Ok(classify(storage.schema_version()?))
}

/// Errors surfaced by the schema guard. Fail-closed: every variant means
/// "do not proceed", never "interpret anyway".
#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    /// Schema version check itself failed at the storage layer.
    #[error("schema version check failed: {0}")]
    CheckFailed(#[from] StorageError),
    /// The on-disk schema is newer than this binary supports.
    #[error("schema version {on_disk} on disk is newer than binary supports (max {binary_max})")]
    NewerSchema {
        /// Version found on disk.
        on_disk: i32,
        /// Maximum version this binary understands.
        binary_max: i32,
    },
    /// The on-disk schema is too old for this binary (no migration path).
    #[error("schema version {on_disk} on disk is too old (min supported {binary_min})")]
    TooOldSchema {
        /// Version found on disk.
        on_disk: i32,
        /// Minimum version this binary supports.
        binary_min: i32,
    },
}

/// Asserts the storage schema is compatible with this binary. `Exact` and
/// `Migratable` pass; anything else fails closed with a [`GuardError`].
pub fn assert_compatible(storage: &Storage) -> std::result::Result<(), GuardError> {
    match check_compatibility(storage)? {
        SchemaCompatibility::Exact | SchemaCompatibility::Migratable { .. } => Ok(()),
        SchemaCompatibility::NewerThanSupported {
            on_disk,
            binary_max,
        } => Err(GuardError::NewerSchema {
            on_disk,
            binary_max,
        }),
        SchemaCompatibility::TooOld {
            on_disk,
            binary_min,
        } => Err(GuardError::TooOldSchema {
            on_disk,
            binary_min,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_exact_on_compiled_version() {
        assert_eq!(
            classify(COMPILED_SCHEMA_VERSION),
            SchemaCompatibility::Exact
        );
    }

    #[test]
    fn classify_migratable_for_supported_older_version() {
        assert_eq!(
            classify(COMPILED_SCHEMA_VERSION - 1),
            SchemaCompatibility::Migratable {
                from: COMPILED_SCHEMA_VERSION - 1,
                to: COMPILED_SCHEMA_VERSION,
            }
        );
    }

    #[test]
    fn classify_newer_than_supported_for_future_version() {
        assert_eq!(
            classify(COMPILED_SCHEMA_VERSION + 1),
            SchemaCompatibility::NewerThanSupported {
                on_disk: COMPILED_SCHEMA_VERSION + 1,
                binary_max: COMPILED_SCHEMA_VERSION,
            }
        );
    }

    #[test]
    fn classify_too_old_below_minimum() {
        assert_eq!(
            classify(0),
            SchemaCompatibility::TooOld {
                on_disk: 0,
                binary_min: MIN_SUPPORTED_SCHEMA_VERSION,
            }
        );
    }

    #[test]
    fn assert_rejects_newer_schema_fail_closed() {
        let compat = classify(COMPILED_SCHEMA_VERSION + 5);
        let err = match compat {
            SchemaCompatibility::NewerThanSupported {
                on_disk,
                binary_max,
            } => GuardError::NewerSchema {
                on_disk,
                binary_max,
            },
            _ => panic!("expected NewerThanSupported"),
        };
        assert!(err.to_string().contains("newer than binary supports"));
    }

    // C3c (session-11) — T25 boundary tests at MIN_SUPPORTED_SCHEMA_VERSION.

    /// T25-1 — The boundary value `MIN_SUPPORTED_SCHEMA_VERSION` MUST NOT
    /// be classified as `TooOld`. The minimum-supported boundary is by
    /// definition supported.
    #[test]
    fn classify_at_min_supported_version_is_not_too_old() {
        let compat = classify(MIN_SUPPORTED_SCHEMA_VERSION);
        // The classification must be Exact (if MIN == COMPILED) or
        // Migratable (if MIN < COMPILED). It must NEVER be TooOld, because
        // TooOld is defined as `on_disk < MIN_SUPPORTED_SCHEMA_VERSION`.
        assert!(
            matches!(
                compat,
                SchemaCompatibility::Exact | SchemaCompatibility::Migratable { .. }
            ),
            "classify(MIN) must be Exact or Migratable, got {:?}",
            compat
        );
    }

    /// T25-2 — One version below MIN_SUPPORTED_SCHEMA_VERSION MUST be
    /// classified as `TooOld { on_disk: MIN - 1, binary_min: MIN }`. This
    /// proves the boundary is exclusive on the lower side, matching the
    /// guard's contract `on_disk < MIN_SUPPORTED_SCHEMA_VERSION`.
    #[test]
    fn classify_just_below_min_is_too_old() {
        let compat = classify(MIN_SUPPORTED_SCHEMA_VERSION - 1);
        assert_eq!(
            compat,
            SchemaCompatibility::TooOld {
                on_disk: MIN_SUPPORTED_SCHEMA_VERSION - 1,
                binary_min: MIN_SUPPORTED_SCHEMA_VERSION,
            }
        );
    }

    /// T25-3 — The `GuardError::TooOldSchema` variant produced by the
    /// fail-closed path must carry the on-disk version and the binary's
    /// minimum, and its Display string must mention both. This is the
    /// observable surface the operator sees in logs/receipts.
    #[test]
    fn too_old_guard_error_carries_diagnostics() {
        let on_disk = MIN_SUPPORTED_SCHEMA_VERSION - 1;
        let err = GuardError::TooOldSchema {
            on_disk,
            binary_min: MIN_SUPPORTED_SCHEMA_VERSION,
        };
        let msg = err.to_string();
        assert!(
            msg.contains(&on_disk.to_string()),
            "error must name the on-disk version: {msg}"
        );
        assert!(
            msg.contains(&MIN_SUPPORTED_SCHEMA_VERSION.to_string()),
            "error must name the binary's min supported: {msg}"
        );
        assert!(
            msg.contains("too old"),
            "error must classify the failure mode: {msg}"
        );
    }

    /// T25-4 — The `GuardError::NewerSchema` variant must carry on-disk
    /// and binary_max, and the Display string must mention "newer".
    #[test]
    fn newer_schema_guard_error_carries_diagnostics() {
        let on_disk = COMPILED_SCHEMA_VERSION + 1;
        let err = GuardError::NewerSchema {
            on_disk,
            binary_max: COMPILED_SCHEMA_VERSION,
        };
        let msg = err.to_string();
        assert!(msg.contains(&on_disk.to_string()));
        assert!(msg.contains(&COMPILED_SCHEMA_VERSION.to_string()));
        assert!(msg.contains("newer"));
    }
}
