//! Release failure evidence artifact for fail-closed recovery from RELEASE_PENDING.
//!
//! When a `RELEASE_PENDING` cycle's release preconditions fail (e.g., version
//! lockstep mismatch, dirty worktree, missing gate receipts), the release
//! coordinator MUST produce a typed `release-failure-evidence` artifact before
//! the `release.recover` transition can move the cycle back to `REMEDIATING/build`.
//!
//! Safety invariants enforced by the workflow transition:
//! - Only RELEASE_PENDING/release cycles can produce release-failure-evidence
//! - Evidence must capture the specific failure kind and message
//! - Evidence is append-only (never overwritten)
//! - Recovery transition requires explicit `release-recovery-authorized` gate receipt
//!
//! The artifact kind name used in cycle manifests:
//! `release-failure-evidence`

use serde::{Deserialize, Serialize};

/// Kinds of release precondition failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseFailureKind {
    /// Version lockstep check failed (workspace Cargo.toml vs tag mismatch).
    VersionLockstepFailed,
    /// Worktree is dirty (uncommitted changes present).
    WorktreeDirty,
    /// Required gate receipt is absent or failed.
    GateFailed,
    /// UAT verdict is absent or failed.
    UatFailed,
    /// MANIFEST exact-set verification failed.
    ManifestExactSetFailed,
    /// Bundle roundtrip verification failed.
    BundleRoundtripFailed,
    /// Release receipt HMAC verification failed.
    ReleaseReceiptFailed,
    /// Cycle HEAD mismatch with local trunk HEAD.
    CycleHeadMismatch,
    /// Cycle points at non-trunk branch.
    NonTrunkBranch,
    /// Checkout is not on trunk branch.
    NotOnTrunk,
    /// Custom failure kind (for extensible error reporting).
    Custom(String),
}

impl ReleaseFailureKind {
    /// Returns the discriminant string for serialization.
    pub fn discriminant(&self) -> &str {
        match self {
            ReleaseFailureKind::VersionLockstepFailed => "version_lockstep_failed",
            ReleaseFailureKind::WorktreeDirty => "worktree_dirty",
            ReleaseFailureKind::GateFailed => "gate_failed",
            ReleaseFailureKind::UatFailed => "uat_failed",
            ReleaseFailureKind::ManifestExactSetFailed => "manifest_exact_set_failed",
            ReleaseFailureKind::BundleRoundtripFailed => "bundle_roundtrip_failed",
            ReleaseFailureKind::ReleaseReceiptFailed => "release_receipt_failed",
            ReleaseFailureKind::CycleHeadMismatch => "cycle_head_mismatch",
            ReleaseFailureKind::NonTrunkBranch => "non_trunk_branch",
            ReleaseFailureKind::NotOnTrunk => "not_on_trunk",
            ReleaseFailureKind::Custom(_) => "custom",
        }
    }
}

/// A typed release failure evidence artifact.
///
/// Produced by the release coordinator when a RELEASE_PENDING cycle cannot
/// proceed to release due to a precondition failure. This artifact captures
/// the failure kind, human-readable message, and which precondition field failed,
/// providing an audit trail for the recovery decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseFailureEvidence {
    /// Schema version.
    pub schema_version: i32,
    /// Cycle identifier.
    pub cycle_id: String,
    /// Project identifier.
    pub project_id: String,
    /// Specific failure kind.
    pub failure_kind: ReleaseFailureKind,
    /// Human-readable failure message.
    pub message: String,
    /// Which precondition field failed (e.g., "version_lockstep_passed").
    pub failed_precondition: Option<String>,
    /// Actor who recorded the failure evidence.
    pub actor: String,
    /// RFC 3339 timestamp when evidence was recorded.
    pub timestamp: String,
}

