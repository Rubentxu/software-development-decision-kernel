//! Capability gateway orchestrating policy, execution, and receipts.

use std::collections::BTreeMap;

use sddk_domain::WorkflowManifest;
use sddk_domain::proposal::{Proposal, ProposalPolicy, ProposalPolicyDecision};
use sddk_storage::{CapabilityReceipt, CapabilityReceiptInput, CapabilityStatus, Storage};
use serde_json::{Value, json};
use thiserror::Error;
use time::OffsetDateTime;

use crate::capability::{Capability, EvidenceBundleWriteCapability};
use crate::policy::{CapabilityPolicy, PolicyDecision};
use crate::redact;
use crate::runner::{RunSpec, run};

/// Caller input used to plan one capability execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPlanInput {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, when applicable.
    pub cycle_id: Option<String>,
    /// Declared capability identifier.
    pub capability: String,
    /// Human-readable justification.
    pub reason: String,
    /// Executable invoked by the typed runner.
    pub program: String,
    /// Positional arguments passed without a shell.
    pub args: Vec<String>,
    /// Environment allowlist.
    pub env: BTreeMap<String, String>,
    /// Runner timeout in milliseconds.
    pub timeout_ms: u64,
    /// Runner output limit in bytes per stream.
    pub output_max_bytes: usize,
    /// Whether the caller supplies explicit human approval.
    pub approve: bool,
    /// Caller-supplied deterministic timestamps and actor.
    pub timestamp: String,
    /// Actor responsible for the request.
    pub actor: String,
}

/// A policy-validated plan ready to execute.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityPlan {
    /// Policy outcome for the capability.
    pub decision: PolicyDecision,
    /// Original request.
    pub input: CapabilityPlanInput,
    /// Derived runner specification.
    pub run_spec: RunSpec,
    /// Idempotency key binding retries to one receipt.
    pub idempotency_key: String,
    /// Deterministic receipt identifier.
    pub receipt_id: String,
}

