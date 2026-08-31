//! Release planning, idempotent application, and effect reconciliation.

use serde::Serialize;
use serde_json::json;
use thiserror::Error;

use crate::forge::{Forge, ForgeError, PrRequest, ReleaseRequest};
use crate::gateway::{CapabilityGateway, CapabilityPlanInput, GatewayError};
use crate::git::{GitError, GitExecutor};
use sddk_storage::{CapabilityReceipt, CapabilityStatus};

/// Inputs for one release across a forge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleasePlanInput {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle identifier.
    pub cycle_id: Option<String>,
    /// Source branch to release.
    pub branch: String,
    /// Target branch for the pull request.
    pub base_branch: String,
    /// Pull request title.
    pub pr_title: String,
    /// Pull request body.
    pub pr_body: String,
    /// Release tag.
    pub tag: String,
    /// Release title.
    pub release_title: String,
    /// Release notes.
    pub release_notes: String,
    /// Explicit approval for R3/R4 forge steps.
    pub approve: bool,
    /// Caller-supplied deterministic timestamp.
    pub timestamp: String,
    /// Actor responsible for the release.
    pub actor: String,
}

/// One executable release phase of the canonical sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStep {
    /// Open the pull request when none is open.
    CreatePr,
    /// Merge the open pull request.
    MergePr,
    /// Publish the release when missing.
    CreateRelease,
}

/// Deterministic release plan over the current forge state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleasePlan {
    /// Original inputs.
    pub input: ReleasePlanInput,
    /// Ordered steps required to converge.
    pub steps: Vec<ReleaseStep>,
}

/// Outcome of applying a release plan.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseOutcome {
    /// Executed steps with their receipts.
    pub applied: Vec<StepOutcome>,
    /// Steps skipped because the provider already held the effect.
    pub skipped: Vec<String>,
    /// Whether the release converged to the target state.
    pub converged: bool,
    /// Whether the version lockstep check passed before planning was allowed.
    #[serde(default)]
    pub version_lockstep_passed: bool,
}

/// Receipt of one executed release step.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StepOutcome {
    /// Step label.
    pub step: String,
    /// Persisted capability receipt id.
    pub receipt_id: String,
    /// Provider result summary.
    pub result: serde_json::Value,
}