impl ReleaseFailureEvidence {
    /// Creates a new release failure evidence record.
    pub fn new(
        cycle_id: String,
        project_id: String,
        failure_kind: ReleaseFailureKind,
        message: String,
        failed_precondition: Option<String>,
        actor: String,
        timestamp: String,
    ) -> Self {
        Self {
            schema_version: 1,
            cycle_id,
            project_id,
            failure_kind,
            message,
            failed_precondition,
            actor,
            timestamp,
        }
    }

    /// Returns the artifact kind name used in cycle manifests.
    pub const ARTIFACT_NAME: &'static str = "release-failure-evidence";

    /// Returns the failed precondition field name, if any.
    pub fn failed_precondition(&self) -> Option<&str> {
        self.failed_precondition.as_deref()
    }
}

/// The artifact kind name used in cycle manifests.
pub const RELEASE_FAILURE_EVIDENCE_ARTIFACT_NAME: &str = "release-failure-evidence";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_release_failure_kind_discriminant() {
        assert_eq!(
            ReleaseFailureKind::VersionLockstepFailed.discriminant(),
            "version_lockstep_failed"
        );
        assert_eq!(
            ReleaseFailureKind::WorktreeDirty.discriminant(),
            "worktree_dirty"
        );
        assert_eq!(ReleaseFailureKind::GateFailed.discriminant(), "gate_failed");
        assert_eq!(ReleaseFailureKind::UatFailed.discriminant(), "uat_failed");
        assert_eq!(
            ReleaseFailureKind::Custom("custom_error".into()).discriminant(),
            "custom"
        );
    }

    #[test]
    fn test_new_release_failure_evidence() {
        let evidence = ReleaseFailureEvidence::new(
            "p-52b95ef55999f9de/roadmap-priority".into(),
            "sddk-framework".into(),
            ReleaseFailureKind::VersionLockstepFailed,
            "workspace version 1.0.0 does not match tag v0.9.0".into(),
            Some("version_lockstep_passed".into()),
            "release-coordinator".into(),
            "2026-08-30T12:00:00Z".into(),
        );

        assert_eq!(evidence.schema_version, 1);
        assert_eq!(evidence.cycle_id, "p-52b95ef55999f9de/roadmap-priority");
        assert!(matches!(
            evidence.failure_kind,
            ReleaseFailureKind::VersionLockstepFailed
        ));
        assert!(evidence.failed_precondition().is_some());
        assert_eq!(
            evidence.failed_precondition().unwrap(),
            "version_lockstep_passed"
        );
    }

    #[test]
    fn test_artifact_name_constant() {
        assert_eq!(
            ReleaseFailureEvidence::ARTIFACT_NAME,
            "release-failure-evidence"
        );
        assert_eq!(
            RELEASE_FAILURE_EVIDENCE_ARTIFACT_NAME,
            "release-failure-evidence"
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let evidence = ReleaseFailureEvidence::new(
            "cycle-1".into(),
            "project-1".into(),
            ReleaseFailureKind::GateFailed,
            "tests-pass gate receipt is absent".into(),
            Some("verification_passed".into()),
            "agent".into(),
            "2026-08-30T12:00:00Z".into(),
        );

        let json = serde_json::to_string(&evidence).unwrap();
        let roundtrip: ReleaseFailureEvidence = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtrip.cycle_id, evidence.cycle_id);
        assert_eq!(roundtrip.project_id, evidence.project_id);
        assert_eq!(roundtrip.message, evidence.message);
        assert_eq!(roundtrip.failed_precondition, evidence.failed_precondition);
    }

    #[test]
    fn test_custom_failure_kind() {
        let custom = ReleaseFailureKind::Custom("custom_error".into());
        assert_eq!(custom.discriminant(), "custom");

        let evidence = ReleaseFailureEvidence::new(
            "cycle-1".into(),
            "project-1".into(),
            custom,
            "A custom error occurred".into(),
            None,
            "system".into(),
            "2026-08-30T12:00:00Z".into(),
        );

        assert!(matches!(
            evidence.failure_kind,
            ReleaseFailureKind::Custom(ref s) if s == "custom_error"
        ));
    }
}
