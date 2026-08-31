//! Persistence model types organized by concern.

#![allow(missing_docs)]

pub mod approval;
pub mod capability;
pub mod debt_report;
pub mod finding;
pub mod gate_receipt;
pub mod identity;
pub mod inc_record;
pub mod lease;
pub mod ledger;
pub mod priority;
pub mod release_failure_evidence;
pub mod release_revalidation;
pub mod severity;
pub mod status;
pub mod storage_error;
pub mod uat;

// Re-exports for the public API surface (mirrors original models.rs layout)
pub use approval::{ApprovalDecision, ApprovalReceipt, ApprovalReceiptInput};
pub use capability::{
    CapabilityReceipt, CapabilityReceiptInput, CapabilityStatus, IdempotencyOutcome,
};
pub use debt_report::DebtReport;
pub use finding::Finding;
pub use gate_receipt::{GateOutcomeStatus, GateReceipt, GateReceiptInput, GateReceiptNextSeqInput};
pub use identity::{CycleRecord, ProjectRecord, WorkspaceRecord};
pub use inc_record::IncRecord;
pub use lease::CycleLease;
pub use ledger::{ArtifactRecord, LedgerEvent, LedgerEventInput, LedgerVerification};
pub use priority::Priority;
pub use release_failure_evidence::{
    RELEASE_FAILURE_EVIDENCE_ARTIFACT_NAME, ReleaseFailureEvidence, ReleaseFailureKind,
};
pub use release_revalidation::{
    FreshEvidence, RELEASE_REVALIDATION_ARTIFACT_NAME, ReleaseRevalidation, RevalidationCheck,
};
pub use severity::Severity;
pub use status::{FindingStatus, IncStatus};
pub use storage_error::StorageError;
pub use uat::UatResultRow;