/// Errors emitted by release planning, application, and reconciliation.
#[derive(Debug, Error)]
pub enum ReleaseError {
    /// The forge rejected an operation.
    #[error("release forge error: {0}")]
    Forge(#[from] ForgeError),
    /// A receipt could not be started or finalized.
    #[error("release gateway error: {0}")]
    Gateway(#[from] GatewayError),
    /// Structured data could not be encoded.
    #[error("release serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    /// Persistence rejected the operation.
    #[error("release storage error: {0}")]
    Storage(#[from] sddk_storage::StorageError),
    /// A local Git postcondition did not hold.
    #[error("release git error: {0}")]
    Git(#[from] GitError),
    /// A required local release precondition is absent.
    #[error("local release precondition failed: {0}")]
    Precondition(String),
}

/// Inputs for a local trunk-based release without a forge dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalReleaseInput {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle identifier.
    pub cycle_id: Option<String>,
    /// Trunk branch pushed directly by the release.
    pub branch: String,
    /// Release tag.
    pub tag: String,
    /// Annotated tag message.
    pub tag_message: String,
    /// Explicit approval for capability effects.
    pub approve: bool,
    /// Caller-supplied deterministic timestamp.
    pub timestamp: String,
    /// Actor responsible for the release.
    pub actor: String,
    /// Local workflow evidence required before publication.
    pub preconditions: LocalReleasePreconditions,
}

/// Local evidence the caller verified from the declared workflow contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalReleasePreconditions {
    /// Required local verification evidence is present and passing.
    pub verification_passed: bool,
    /// The configured local UAT gate is present and passing.
    pub uat_passed: bool,
    /// Whether the version lockstep check passed before release was allowed.
    pub version_lockstep_passed: bool,
    /// MANIFEST exact-set gate passed before push/tag.
    #[serde(default)]
    pub manifest_exact_set_verified: bool,
    /// Staged bundle roundtrip verified in `release dist`.
    #[serde(default)]
    pub bundle_roundtrip_verified: bool,
    /// `release-receipt.json` is present and HMAC-verified.
    #[serde(default)]
    pub release_receipt_verified: bool,
}

/// Outcome of a local trunk-based release.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalReleaseOutcome {
    /// SHA verified on both local HEAD and the remote trunk branch.
    pub sha: String,
    /// Verified annotated tag on the remote.
    pub tag: String,
    /// Executed capability effects with their receipts.
    pub applied: Vec<StepOutcome>,
    /// Effects skipped because their postconditions already held.
    pub skipped: Vec<String>,
    /// Whether the local release converged to trunk SHA plus remote tag.
    pub converged: bool,
}

/// Computes the ordered steps needed to converge a release.
pub fn plan_release(
    input: ReleasePlanInput,
    forge: &dyn Forge,
) -> Result<ReleasePlan, ReleaseError> {
    let mut steps = Vec::new();
    let open_pr = forge.find_open_pr(&input.branch, &input.base_branch)?;
    if open_pr.is_none() {
        steps.push(ReleaseStep::CreatePr);
    }
    steps.push(ReleaseStep::MergePr);
    if !forge
        .release_state(&input.tag)?
        .is_some_and(|state| state.published)
    {
        steps.push(ReleaseStep::CreateRelease);
    }
    Ok(ReleasePlan { input, steps })
}

/// Applies the local release contract: push trunk, verify its SHA, and create
/// or verify one annotated remote tag. It never reads provider checks or waits
/// for CI/CD or external distribution.
pub fn apply_local_release(
    gateway: &mut CapabilityGateway,
    input: &LocalReleaseInput,
    git: &GitExecutor,
) -> Result<LocalReleaseOutcome, ReleaseError> {
    let mut applied = Vec::new();
    let mut skipped = Vec::new();
    reconcile_local_pending(gateway, git)?;
    verify_local_preconditions(input, git)?;
    // L2 lockstep refusal: refuse if version lockstep check did not pass
    if !input.preconditions.version_lockstep_passed {
        return Err(ReleaseError::Precondition(
            "version lockstep check did not pass; workspace and tag versions must match".into(),
        ));
    }
    let head = git.head_sha()?;

    if git.remote_branch_sha(&input.branch)?.as_deref() == Some(head.as_str()) {
        skipped.push(format!(
            "push-main (origin/{} already at {head})",
            input.branch
        ));
    } else {
        let branch_args = vec![input.branch.clone(), head.clone()];
        let receipt = run_local_step(
            gateway,
            input,
            "git.push",
            &branch_args,
            "push direct trunk branch",
            |git| {
                Ok(
                    json!({"branch": input.branch, "sha": git.push_and_verify_branch(&input.branch)?}),
                )
            },
            git,
        )?;
        applied.push(receipt);
    }

    let sha = git.verify_head_matches_remote_branch(&input.branch)?;
    match git.remote_annotated_tag_target(&input.tag)? {
        Some(existing) if existing == sha => {
            skipped.push(format!("tag ({} already points to {sha})", input.tag));
        }
        Some(existing) => {
            return Err(ReleaseError::Git(GitError::Postcondition {
                command: format!("verify remote tag {}", input.tag),
                expected: sha,
                actual: existing,
            }));
        }
        None => {
            let tag_args = vec![input.tag.clone(), sha.clone()];
            let receipt = run_local_step(
                gateway,
                input,
                "git.tag",
                &tag_args,
                "create and push annotated release tag",
                |git| {
                    git.create_annotated_tag(&input.tag, &sha, &input.tag_message)?;
                    git.push_and_verify_annotated_tag(&input.tag, &sha)?;
                    Ok(json!({"tag": input.tag, "sha": sha, "annotated": true}))
                },
                git,
            )?;
            applied.push(receipt);
        }
    }

    // Post-conditions: the local release MUST not return Ok unless every
    // required local effect is observable. If any of them is missing, the
    // caller needs an explicit error so a retry can re-apply the effect; a
    // silent `converged: false` would mask a broken trunk-based state.
    let head_matches_remote = git.verify_head_matches_remote_branch(&input.branch)? == sha;
    let tag_matches_remote =
        git.remote_annotated_tag_target(&input.tag)?.as_deref() == Some(sha.as_str());
    if !head_matches_remote {
        return Err(ReleaseError::Precondition(format!(
            "post-conditions failed: local HEAD does not match origin/{} after release",
            input.branch
        )));
    }
    if !tag_matches_remote {
        return Err(ReleaseError::Precondition(format!(
            "post-conditions failed: remote tag {} does not peel to {sha}",
            input.tag
        )));
    }
    Ok(LocalReleaseOutcome {
        sha,
        tag: input.tag.clone(),
        applied,
        skipped,
        converged: true,
    })
}

fn verify_local_preconditions(
    input: &LocalReleaseInput,
    git: &GitExecutor,
) -> Result<(), ReleaseError> {
    if input.branch != "main" {
        return Err(ReleaseError::Precondition(format!(
            "local releases require the declared trunk branch main, received {}",
            input.branch
        )));
    }
    let inspect = git.inspect()?;
    if inspect.branch.as_deref() != Some(input.branch.as_str()) {
        return Err(ReleaseError::Precondition(format!(
            "checkout must be {}, found {}",
            input.branch,
            inspect.branch.as_deref().unwrap_or("detached HEAD")
        )));
    }
    if inspect.dirty {
        return Err(ReleaseError::Precondition(
            "worktree must be clean before local release".into(),
        ));
    }
    if !input.preconditions.verification_passed {
        return Err(ReleaseError::Precondition(
            "required local verification evidence is missing or failed".into(),
        ));
    }
    if !input.preconditions.uat_passed {
        return Err(ReleaseError::Precondition(
            "configured local UAT evidence is missing or failed".into(),
        ));
    }
    // REQ-RDI-003: exact-set preflight gate must have succeeded.
    if !input.preconditions.manifest_exact_set_verified {
        return Err(ReleaseError::Precondition(
            "MANIFEST exact-set preflight (release-dist) did not succeed".into(),
        ));
    }
    // REQ-RDI-003: staged bundle roundtrip must have succeeded.
    if !input.preconditions.bundle_roundtrip_verified {
        return Err(ReleaseError::Precondition(
            "staged bundle roundtrip (release-dist) did not succeed".into(),
        ));
    }
    // REQ-RDI-003: release-receipt HMAC must verify.
    if !input.preconditions.release_receipt_verified {
        return Err(ReleaseError::Precondition(
            "release-receipt signature or content verification failed".into(),
        ));
    }
    Ok(())
}

/// Inputs for a vault-managed closure (archive.vault.complete route).
///
/// This is the mirror of `LocalReleaseInput` for BLOCKED cycles that bypass
/// the normal release step and enter the vault archive route directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VaultClosureInput {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle identifier.
    pub cycle_id: String,
    /// Delivery kind — must be `ManagedClosureDelivery`.
    pub delivery_kind: String,
    /// Caller-supplied deterministic timestamp.
    pub timestamp: String,
    /// Actor responsible for the closure.
    pub actor: String,
}