/// Errors emitted by the capability gateway.
#[derive(Debug, Error)]
pub enum GatewayError {
    /// The policy denies the capability.
    #[error("capability {capability} is denied by policy")]
    Denied {
        /// Denied capability identifier.
        capability: String,
    },
    /// The capability requires approval that was not supplied.
    #[error("capability {capability} requires approval")]
    ApprovalRequired {
        /// Capability awaiting approval.
        capability: String,
    },
    /// Approval window has expired before a decision was recorded.
    #[error("approval for capability {capability} expired at {expired_at}")]
    ApprovalExpired {
        /// Capability whose approval window closed.
        capability: String,
        /// RFC3339 expiry timestamp from the proposal.
        expired_at: String,
    },
    /// An approval decision has already been recorded for this cycle and capability.
    #[error("approval for cycle {cycle_id} capability {capability} is already resolved")]
    ApprovalAlreadyResolved {
        /// Cycle that owns the approval.
        cycle_id: String,
        /// Capability that was already decided.
        capability: String,
    },
    /// A decision reason is required but was not supplied.
    #[error("approval decision reason is required")]
    ApprovalReasonRequired,
    /// The stored request disagrees with the supplied idempotency key.
    #[error("gateway idempotency error: {0}")]
    Idempotency(#[from] sddk_storage::StorageError),
    /// The runner failed to execute the plan.
    #[error("gateway runner error: {0}")]
    Runner(#[from] crate::runner::RunnerError),
    /// A structured payload could not be encoded.
    #[error("payload serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    /// A capability execution or verification error occurred.
    #[error("capability error: {0}")]
    Capability(#[from] crate::capability::CapabilityError),
}

/// Default-deny gateway combining policy, execution, and receipt persistence.
pub struct CapabilityGateway {
    pub(crate) policy: CapabilityPolicy,
    pub(crate) workflow: WorkflowManifest,
    pub(crate) storage: Storage,
}

impl CapabilityGateway {
    /// Constructs a gateway with a policy, workflow manifest, and the project ledger.
    pub fn new(policy: CapabilityPolicy, workflow: WorkflowManifest, storage: Storage) -> Self {
        Self {
            policy,
            workflow,
            storage,
        }
    }

    /// Evaluates policy and builds an executable plan.
    pub fn plan(&self, input: CapabilityPlanInput) -> Result<CapabilityPlan, GatewayError> {
        let decision = self.policy.authorize(&input.capability, input.approve);
        if !decision.allowed {
            if decision.requires_approval {
                return Err(GatewayError::ApprovalRequired {
                    capability: input.capability.clone(),
                });
            }
            return Err(GatewayError::Denied {
                capability: input.capability.clone(),
            });
        }
        // Merge capability-specific env defaults (git.* uses git_capability_env)
        // with caller-provided env. Caller wins on key collision (Command::env semantics).
        let defaults = self.policy.env_allowlist(&input.capability);
        let env = defaults.into_iter().chain(input.env.clone()).collect();
        let run_spec = RunSpec {
            program: input.program.clone(),
            args: input.args.clone(),
            env,
            timeout_ms: input.timeout_ms,
            output_max_bytes: input.output_max_bytes,
        };
        let request_key = crate::stable_request_key(
            &input.project_id,
            &input.cycle_id,
            &input.capability,
            &input.args,
            &input.reason,
        );
        let idempotency_key = format!("{}-{}", input.capability, &request_key[..16]);
        let receipt_id = format!(
            "cap-{}-{}",
            input.capability.replace('.', "-"),
            &request_key[..12]
        );
        Ok(CapabilityPlan {
            decision,
            input,
            run_spec,
            idempotency_key,
            receipt_id,
        })
    }

    /// Executes a plan with begin -> run -> finalize receipt lifecycle.
    ///
    /// The request and result are redacted before persistence. A failed or
    /// timed-out run finalizes the receipt as `Failed`.
    pub fn apply(&mut self, plan: &CapabilityPlan) -> Result<CapabilityReceipt, GatewayError> {
        let begin = self.begin_effect(&plan.input)?;
        if begin.status != CapabilityStatus::Started {
            return Ok(begin);
        }

        let outcome = run(&plan.run_spec)?;
        let (status, result) = if outcome.timed_out {
            (
                CapabilityStatus::Failed,
                json!({"error": "timed out", "stderr": outcome.stderr}),
            )
        } else if outcome.exit_status == Some(0) {
            (
                CapabilityStatus::Succeeded,
                json!({"stdout": outcome.stdout}),
            )
        } else {
            (
                CapabilityStatus::Failed,
                json!({"exit_status": outcome.exit_status, "stderr": outcome.stderr}),
            )
        };

        self.finish_effect(&begin.receipt_id, status, result, &plan.input.timestamp)
    }

    /// Starts a capability effect under policy and persists a started receipt.
    ///
    /// The request is redacted and the idempotency key is derived
    /// deterministically from the request; replaying the same request returns
    /// the original receipt.
    pub fn begin_effect(
        &mut self,
        input: &CapabilityPlanInput,
    ) -> Result<CapabilityReceipt, GatewayError> {
        let decision = self.policy.authorize(&input.capability, input.approve);
        if !decision.allowed {
            if decision.requires_approval {
                return Err(GatewayError::ApprovalRequired {
                    capability: input.capability.clone(),
                });
            }
            return Err(GatewayError::Denied {
                capability: input.capability.clone(),
            });
        }
        let request_key = crate::stable_request_key(
            &input.project_id,
            &input.cycle_id,
            &input.capability,
            &input.args,
            &input.reason,
        );
        let request = json!({
            "capability": input.capability,
            "arguments": input.args,
            "reason": input.reason,
        });
        Ok(self
            .storage
            .begin_capability_receipt(&CapabilityReceiptInput {
                receipt_id: format!(
                    "cap-{}-{}",
                    input.capability.replace('.', "-"),
                    &request_key[..12]
                ),
                project_id: input.project_id.clone(),
                cycle_id: input.cycle_id.clone(),
                capability: input.capability.clone(),
                idempotency_key: format!("{}-{}", input.capability, &request_key[..16]),
                request: redact(request),
                status: CapabilityStatus::Started,
                result: None,
                started_at: input.timestamp.clone(),
                completed_at: None,
                agent_version_hash: None,
                behavior_version_hash: None,
            })?)
    }

    /// Finalizes a started effect receipt with a redacted result.
    pub fn finish_effect(
        &mut self,
        receipt_id: &str,
        status: CapabilityStatus,
        result: Value,
        completed_at: &str,
    ) -> Result<CapabilityReceipt, GatewayError> {
        Ok(self.storage.finalize_capability_receipt(
            receipt_id,
            status,
            Some(redact(result)),
            completed_at,
        )?)
    }

    /// Executes a governed capability via the Proposal → Policy → Verify → Capability → Receipt chain.
    ///
    /// This method orchestrates:
    /// 1. Policy authorization via `ProposalPolicy::authorize`
    /// 2. Capability execution via `EvidenceBundleWriteCapability::execute`
    /// 3. Receipt persistence with version hashes via `finalize_capability_receipt_with_hashes`
    ///
    /// Returns the capability receipt on success, or an error if authorization was denied,
    /// execution failed, or receipt persistence failed.
    pub fn execute_governed(
        &mut self,
        proposal: Proposal,
        approve: bool,
    ) -> Result<CapabilityReceipt, GatewayError> {
        // Step 1: Evaluate policy
        // Build a ProposalPolicy from the workflow's forge capabilities
        let proposal_policy = ProposalPolicy::from_workflow(&self.workflow);
        let decision = proposal_policy.authorize(&proposal, approve);

        match decision {
            ProposalPolicyDecision::Deny => {
                return Err(GatewayError::Denied {
                    capability: proposal.capability.clone(),
                });
            }
            ProposalPolicyDecision::ApprovalRequired => {
                return Err(GatewayError::ApprovalRequired {
                    capability: proposal.capability.clone(),
                });
            }
            ProposalPolicyDecision::Allow => {
                // Continue execution
            }
        }

        // Step 2: Execute capability via EvidenceBundleWriteCapability
        let capability = EvidenceBundleWriteCapability::new("/tmp/evidence");
        let outcome = capability
            .execute(&proposal, &mut self.storage)
            .map_err(GatewayError::Capability)?;

        // Step 3: Persist receipt with version hashes
        let now = OffsetDateTime::now_utc();
        let now_str = now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| String::new());
        let timestamp = now_str.clone();

        // Build idempotency key and receipt_id
        let request_key = crate::stable_request_key(
            &proposal.project_id,
            &proposal.cycle_id,
            &proposal.capability,
            &proposal.args,
            &proposal.reason,
        );
        let idempotency_key = format!("{}-{}", proposal.capability, &request_key[..16]);
        let receipt_id = format!(
            "cap-{}-{}",
            proposal.capability.replace('.', "-"),
            &request_key[..12]
        );

        // Begin receipt
        let request = json!({
            "capability": proposal.capability,
            "arguments": proposal.args,
            "reason": proposal.reason,
        });
        let begin_receipt = self
            .storage
            .begin_capability_receipt(&CapabilityReceiptInput {
                receipt_id: receipt_id.clone(),
                project_id: proposal.project_id.clone(),
                cycle_id: proposal.cycle_id.clone(),
                capability: proposal.capability.clone(),
                idempotency_key,
                request: redact(request),
                status: CapabilityStatus::Started,
                result: None,
                started_at: timestamp.clone(),
                completed_at: None,
                agent_version_hash: None,
                behavior_version_hash: None,
            })
            .map_err(GatewayError::Idempotency)?;

        // Determine final status and result from outcome
        let (status, result) = if outcome.succeeded {
            (
                CapabilityStatus::Succeeded,
                json!({
                    "evidence_digest": outcome.evidence_digest,
                    "exit_status": outcome.exit_status,
                    "stdout": outcome.stdout,
                    "stderr": outcome.stderr,
                }),
            )
        } else {
            (
                CapabilityStatus::Failed,
                json!({
                    "exit_status": outcome.exit_status,
                    "stderr": outcome.stderr,
                }),
            )
        };

        // Finalize with version hashes
        let final_receipt = self
            .storage
            .finalize_capability_receipt_with_hashes(
                &begin_receipt.receipt_id,
                status,
                Some(redact(result)),
                &now_str,
                Some(proposal.agent_version_hash),
                Some(proposal.behavior_version_hash),
            )
            .map_err(GatewayError::Idempotency)?;

        Ok(final_receipt)
    }

    /// Lists persisted receipts for a project.
    pub fn receipts(&self, project_id: &str) -> Result<Vec<CapabilityReceipt>, GatewayError> {
        Ok(self.storage.list_capability_receipts(project_id)?)
    }

    /// Checks whether a proposal's approval window has expired.
    ///
    /// Returns `Err(ApprovalExpired)` if `now > proposal.expires_at`, otherwise `Ok(())`.
    /// The orchestrator polls this after receiving `ApprovalRequired` to detect stale requests.
    pub fn check_proposal_expiry(&self, proposal: &Proposal) -> Result<(), GatewayError> {
        if proposal.is_expired() {
            return Err(GatewayError::ApprovalExpired {
                capability: proposal.capability.clone(),
                expired_at: proposal.expires_at.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sddk_domain::{CapabilityDef, ForgeDef};
    use sddk_storage::{ProjectRecord, Storage};

    use super::{CapabilityGateway, CapabilityPlanInput};

    const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");

    fn gateway() -> (Storage, CapabilityGateway) {
        let mut workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
        workflow.forge = Some(ForgeDef {
            provider: "auto".into(),
            capabilities: Some(
                [
                    ("echo.test", Some("low"), Some("creates")),
                    ("git.delete_branch", Some("medium"), Some("irreversible")),
                ]
                .into_iter()
                .map(|(name, risk, consequence)| {
                    (
                        name.to_owned(),
                        CapabilityDef {
                            risk: risk.map(str::to_owned),
                            consequence: consequence.map(str::to_owned),
                        },
                    )
                })
                .collect(),
            ),
        });
        let policy = crate::CapabilityPolicy::from_workflow(&workflow);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.sqlite");
        let storage = Storage::open(&path).unwrap();
        storage
            .insert_project(&ProjectRecord {
                project_id: "project-1".into(),
                display_name: "project".into(),
                remote_url: Some("https://example.com/owner/project".into()),
                scope: "owner".into(),
                created_at: "2026-08-04T10:00:00Z".into(),
            })
            .unwrap();
        let gateway_storage = Storage::open(&path).unwrap();
        std::mem::forget(directory);
        let gateway = CapabilityGateway::new(policy, workflow, gateway_storage);
        (storage, gateway)
    }

    fn input(capability: &str, program: &str, approve: bool) -> CapabilityPlanInput {
        CapabilityPlanInput {
            project_id: "project-1".into(),
            cycle_id: None,
            capability: capability.into(),
            reason: "test".into(),
            program: program.into(),
            args: vec!["hello".into()],
            env: Default::default(),
            timeout_ms: 5_000,
            output_max_bytes: 1_024,
            approve,
            timestamp: "2026-08-04T10:00:00Z".into(),
            actor: "gateway-test".into(),
        }
    }

    #[test]
    fn unknown_capability_is_denied() {
        let (_storage, gateway) = gateway();
        let plan = gateway.plan(input("git.push", "echo", false));
        assert!(matches!(
            plan,
            Err(crate::GatewayError::Denied { capability }) if capability == "git.push"
        ));
    }

    #[test]
    fn irreversible_capability_requires_approval() {
        let (_storage, gateway) = gateway();
        let denied = gateway.plan(input("git.delete_branch", "echo", false));
        assert!(matches!(
            denied,
            Err(crate::GatewayError::ApprovalRequired { capability }) if capability == "git.delete_branch"
        ));
        let plan = gateway
            .plan(input("git.delete_branch", "echo", true))
            .unwrap();
        assert!(plan.decision.allowed);
    }

    #[test]
    fn apply_runs_typed_program_and_persists_redacted_receipt() {
        let (storage, mut gateway) = gateway();
        let plan = gateway.plan(input("echo.test", "echo", false)).unwrap();
        let receipt = gateway.apply(&plan).unwrap();
        assert_eq!(receipt.status, sddk_storage::CapabilityStatus::Succeeded);
        assert!(receipt.result.unwrap().to_string().contains("hello"));

        let listed = gateway.receipts("project-1").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, sddk_storage::CapabilityStatus::Succeeded);
        assert!(storage.get_capability_receipt(&receipt.receipt_id).is_ok());
    }

    #[test]
    fn apply_reuses_receipt_for_the_same_request() {
        let (_storage, mut gateway) = gateway();
        let plan = gateway.plan(input("echo.test", "echo", false)).unwrap();
        let first = gateway.apply(&plan).unwrap();
        let second = gateway.apply(&plan).unwrap();
        assert_eq!(first.receipt_id, second.receipt_id);
    }

    #[test]
    fn approval_expired_error_contains_capability_and_expiry() {
        let err = crate::GatewayError::ApprovalExpired {
            capability: "git.delete_branch".into(),
            expired_at: "2026-08-18T18:00:00Z".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("git.delete_branch"));
        assert!(msg.contains("2026-08-18T18:00:00Z"));
    }

    #[test]
    fn approval_already_resolved_error_contains_cycle_and_capability() {
        let err = crate::GatewayError::ApprovalAlreadyResolved {
            cycle_id: "c-42".into(),
            capability: "git.delete_branch".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("c-42"));
        assert!(msg.contains("git.delete_branch"));
    }

    #[test]
    fn approval_reason_required_error() {
        let err = crate::GatewayError::ApprovalReasonRequired;
        let msg = err.to_string();
        assert!(msg.contains("reason"));
    }

    #[test]
    fn check_proposal_expiry_returns_err_for_expired_proposal() {
        let (_storage, gateway) = gateway();
        // Build a proposal that is already expired (created 2 hours ago, expires 1 hour ago)
        let proposal = sddk_domain::proposal::Proposal {
            proposal_id: "p-1".into(),
            project_id: "project-1".into(),
            cycle_id: Some("c-1".into()),
            capability: "git.delete_branch".into(),
            reason: "test".into(),
            program: "echo".into(),
            args: vec!["hello".into()],
            env: Default::default(),
            timeout_ms: 5_000,
            output_max_bytes: 1_024,
            created_at: "2026-08-17T10:00:00Z".into(),
            expires_at: "2026-08-17T12:00:00Z".into(), // already expired
            status: sddk_domain::proposal::ProposalStatus::Pending,
            agent_version_hash: String::new(),
            behavior_version_hash: String::new(),
        };
        let result = gateway.check_proposal_expiry(&proposal);
        assert!(matches!(
            result,
            Err(crate::GatewayError::ApprovalExpired { capability, expired_at })
                if capability == "git.delete_branch" && expired_at == "2026-08-17T12:00:00Z"
        ));
    }

    #[test]
    fn check_proposal_expiry_returns_ok_for_valid_proposal() {
        let (_storage, gateway) = gateway();
        // Build a proposal that is not yet expired (expires 1 hour from now)
        let now = time::OffsetDateTime::now_utc();
        let future = now + time::Duration::hours(1);
        let future_str = future
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        let proposal = sddk_domain::proposal::Proposal {
            proposal_id: "p-1".into(),
            project_id: "project-1".into(),
            cycle_id: Some("c-1".into()),
            capability: "git.delete_branch".into(),
            reason: "test".into(),
            program: "echo".into(),
            args: vec!["hello".into()],
            env: Default::default(),
            timeout_ms: 5_000,
            output_max_bytes: 1_024,
            created_at: now
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            expires_at: future_str,
            status: sddk_domain::proposal::ProposalStatus::Pending,
            agent_version_hash: String::new(),
            behavior_version_hash: String::new(),
        };
        let result = gateway.check_proposal_expiry(&proposal);
        assert!(result.is_ok());
    }
}
