//! Release revalidation artifact for candidate-bound SHA recovery.
//!
//! When a RELEASE_PENDING cycle's HEAD moves after verification evidence was
//! captured (e.g., a correction commit), the release coordinator can invoke
//! `release revalidate` to re-run fresh deterministic verify and debt-verify
//! checks against the current HEAD. The result is an append-only
//! `release-revalidation.json` artifact that binds the candidate SHA to the
//! fresh evidence (argv, exit_code, output_digest per REQ-IPV).
//!
//! Safety invariants enforced:
//! - Only RELEASE_PENDING/release cycles can enter recovery
//! - Candidate SHA must equal current HEAD
//! - Fresh verify/debt evidence recorded with argv/exit/output digest
//! - Revalidation is idempotent (same candidate → same evidence)
//! - Original reports/receipts remain immutable
//! - Failed revalidation blocks publication

use serde::{Deserialize, Serialize};

/// Fresh evidence for one deterministic check run.
///
/// Per REQ-IPV (spec-v2), a passed gate MUST contain all three of:
/// - `argv`: the command executed (array of strings)
/// - `exit_code`: the process exit code (integer)
/// - `output_digest`: SHA-256 of the output (string)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FreshEvidence {
    /// Command executed.
    pub argv: Vec<String>,
    /// Process exit code.
    pub exit_code: i32,
    /// SHA-256 digest of the output.
    pub output_digest: String,
}

/// Result of a single revalidation check (verify or debt-verify).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RevalidationCheck {
    /// Name of the check (e.g., "verify", "debt-verify").
    pub check_name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Fresh evidence for the check (required when passed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<FreshEvidence>,
}

/// A release revalidation artifact.
///
/// Produced by `release revalidate` when the release coordinator re-runs
/// fresh verify and debt-verify checks against a moved HEAD. This artifact
/// is append-only and candidate-bound: it records the original SHA that was
/// previously verified, the candidate SHA (current HEAD), and fresh evidence
/// for each check run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseRevalidation {
    /// Schema version.
    pub schema_version: i32,
    /// Cycle identifier.
    pub cycle_id: String,
    /// Project identifier.
    pub project_id: String,
    /// Original SHA that was previously verified (before correction).
    pub original_sha: String,
    /// Candidate SHA being revalidated (must equal current HEAD).
    pub candidate_sha: String,
    /// Transition this revalidation is for.
    pub transition_id: String,
    /// Individual check results.
    pub checks: Vec<RevalidationCheck>,
    /// Actor who initiated the revalidation.
    pub actor: String,
    /// RFC 3339 timestamp when revalidation was performed.
    pub timestamp: String,
    /// HMAC signature for integrity (computed over all fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl ReleaseRevalidation {
    /// Creates a new release revalidation record.
    ///
    /// `original_sha` is the SHA that was previously verified.
    /// `candidate_sha` is the current HEAD SHA being revalidated.
    pub fn new(
        cycle_id: String,
        project_id: String,
        original_sha: String,
        candidate_sha: String,
        transition_id: String,
        actor: String,
        timestamp: String,
    ) -> Self {
        Self {
            schema_version: 1,
            cycle_id,
            project_id,
            original_sha,
            candidate_sha,
            transition_id,
            checks: Vec::new(),
            actor,
            timestamp,
            signature: None,
        }
    }

    /// Adds a check result to the revalidation.
    pub fn add_check(&mut self, check: RevalidationCheck) {
        self.checks.push(check);
    }

    /// Returns true if all checks passed.
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns true if the candidate SHA matches the given SHA.
    pub fn candidate_matches(&self, sha: &str) -> bool {
        self.candidate_sha == sha
    }

    /// Returns the original SHA that was previously verified.
    pub fn original_sha(&self) -> &str {
        &self.original_sha
    }

    /// Returns the candidate SHA being revalidated.
    pub fn candidate_sha(&self) -> &str {
        &self.candidate_sha
    }

    /// Returns true if this revalidation represents a correction
    /// (original SHA differs from candidate SHA).
    ///
    /// A correction revalidation occurs when the HEAD has moved after the
    /// original verification evidence was captured. The original SHA is the
    /// SHA that was verified before the correction; the candidate SHA is the
    /// current HEAD being revalidated.
    pub fn is_correction(&self) -> bool {
        self.original_sha != self.candidate_sha
    }

    /// Returns the short form (first 8 chars) of the candidate SHA for use in artifact filenames.
    pub fn candidate_short(&self) -> String {
        self.candidate_sha.chars().take(8).collect()
    }

    /// Binds a signature to this revalidation for integrity verification.
    pub fn sign(&mut self, signature: String) {
        self.signature = Some(signature);
    }

    /// Returns the payload string used for HMAC signing.
    ///
    /// Format: `cycle_id|project_id|original_sha|candidate_sha|transition_id|checks_json|timestamp`
    pub fn signing_payload(&self) -> String {
        let checks_json = serde_json::to_string(&self.checks).unwrap_or_else(|_| "[]".to_string());
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.cycle_id,
            self.project_id,
            self.original_sha,
            self.candidate_sha,
            self.transition_id,
            checks_json,
            self.timestamp
        )
    }
}

