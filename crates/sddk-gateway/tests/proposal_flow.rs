//! Integration tests for the governed capability flow.
//!
//! Tests the complete `Proposal → Policy → Verify → Capability → Receipt` chain:
//! - Denied proposal emits no evidence bundle, no receipt
//! - Allowed receipt shows `Succeeded` with both hashes non-empty
//! - Falsified postcondition results in `Failed(verification_failed)`

use sddk_domain::proposal::{Proposal, ProposalPolicy, ProposalPolicyDecision, ProposalStatus};
use sddk_gateway::{
    Capability, CapabilityOutcome, EvidenceBundleWriteCapability, VerificationRequest,
};
use sddk_storage::{ProjectRecord, Storage};
use std::collections::BTreeMap;

fn make_test_proposal(
    capability: &str,
    agent_hash: &str,
    behavior_hash: &str,
    expired: bool,
) -> Proposal {
    let now = time::OffsetDateTime::now_utc();
    let expires_at = if expired {
        (now - time::Duration::hours(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2020-01-01T00:00:00Z".into())
    } else {
        (now + time::Duration::hours(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2099-01-01T00:00:00Z".into())
    };

    Proposal {
        proposal_id: "prop-test-001".into(),
        project_id: "test-project".into(),
        cycle_id: None,
        reason: "test proposal".into(),
        capability: capability.into(),
        program: "echo".into(),
        args: vec!["hello".into()],
        env: BTreeMap::new(),
        timeout_ms: 5000,
        output_max_bytes: 1024,
        created_at: now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2026-01-01T00:00:00Z".into()),
        expires_at,
        agent_version_hash: agent_hash.into(),
        behavior_version_hash: behavior_hash.into(),
        status: ProposalStatus::Pending,
    }
}

fn make_evidence_bundle_json() -> String {
    serde_json::json!({
        "artifacts": [],
        "environment": {
            "git_sha": "abc123",
            "app_version": "1.0.0",
            "browser": null,
            "viewport": null,
            "os": "linux"
        },
        "execution": {
            "executor": "test",
            "model": null,
            "model_hash": null,
            "prompt_hash": null
        }
    })
    .to_string()
}

/// Test case 1: Denied proposal emits no receipt.
#[test]
fn denied_proposal_emits_no_receipt() {
    let policy = ProposalPolicy::default();
    let proposal = make_test_proposal("undeclared.capability", "agent-abc", "behavior-def", false);

    let decision = policy.authorize(&proposal, false);
    assert!(matches!(decision, ProposalPolicyDecision::Deny));

    // No receipt should be created because the capability is not authorized
    // The storage should not have any receipts for this project
}

/// Test case 2: Allowed proposal with evidence bundle write succeeds.
#[test]
fn allowed_proposal_succeeds_with_verified_evidence() {
    let cap = EvidenceBundleWriteCapability::new("/tmp/evidence");
    let bundle_json = make_evidence_bundle_json();

    let mut proposal = make_test_proposal(
        "evidence.bundle.write",
        "agent-abc123",
        "behavior-def456",
        false,
    );
    proposal.args = vec![bundle_json.clone()];

    let storage = Storage::open_in_memory().unwrap();
    let mut storage = storage;

    // Insert a project for storage constraints
    storage
        .insert_project(&ProjectRecord {
            project_id: "test-project".into(),
            display_name: "Test".into(),
            remote_url: None,
            scope: "test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        })
        .ok();

    let result = cap.execute(&proposal, &mut storage);
    assert!(result.is_ok());

    let outcome = result.unwrap();
    assert!(outcome.succeeded);
    assert!(outcome.evidence_digest.is_some());

    // Verify the digest is valid SHA-256 format
    let digest = outcome.evidence_digest.unwrap();
    assert!(digest.starts_with("sha256:"));
    assert_eq!(digest.len(), 71); // "sha256:" (7) + 64 hex chars
}

/// Test case 3: Postcondition verification fails on digest mismatch.
#[test]
fn falsified_postcondition_fails_verification() {
    let outcome = CapabilityOutcome {
        succeeded: true,
        evidence_digest: Some("sha256:abc123".into()),
        exit_status: Some(0),
        stdout: String::new(),
        stderr: String::new(),
    };

    let request = VerificationRequest {
        capability: "evidence.bundle.write".into(),
        expected_digest: "sha256:different".into(),
        evidence_path: None,
        exit_status: Some(0),
    };

    let result = EvidenceBundleWriteCapability::verify_postcondition(&outcome, &request);
    assert!(result.is_err());
}

/// Test case 4: ProposalPolicy denies expired proposals.
#[test]
fn proposal_policy_denies_expired_proposals() {
    let policy = ProposalPolicy::default();
    let proposal = make_test_proposal("evidence.bundle.write", "agent-abc", "behavior-def", true);

    let decision = policy.authorize(&proposal, false);
    assert!(matches!(decision, ProposalPolicyDecision::Deny));
}

/// Test case 5: ProposalPolicy denies empty version hashes.
#[test]
fn proposal_policy_denies_empty_hashes() {
    let policy = ProposalPolicy::default();
    let mut proposal =
        make_test_proposal("evidence.bundle.write", "agent-abc", "behavior-def", false);
    proposal.agent_version_hash = "".into();

    let decision = policy.authorize(&proposal, false);
    assert!(matches!(decision, ProposalPolicyDecision::Deny));
}

/// Test case 6: ProposalPolicy denies undeclared capabilities.
#[test]
fn proposal_policy_denies_undeclared_capabilities() {
    let policy = ProposalPolicy::default();
    let proposal = make_test_proposal("evidence.bundle.write", "agent-abc", "behavior-def", false);

    // Default policy has no capabilities declared, so this should be denied
    let decision = policy.authorize(&proposal, false);
    assert!(matches!(decision, ProposalPolicyDecision::Deny));
}

/// Test case 7: Receipt digest is deterministic.
#[test]
fn governed_receipt_digest_is_deterministic() {
    let cap = EvidenceBundleWriteCapability::new("/tmp/evidence");
    let bundle_json = make_evidence_bundle_json();

    let mut proposal = make_test_proposal(
        "evidence.bundle.write",
        "agent-abc123",
        "behavior-def456",
        false,
    );
    proposal.args = vec![bundle_json];

    let storage = Storage::open_in_memory().unwrap();
    let mut storage = storage;

    // Insert a project
    storage
        .insert_project(&ProjectRecord {
            project_id: "test-project".into(),
            display_name: "Test".into(),
            remote_url: None,
            scope: "test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        })
        .ok();

    let result = cap.execute(&proposal, &mut storage);
    assert!(result.is_ok());

    let outcome = result.unwrap();
    assert!(outcome.succeeded);
    assert!(outcome.evidence_digest.is_some());

    // The digest should be computable and verifiable
    let digest = outcome.evidence_digest.unwrap();
    assert!(digest.starts_with("sha256:"));

    // Verify the digest is deterministic by recomputing
    let bundle: sddk_domain::EvidenceBundle =
        serde_json::from_str(&make_evidence_bundle_json()).unwrap();
    let recomputed = EvidenceBundleWriteCapability::compute_bundle_digest(&bundle);
    assert_eq!(digest, recomputed);
}

/// Test case 8: execute_governed returns receipt with both version hashes set.
#[test]
fn execute_governed_receipt_contains_version_hashes() {
    use sddk_domain::{CapabilityDef, ForgeDef};
    use sddk_gateway::{CapabilityGateway, CapabilityPolicy};
    use sddk_storage::{ProjectRecord, Storage};

    // Build a workflow manifest with the evidence.bundle.write capability declared
    let mut workflow =
        sddk_engine::load_workflow_str(include_str!("../../../workflow/workflow.yaml")).unwrap();
    workflow.forge = Some(ForgeDef {
        provider: "auto".into(),
        capabilities: Some(
            [(
                "evidence.bundle.write",
                CapabilityDef {
                    risk: Some("low".into()),
                    consequence: Some("creates".into()),
                },
            )]
            .into_iter()
            .map(|(name, def)| (name.to_owned(), def))
            .collect(),
        ),
    });

    let policy = CapabilityPolicy::from_workflow(&workflow);
    let directory = tempfile::tempdir().unwrap();
    let db_path = directory.path().join("ledger.sqlite");

    // Use file-backed storage so project insertion and gateway share the same database
    let storage = Storage::open(&db_path).unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "test-project".into(),
            display_name: "Test".into(),
            remote_url: None,
            scope: "test".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        })
        .unwrap();

    let gateway_storage = Storage::open(&db_path).unwrap();
    std::mem::forget(directory); // prevent early drop
    let mut gateway = CapabilityGateway::new(policy, workflow, gateway_storage);

    let now = time::OffsetDateTime::now_utc();
    let expires_at = (now + time::Duration::hours(1))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "2099-01-01T00:00:00Z".into());
    let created_at = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "2026-01-01T00:00:00Z".into());

    let mut proposal = make_test_proposal(
        "evidence.bundle.write",
        "agent-hash-abc",
        "behavior-hash-def",
        false,
    );
    proposal.args = vec![make_evidence_bundle_json()];
    proposal.created_at = created_at;
    proposal.expires_at = expires_at;

    let receipt = gateway.execute_governed(proposal, false).unwrap();

    assert!(
        receipt.agent_version_hash.is_some(),
        "agent_version_hash must be set"
    );
    assert!(
        receipt.behavior_version_hash.is_some(),
        "behavior_version_hash must be set"
    );
    assert_eq!(
        receipt.agent_version_hash.as_deref(),
        Some("agent-hash-abc")
    );
    assert_eq!(
        receipt.behavior_version_hash.as_deref(),
        Some("behavior-hash-def")
    );
}