/// Preconditions for the `archive.vault.complete` managed-closure route.
///
/// These are checked by the CLI's `run_release_vault()` before emitting
/// `vault-receipt.json`. This function provides a gateway-level mirror for
/// orchestration contexts that need to validate before delegating to the CLI.
pub fn vault_release_preconditions(input: &VaultClosureInput) -> Result<(), ReleaseError> {
    // SAFETY INVARIANT: delivery_kind must be ManagedClosureDelivery
    if input.delivery_kind != "managed-closure-delivery" {
        return Err(ReleaseError::Precondition(format!(
            "vault route requires delivery_kind=managed-closure-delivery, found {}",
            input.delivery_kind
        )));
    }
    // SAFETY INVARIANT: vault route is only for ManagedClosureDelivery cycles
    // The CLI additionally enforces status==BLOCKED and absence of release-receipt.json
    // at the runtime context level.
    Ok(())
}

/// Applies a release plan idempotently against the forge.
///
/// Every step records a capability receipt. Interrupted runs converge: an
/// already-merged PR or already-published release is skipped without duplicating
/// effects, and the provider state is re-checked before each step.
///
/// The `version_lockstep_passed` parameter records whether the caller verified
/// the release tag matches the workspace Cargo.toml version before planning.
pub fn apply_release(
    gateway: &mut CapabilityGateway,
    plan: &ReleasePlan,
    forge: &mut dyn Forge,
    version_lockstep_passed: bool,
) -> Result<ReleaseOutcome, ReleaseError> {
    let mut applied = Vec::new();
    let mut skipped = Vec::new();

    for step in &plan.steps {
        match step {
            ReleaseStep::CreatePr => {
                if forge
                    .find_open_pr(&plan.input.branch, &plan.input.base_branch)?
                    .is_some()
                {
                    skipped.push("create-pr (open PR exists)".into());
                    continue;
                }
                let branch_args = vec![plan.input.branch.clone()];
                let receipt = run_step(
                    gateway,
                    &plan.input,
                    "pr.create",
                    &branch_args,
                    "open pull request",
                    |forge: &mut dyn Forge, _: &str| {
                        let pr = forge.create_pr(&PrRequest {
                            title: plan.input.pr_title.clone(),
                            body: plan.input.pr_body.clone(),
                            head: plan.input.branch.clone(),
                            base: plan.input.base_branch.clone(),
                        })?;
                        Ok(json!({"pr_number": pr.pr_number, "url": pr.url}))
                    },
                    forge,
                )?;
                applied.push(receipt);
            }
            ReleaseStep::MergePr => {
                let Some(number) =
                    forge.find_open_pr(&plan.input.branch, &plan.input.base_branch)?
                else {
                    skipped.push("merge-pr (no open PR)".into());
                    continue;
                };
                let number_args = vec![number.to_string()];
                let receipt = run_step(
                    gateway,
                    &plan.input,
                    "pr.merge",
                    &number_args,
                    "merge pull request",
                    |forge: &mut dyn Forge, _: &str| {
                        let merged = forge.merge_pr(number)?;
                        Ok(json!({"merged": merged.merged, "merge_sha": merged.merge_sha}))
                    },
                    forge,
                )?;
                applied.push(receipt);
            }
            ReleaseStep::CreateRelease => {
                if forge
                    .release_state(&plan.input.tag)?
                    .is_some_and(|state| state.published)
                {
                    skipped.push(format!(
                        "create-release ({} already published)",
                        plan.input.tag
                    ));
                    continue;
                }
                let tag_args = vec![plan.input.tag.clone()];
                let receipt = run_step(
                    gateway,
                    &plan.input,
                    "release.create",
                    &tag_args,
                    "publish release",
                    |forge: &mut dyn Forge, _: &str| {
                        let release = forge.create_release(&ReleaseRequest {
                            tag: plan.input.tag.clone(),
                            title: plan.input.release_title.clone(),
                            notes: plan.input.release_notes.clone(),
                            target_commitish: plan.input.base_branch.clone(),
                        })?;
                        Ok(json!({"tag": release.tag, "url": release.url}))
                    },
                    forge,
                )?;
                applied.push(receipt);
            }
        }
    }

    let converged = forge
        .find_open_pr(&plan.input.branch, &plan.input.base_branch)?
        .is_none()
        && forge
            .release_state(&plan.input.tag)?
            .is_some_and(|state| state.published);
    Ok(ReleaseOutcome {
        applied,
        skipped,
        converged,
        version_lockstep_passed,
    })
}

