//! Default-deny capability gateway for SDDK external effects.
//!
//! The gateway owns the pipeline from ADR-0005: policy evaluation, approval
//! resolution, typed execution without a shell, safe filesystem access, output
//! sanitization, and receipt lifecycle (`started` -> `succeeded|failed`).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod artifact_store;
mod capability;
mod computer_use;
mod evidence;
mod filesystem;
mod forge;
mod gateway;
mod git;
mod oracles;
mod permissions;
mod playwright;
mod policy;
mod release;
mod runner;
mod semantic;
pub mod test_runner;
mod uat_policy;

pub use artifact_store::{ArtifactMeta, ArtifactStore, ArtifactStoreError, write_atomic};
pub use capability::{
    Capability, CapabilityError, CapabilityOutcome, EvidenceBundleWriteCapability,
    VerificationRequest,
};
pub use computer_use::{ComputerUseError, ComputerUseOutcome, ComputerUseSpec, run_computer_use};
pub use evidence::{EvidenceCollector, EvidenceCollectorError, EvidenceContext, EvidenceFile};
pub use filesystem::{FsError, ScopedFs};
pub use forge::{
    CheckState, Forge, ForgeError, GitHubForge, MergeReceipt, MockForge, PrReceipt, PrRequest,
    ReleaseReceipt, ReleaseRequest, ReleaseState,
};
pub use gateway::{CapabilityGateway, CapabilityPlan, CapabilityPlanInput, GatewayError};
pub use git::{GitBranch, GitCommit, GitError, GitExecutor, GitInspect, GitTag};
pub use oracles::{
    OracleError, OracleRunContext, aggregate_verdict, evaluate_deterministic, validate_json_schema,
};
pub use permissions::{AgentPermissions, PermissionDecision, PermissionPolicy, PermissionsError};
pub use playwright::{PlaywrightError, PlaywrightOutcome, PlaywrightSpec, run_playwright};
pub use policy::{CapabilityPolicy, Consequence, PolicyDecision, Risk};
pub use release::{
    LocalReleaseInput, LocalReleaseOutcome, LocalReleasePreconditions, ReleaseError,
    ReleaseOutcome, ReleasePlan, ReleasePlanInput, ReleaseStep, apply_local_release, apply_release,
    plan_release, reconcile_pending,
};
pub use runner::{RunOutcome, RunSpec, RunnerError, run};
pub use sddk_storage::CapabilityReceipt;
pub use semantic::{
    SemanticOracleError, SemanticOracleOutcome, SemanticOracleSpec, run_semantic_oracle,
};
pub use uat_policy::{UatPolicyError, authorize_uat, capability_name, default_risk};

/// Resolves a UAT driver/harness asset (driver.mjs, computer_use.mjs,
/// assess.mjs) from the active framework bundle, falling back to the current
/// directory. The bundle path mirrors the CLI's `resolve_assets_dir`:
/// `$SDDK_DATA_DIR/framework/current/assets/uat-driver/<name>`.
pub fn resolve_uat_driver(name: &str) -> std::path::PathBuf {
    // 1. Framework bundle runtime (installed releases / dev update sync).
    if let Some(data_dir) = std::env::var_os("SDDK_DATA_DIR") {
        let candidate = std::path::PathBuf::from(&data_dir)
            .join("framework/current/assets/uat-driver")
            .join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        let candidate = std::path::PathBuf::from(&xdg)
            .join("sddk/framework/current/assets/uat-driver")
            .join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let candidate = std::path::PathBuf::from(&home)
            .join(".local/share/sddk/framework/current/assets/uat-driver")
            .join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    // 2. Dogfooding: compiled crate manifest dir (stable at compile time).
    //    From crates/sddk-gateway/ go up two levels to the workspace root.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dogfood = manifest_dir.join("../../assets/uat-driver").join(name);
    if dogfood.is_file() {
        return dogfood;
    }
    // 3. Current working directory fallback (for dev/link scenarios).
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let candidate = cwd.join("assets/uat-driver").join(name);
    if candidate.is_file() {
        return candidate;
    }
    // 4. Relative default (caller context).
    std::path::PathBuf::from("assets/uat-driver").join(name)
}

use serde_json::Value;

impl sddk_domain::SddkErrorCode for GatewayError {
    fn code(&self) -> &'static str {
        match self {
            Self::Denied { .. } => "GATEWAY_DENIED",
            Self::ApprovalRequired { .. } => "GATEWAY_APPROVAL_REQUIRED",
            Self::ApprovalExpired { .. } => "APPROVAL_EXPIRED",
            Self::ApprovalAlreadyResolved { .. } => "GATEWAY_APPROVAL_ALREADY_RESOLVED",
            Self::ApprovalReasonRequired => "GATEWAY_APPROVAL_REASON_REQUIRED",
            Self::Idempotency(..) => "GATEWAY_IDEMPOTENCY",
            Self::Runner(..) => "GATEWAY_RUNNER",
            Self::Serialization(..) => "GATEWAY_SERIALIZATION",
            Self::Capability(..) => "GATEWAY_CAPABILITY",
        }
    }

    fn recovery(&self) -> String {
        match self {
            Self::Denied { .. } => "use a capability declared in the workflow policy".into(),
            Self::ApprovalRequired { .. } => {
                "re-run with explicit `--approve` for R3/R4 capabilities".into()
            }
            Self::ApprovalExpired { .. } => {
                "the approval window has closed; a new proposal must be submitted".into()
            }
            Self::ApprovalAlreadyResolved { .. } => {
                "this capability was already decided for the given cycle".into()
            }
            Self::ApprovalReasonRequired => {
                "supply a non-empty `--reason` with the approval decision".into()
            }
            Self::Idempotency(..) => "use a fresh idempotency key or the original request".into(),
            Self::Runner(..) => "check the typed runner executable and arguments".into(),
            Self::Serialization(..) => "fix the structured payload before retrying".into(),
            Self::Capability(..) => "check the capability execution and verification".into(),
        }
    }
}