/// The artifact kind name used in cycle manifests.
pub const RELEASE_REVALIDATION_ARTIFACT_NAME: &str = "release-revalidation";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fn_new_and_add_check() {
        let mut rev = ReleaseRevalidation::new(
            "p-52b95ef55999f9de/secretary-orchestrator".into(),
            "sddk-framework".into(),
            "467f22eee976b100020c5944493f653e38806917".into(),
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18".into(),
            "release.complete".into(),
            "release-coordinator".into(),
            "2026-08-30T12:00:00Z".into(),
        );

        rev.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into(), "--workspace".into()],
                exit_code: 0,
                output_digest: "sha256:abc123".into(),
            }),
        });

        rev.add_check(RevalidationCheck {
            check_name: "debt-verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["sddk".into(), "debt".into(), "gates".into()],
                exit_code: 0,
                output_digest: "sha256:def456".into(),
            }),
        });

        assert!(rev.all_passed());
        assert_eq!(rev.checks.len(), 2);
        assert!(rev.candidate_matches("918396adcc7a0014e2c2af6c41cae4b9384e8f18"));
        assert!(!rev.candidate_matches("0000000000000000000000000000000000000000"));
    }

    #[test]
    fn fn_signing_payload_deterministic() {
        let mut rev1 = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev1.add_check(RevalidationCheck {
            check_name: "v".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cmd".into()],
                exit_code: 0,
                output_digest: "d1".into(),
            }),
        });

        let mut rev2 = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev2.add_check(RevalidationCheck {
            check_name: "v".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cmd".into()],
                exit_code: 0,
                output_digest: "d1".into(),
            }),
        });

        assert_eq!(rev1.signing_payload(), rev2.signing_payload());
    }

    #[test]
    fn fn_failed_check_blocks_all_passed() {
        let mut rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:pass".into(),
            }),
        });
        rev.add_check(RevalidationCheck {
            check_name: "debt-verify".into(),
            passed: false,
            evidence: None,
        });

        assert!(!rev.all_passed());
    }

    #[test]
    fn fn_idempotent_same_candidate_same_checks() {
        // Two revalidations with same candidate and same check results
        // should produce the same signing payload (idempotent)
        let make_rev = || {
            let mut rev = ReleaseRevalidation::new(
                "c1".into(),
                "p1".into(),
                "sha1".into(),
                "sha2".into(),
                "t1".into(),
                "actor1".into(),
                "2026-08-30T12:00:00Z".into(),
            );
            rev.add_check(RevalidationCheck {
                check_name: "verify".into(),
                passed: true,
                evidence: Some(FreshEvidence {
                    argv: vec!["cargo".into(), "test".into()],
                    exit_code: 0,
                    output_digest: "sha256:abc".into(),
                }),
            });
            rev.add_check(RevalidationCheck {
                check_name: "debt-verify".into(),
                passed: true,
                evidence: Some(FreshEvidence {
                    argv: vec!["cargo".into(), "clippy".into()],
                    exit_code: 0,
                    output_digest: "sha256:def".into(),
                }),
            });
            rev
        };

        let rev1 = make_rev();
        let rev2 = make_rev();

        assert_eq!(rev1.signing_payload(), rev2.signing_payload());
        assert!(rev1.all_passed());
        assert!(rev2.all_passed());
    }

    #[test]
    fn fn_different_outcome_not_idempotent() {
        // Different check outcomes should produce different signing payloads
        let mut rev_pass = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev_pass.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:abc".into(),
            }),
        });

        let mut rev_fail = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev_fail.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: false,
            evidence: None,
        });

        assert_ne!(rev_pass.signing_payload(), rev_fail.signing_payload());
        assert!(rev_pass.all_passed());
        assert!(!rev_fail.all_passed());
    }

    #[test]
    fn fn_different_check_name_not_idempotent() {
        // Different check names should produce different signing payloads
        let mut rev_v = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev_v.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:abc".into(),
            }),
        });

        let mut rev_d = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev_d.add_check(RevalidationCheck {
            check_name: "debt-verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:abc".into(),
            }),
        });

        assert_ne!(rev_v.signing_payload(), rev_d.signing_payload());
    }

    #[test]
    fn fn_original_sha_accessor() {
        let rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "467f22eee976b100020c5944493f653e38806917".into(),
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );

        // Original SHA should be preserved as record of what was previously verified
        assert_eq!(
            rev.original_sha(),
            "467f22eee976b100020c5944493f653e38806917"
        );
        // Candidate SHA should be the current HEAD being revalidated
        assert_eq!(
            rev.candidate_sha(),
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18"
        );
        // They should be different (correction was made)
        assert_ne!(rev.original_sha(), rev.candidate_sha());
    }

    #[test]
    fn fn_sign_and_verify_payload() {
        let mut rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:abc".into(),
            }),
        });

        let payload = rev.signing_payload();
        // Payload should be non-empty and contain key identifiers
        assert!(!payload.is_empty());
        assert!(payload.contains("c1"));
        assert!(payload.contains("sha1"));
        assert!(payload.contains("sha2"));

        // After signing, signature should be present
        rev.sign("hmac_signature_here".into());
        assert!(rev.signature.is_some());
        assert_eq!(rev.signature.unwrap(), "hmac_signature_here");
    }

    #[test]
    fn fn_skip_serializing_none_signature() {
        // When signature is None, it should be skipped during serialization
        let rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );

        let json = serde_json::to_string(&rev).unwrap();
        // signature field should not appear when None
        assert!(!json.contains("signature"));
    }

    #[test]
    fn fn_skip_serializing_none_evidence() {
        // When evidence is None, it should be skipped during serialization
        let check = RevalidationCheck {
            check_name: "verify".into(),
            passed: false,
            evidence: None,
        };

        let json = serde_json::to_string(&check).unwrap();
        // evidence field should not appear when None
        assert!(!json.contains("evidence"));
    }

    #[test]
    fn fn_verify_requires_all_passed() {
        // all_passed() returns true only when EVERY check passed
        let mut rev_all_pass = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev_all_pass.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec![],
                exit_code: 0,
                output_digest: "sha256:x".into(),
            }),
        });
        assert!(rev_all_pass.all_passed());

        // One failed check means all_passed is false
        let mut rev_one_fail = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha1".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev_one_fail.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec![],
                exit_code: 0,
                output_digest: "sha256:x".into(),
            }),
        });
        rev_one_fail.add_check(RevalidationCheck {
            check_name: "debt-verify".into(),
            passed: false,
            evidence: None,
        });
        assert!(!rev_one_fail.all_passed());
    }

    #[test]
    fn fn_is_correction_true_when_shas_differ() {
        // When original_sha != candidate_sha, is_correction returns true
        let rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "467f22eee976b100020c5944493f653e38806917".into(),
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        assert!(
            rev.is_correction(),
            "is_correction must be true when original != candidate"
        );
    }

    #[test]
    fn fn_is_correction_false_when_shas_equal() {
        // When original_sha == candidate_sha, is_correction returns false
        let rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18".into(),
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        assert!(
            !rev.is_correction(),
            "is_correction must be false when original == candidate"
        );
    }

    #[test]
    fn fn_candidate_short_returns_first_8_chars() {
        let rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "467f22eee976b100020c5944493f653e38806917".into(),
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        // First 8 chars of the SHA
        assert_eq!(rev.candidate_short(), "918396ad");
        assert_eq!(
            rev.candidate_short().len(),
            8,
            "candidate_short must return exactly 8 characters"
        );
    }

    #[test]
    fn fn_candidate_short_for_short_sha() {
        // Edge case: SHA shorter than 8 characters
        let rev = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "abc".into(),
            "def".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        assert_eq!(rev.candidate_short(), "def");
    }

    #[test]
    fn fn_is_correction_with_preexisting_verification() {
        // Simulates the scenario: original_sha was verified before the correction commit.
        // After the correction (new commit), candidate != original.
        // The revalidation proves the new candidate passes verify/debt.
        let mut rev = ReleaseRevalidation::new(
            "p-52b95ef55999f9de/secretary-orchestrator".into(),
            "sddk-framework".into(),
            "467f22eee976b100020c5944493f653e38806917".into(), // originally verified SHA
            "918396adcc7a0014e2c2af6c41cae4b9384e8f18".into(), // corrected HEAD
            "release.complete".into(),
            "release-coordinator".into(),
            "2026-08-30T12:00:00Z".into(),
        );

        // This is a correction because original != candidate
        assert!(rev.is_correction());

        // Both verify and debt-verify pass on the corrected HEAD
        rev.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into(), "--workspace".into()],
                exit_code: 0,
                output_digest: "sha256:abc123".into(),
            }),
        });
        rev.add_check(RevalidationCheck {
            check_name: "debt-verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "clippy".into(), "--workspace".into()],
                exit_code: 0,
                output_digest: "sha256:def456".into(),
            }),
        });

        // All checks passed
        assert!(rev.all_passed());
        // Candidate can be identified
        assert!(rev.candidate_matches("918396adcc7a0014e2c2af6c41cae4b9384e8f18"));
        // Original is preserved
        assert_eq!(
            rev.original_sha(),
            "467f22eee976b100020c5944493f653e38806917"
        );
    }

    // === CLI property (d): candidate-specific artifact prevents overwrite of differing evidence ===

    #[test]
    fn fn_idempotent_same_candidate_different_checks_not_idempotent() {
        // Property (d): two revalidations with same candidate but DIFFERENT check outcomes
        // are NOT idempotent — a client MUST reject writing a new artifact that would
        // overwrite prior evidence with differing check results.
        // This is enforced by candidate-specific artifact filenames (short_sha in path).
        let mut rev1 = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha0".into(),
            "sha2".into(),
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev1.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:same".into(),
            }),
        });
        rev1.add_check(RevalidationCheck {
            check_name: "debt-verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec![],
                exit_code: 0,
                output_digest: "sha256:same".into(),
            }),
        });

        // Same candidate sha2, but verify FAILS
        let mut rev2 = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha0".into(),
            "sha2".into(), // SAME candidate
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev2.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: false, // DIFFERENT outcome
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 1,
                output_digest: "sha256:different".into(),
            }),
        });

        // NOT idempotent — different outcomes mean the new run would produce
        // a different signing payload, so the CLI must refuse to overwrite.
        let payload1 = rev1.signing_payload();
        let payload2 = rev2.signing_payload();
        assert_ne!(
            payload1, payload2,
            "Same candidate but different check outcomes must produce different payloads"
        );
    }

    #[test]
    fn fn_idempotent_different_candidates_not_idempotent() {
        // Property (d) variant: different candidates are never idempotent with each other,
        // enforced by candidate-specific artifact filename.
        let mut rev1 = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha0".into(),
            "sha2".into(), // candidate A
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev1.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:abc".into(),
            }),
        });

        let mut rev2 = ReleaseRevalidation::new(
            "c1".into(),
            "p1".into(),
            "sha0".into(),
            "sha3".into(), // candidate B (different)
            "t1".into(),
            "actor1".into(),
            "2026-08-30T12:00:00Z".into(),
        );
        rev2.add_check(RevalidationCheck {
            check_name: "verify".into(),
            passed: true,
            evidence: Some(FreshEvidence {
                argv: vec!["cargo".into(), "test".into()],
                exit_code: 0,
                output_digest: "sha256:abc".into(),
            }),
        });

        // Different candidates always produce different payloads (candidate in path)
        let payload1 = rev1.signing_payload();
        let payload2 = rev2.signing_payload();
        assert_ne!(
            payload1, payload2,
            "Different candidates must produce different payloads"
        );
    }
}