fn run_step(
    gateway: &mut CapabilityGateway,
    input: &ReleasePlanInput,
    capability: &str,
    args: &[String],
    reason: &str,
    effect: impl FnOnce(&mut dyn Forge, &str) -> Result<serde_json::Value, ForgeError>,
    forge: &mut dyn Forge,
) -> Result<StepOutcome, ReleaseError> {
    let plan_input = CapabilityPlanInput {
        project_id: input.project_id.clone(),
        cycle_id: input.cycle_id.clone(),
        capability: capability.to_owned(),
        reason: reason.to_owned(),
        program: "forge".into(),
        args: args.to_vec(),
        env: Default::default(),
        timeout_ms: 60_000,
        output_max_bytes: 1_048_576,
        approve: input.approve,
        timestamp: input.timestamp.clone(),
        actor: input.actor.clone(),
    };
    let begin = gateway.begin_effect(&plan_input)?;
    if begin.status != CapabilityStatus::Started {
        return Ok(StepOutcome {
            step: capability.to_owned(),
            receipt_id: begin.receipt_id,
            result: begin.result.unwrap_or(serde_json::Value::Null),
        });
    }
    let argument = args.first().cloned().unwrap_or_default();
    let result = effect(forge, &argument)?;
    let receipt = gateway.finish_effect(
        &begin.receipt_id,
        CapabilityStatus::Succeeded,
        result.clone(),
        &input.timestamp,
    )?;
    Ok(StepOutcome {
        step: capability.to_owned(),
        receipt_id: receipt.receipt_id,
        result,
    })
}

