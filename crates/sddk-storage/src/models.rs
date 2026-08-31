//! Re-export shim: value types moved to `sddk_domain::models` (Phase 1 M1
//! Sub-ciclo A). Storage retains its infra (SQLite impl, migrations,
//! StorageError) and `Storage` here continues to satisfy the legacy imports
//! (`use sddk_storage::CycleRecord`, etc.) across the workspace.

pub use sddk_domain::{
    ArtifactRecord, CapabilityReceipt, CapabilityReceiptInput, CapabilityStatus, CycleLease,
    CycleRecord, GateOutcomeStatus, GateReceipt, GateReceiptInput, GateReceiptNextSeqInput,
    IdempotencyOutcome, LedgerEvent, LedgerEventInput, LedgerVerification, ProjectRecord,
    WorkspaceRecord,
};