impl sddk_domain::SddkErrorCode for crate::release::ReleaseError {
    fn code(&self) -> &'static str {
        match self {
            Self::Forge(..) => "RELEASE_FORGE",
            Self::Gateway(..) => "RELEASE_GATEWAY",
            Self::Serialization(..) => "RELEASE_SERIALIZATION",
            Self::Storage(..) => "RELEASE_STORAGE",
            Self::Git(..) => "RELEASE_GIT",
            Self::Precondition(..) => "RELEASE_PRECONDITION",
        }
    }

    fn recovery(&self) -> String {
        match self {
            Self::Forge(..) => {
                "check the provider state and re-run; apply converges without duplicates".into()
            }
            Self::Gateway(..) => "resolve the underlying gateway error first".into(),
            Self::Serialization(..) => "fix the release payload before retrying".into(),
            Self::Storage(..) => "resolve the underlying storage error first".into(),
            Self::Git(..) => {
                "restore the local and remote Git postconditions before retrying".into()
            }
            Self::Precondition(..) => {
                "satisfy the local release preconditions before retrying".into()
            }
        }
    }
}

/// Keys whose values are treated as secrets and redacted from persisted output.
const SECRET_KEY_PATTERN: [&str; 9] = [
    "api_key",
    "api_key_id",
    "authorization",
    "auth_token",
    "cookie",
    "credential",
    "password",
    "secret",
    "token",
];

/// Deterministic request key used to derive idempotency and receipt identifiers.
pub(crate) fn stable_request_key(
    project_id: &str,
    cycle_id: &Option<String>,
    capability: &str,
    args: &[String],
    reason: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    if let Some(cycle_id) = cycle_id {
        hasher.update(cycle_id.as_bytes());
    }
    hasher.update(capability.as_bytes());
    for arg in args {
        hasher.update(arg.as_bytes());
    }
    hasher.update(reason.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Recursively masks values under secret-like keys.
pub fn redact(value: Value) -> Value {
    match value {
        Value::Object(mut object) => {
            for key in object.keys().cloned().collect::<Vec<_>>() {
                let normalized = key.to_ascii_lowercase();
                if SECRET_KEY_PATTERN.iter().any(|pattern| {
                    normalized == *pattern || normalized.ends_with(&format!("_{pattern}"))
                }) {
                    object.insert(key, Value::String("<redacted>".to_owned()));
                } else if let Some(inner) = object.get(&key).cloned() {
                    object.insert(key, redact(inner));
                }
            }
            Value::Object(object)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use sddk_domain::SddkErrorCode;
    use serde_json::json;

    use super::redact;

    #[test]
    fn redaction_masks_secret_keys_recursively() {
        let input = json!({
            "branch": "feature/x",
            "credentials": {"password": "hunter2", "username": "alice"},
            "headers": {"authorization": "Bearer abc", "x-request-id": "123"}
        });
        let output = redact(input);
        assert_eq!(output["credentials"]["password"], "<redacted>");
        assert_eq!(output["credentials"]["username"], "alice");
        assert_eq!(output["headers"]["authorization"], "<redacted>");
        assert_eq!(output["headers"]["x-request-id"], "123");
        assert_eq!(output["branch"], "feature/x");
    }

    #[test]
    fn redaction_masks_keys_in_arrays() {
        let input = json!([{"token": "abc"}, {"value": 1}]);
        let output = redact(input);
        assert_eq!(output[0]["token"], "<redacted>");
        assert_eq!(output[1]["value"], 1);
    }

    #[test]
    fn approval_expired_error_code_is_stable() {
        let err = crate::GatewayError::ApprovalExpired {
            capability: "git.delete_branch".into(),
            expired_at: "2026-08-18T18:00:00Z".into(),
        };
        assert_eq!(err.code(), "APPROVAL_EXPIRED");
        assert!(err.recovery().contains("approval window"));
    }

    #[test]
    fn approval_already_resolved_error_code_is_stable() {
        let err = crate::GatewayError::ApprovalAlreadyResolved {
            cycle_id: "c-1".into(),
            capability: "git.delete_branch".into(),
        };
        assert_eq!(err.code(), "GATEWAY_APPROVAL_ALREADY_RESOLVED");
    }

    #[test]
    fn approval_reason_required_error_code_is_stable() {
        let err = crate::GatewayError::ApprovalReasonRequired;
        assert_eq!(err.code(), "GATEWAY_APPROVAL_REASON_REQUIRED");
    }
}