fn run_local_step(
    gateway: &mut CapabilityGateway,
    input: &LocalReleaseInput,
    capability: &str,
    args: &[String],
    reason: &str,
    effect: impl FnOnce(&GitExecutor) -> Result<serde_json::Value, GitError>,
    git: &GitExecutor,
) -> Result<StepOutcome, ReleaseError> {
    let plan_input = CapabilityPlanInput {
        project_id: input.project_id.clone(),
        cycle_id: input.cycle_id.clone(),
        capability: capability.to_owned(),
        reason: reason.to_owned(),
        program: "git".into(),
        args: args.to_vec(),
        env: Default::default(),
        timeout_ms: 60_000,
        output_max_bytes: 1_048_576,
        approve: input.approve,
        timestamp: input.timestamp.clone(),
        actor: input.actor.clone(),
    };
    let begin = gateway.begin_effect(&plan_input)?;
    if begin.status != CapabilityStatus::Started {
        return Ok(StepOutcome {
            step: capability.to_owned(),
            receipt_id: begin.receipt_id,
            result: begin.result.unwrap_or(serde_json::Value::Null),
        });
    }
    let result = effect(git)?;
    let receipt = gateway.finish_effect(
        &begin.receipt_id,
        CapabilityStatus::Succeeded,
        result.clone(),
        &input.timestamp,
    )?;
    Ok(StepOutcome {
        step: capability.to_owned(),
        receipt_id: receipt.receipt_id,
        result,
    })
}

/// Reconciles started receipts against provider reality.
///
/// Forge receipts (`pr.create`, `release.create`, `release.publish`) are
/// finalized by querying the forge: a present effect finalizes as succeeded,
/// an absent one as failed. Local receipts (`git.push`, `git.tag`) follow a
/// stricter idempotency contract: a pre-effect crash MUST keep the receipt
/// `Started` so a retry can apply the missing effect, and a post-effect crash
/// MUST finalize as `Succeeded` because the local Git state already converged.
pub fn reconcile_pending(
    gateway: &mut CapabilityGateway,
    forge: &dyn Forge,
    git: &GitExecutor,
) -> Result<Vec<CapabilityReceipt>, ReleaseError> {
    let mut reconciled = Vec::new();
    for receipt in gateway.storage.list_all_capability_receipts()? {
        if receipt.status != CapabilityStatus::Started {
            continue;
        }
        let arguments = receipt
            .request
            .get("arguments")
            .and_then(|args| args.as_array())
            .cloned()
            .unwrap_or_default();
        let argument = arguments
            .first()
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let (status, result) = match receipt.capability.as_str() {
            "pr.create" => match forge.find_open_pr(argument, "")? {
                Some(_) => (CapabilityStatus::Succeeded, json!({"present": true})),
                None => (CapabilityStatus::Failed, json!({"present": false})),
            },
            "release.create" | "release.publish" => {
                let published = forge
                    .release_state(argument)?
                    .is_some_and(|state| state.published);
                if published {
                    (CapabilityStatus::Succeeded, json!({"present": true}))
                } else {
                    (CapabilityStatus::Failed, json!({"present": false}))
                }
            }
            "git.push" | "git.tag" => match local_receipt_state(&receipt, git)? {
                Some(state) => state,
                // Pre-effect crash: keep the Started receipt so the next
                // apply_local_release re-applies the missing effect.
                None => continue,
            },
            _ => continue,
        };
        let finalized = gateway.finish_effect(
            &receipt.receipt_id,
            status,
            result,
            receipt.started_at.as_str(),
        )?;
        reconciled.push(finalized);
    }
    Ok(reconciled)
}

fn reconcile_local_pending(
    gateway: &mut CapabilityGateway,
    git: &GitExecutor,
) -> Result<Vec<CapabilityReceipt>, ReleaseError> {
    let mut reconciled = Vec::new();
    for receipt in gateway.storage.list_all_capability_receipts()? {
        if receipt.status != CapabilityStatus::Started {
            continue;
        }
        let Some((status, result)) = local_receipt_state(&receipt, git)? else {
            // Pre-effect crash: keep the Started receipt so the next
            // apply_local_release re-applies the missing effect.
            continue;
        };
        let finalized = gateway.finish_effect(
            &receipt.receipt_id,
            status,
            result,
            receipt.started_at.as_str(),
        )?;
        reconciled.push(finalized);
    }
    Ok(reconciled)
}

/// Returns the canonical status of a local receipt when the effect is
/// observable on the remote. A `None` return means the effect is absent
/// (pre-effect crash): the caller MUST keep the receipt `Started` so a
/// subsequent `apply_local_release` can re-apply the missing effect.
fn local_receipt_state(
    receipt: &CapabilityReceipt,
    git: &GitExecutor,
) -> Result<Option<(CapabilityStatus, serde_json::Value)>, ReleaseError> {
    let arguments = receipt
        .request
        .get("arguments")
        .and_then(|args| args.as_array())
        .cloned()
        .unwrap_or_default();
    let Some(target) = arguments.get(1).and_then(|value| value.as_str()) else {
        return Ok(Some((
            CapabilityStatus::Failed,
            json!({"present": false, "reason": "missing expected SHA"}),
        )));
    };
    let argument = arguments
        .first()
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let present = match receipt.capability.as_str() {
        "git.push" => git.remote_branch_sha(argument)?.as_deref() == Some(target),
        "git.tag" => git.remote_annotated_tag_target(argument)?.as_deref() == Some(target),
        _ => return Ok(None),
    };
    if present {
        // Post-effect crash: the local Git state already converged; finalize
        // the Started receipt as Succeeded.
        Ok(Some((
            CapabilityStatus::Succeeded,
            json!({"present": true, "sha": target}),
        )))
    } else {
        // Pre-effect crash: keep the receipt Started so a retry can apply the
        // effect. Finalizing as Failed here would make a retry unreachable.
        Ok(None)
    }
}
