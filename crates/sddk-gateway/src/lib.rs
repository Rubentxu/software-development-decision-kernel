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

/// Keys under which string values are additionally scanned for embedded
/// secret-bearing substrings (e.g. `GITHUB_TOKEN=abc...` inside a stdout
/// line). The key-name redaction (`SECRET_KEY_PATTERN`) does NOT cover
/// these surfaces because they are *containers* of free-form output,
/// not fields that *are* the secret.
const STRING_LEVEL_KEY_PATTERN: [&str; 2] = ["stdout", "stderr"];

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
            // Second pass: for string-bearing fields that act as free-form
            // output containers (stdout/stderr), apply a string-level scan
            // for embedded secret-bearing substrings.
            for key in object.keys().cloned().collect::<Vec<_>>() {
                let normalized = key.to_ascii_lowercase();
                if STRING_LEVEL_KEY_PATTERN.contains(&normalized.as_str())
                    && let Some(inner) = object.get(&key).cloned()
                    && let Value::String(s) = inner
                {
                    let masked = redact_text(&s);
                    if masked != s {
                        object.insert(key, Value::String(masked));
                    }
                }
            }
            Value::Object(object)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        other => other,
    }
}

/// Scans a free-form text surface (stdout, stderr) for substrings of the
/// shape `<secret_key>=<value>` / `<secret_key>:<value>` / `<secret_key>: <value>`
/// where `secret_key` is a member of [`SECRET_KEY_PATTERN`]. Any match has
/// its value masked while preserving a short diagnostic prefix and the
/// original value length, so engineers can still see *that* a credential
/// shape was present without seeing the credential itself.
///
/// Pre-existing `<redacted>` tokens (case-insensitive) are left untouched.
/// Lines without a recognised key pass through verbatim.
fn redact_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut cursor = 0usize;
    let lower = input.to_ascii_lowercase();
    while let Some((rel_start, rel_matched_len)) =
        find_next_secret_pair(&lower.as_bytes()[cursor..])
    {
        let matched_start = cursor + rel_start;
        // Push free-form prefix verbatim.
        out.push_str(&input[cursor..matched_start]);
        // Push key + separator + post-separator spaces verbatim.
        let key_sep_end = matched_start + rel_matched_len;
        out.push_str(&input[matched_start..key_sep_end]);
        // Walk forward to consume the value.
        let value_end = scan_value_end(bytes, key_sep_end, input.len());
        let value_len = value_end - key_sep_end;
        if value_len > 0 {
            out.push_str(&format!("<redacted:{}>", value_len));
        }
        cursor = value_end;
    }
    out.push_str(&input[cursor..]);
    out
}

/// Returns `(start_byte, total_matched_len_of_key_plus_separator)` for the
/// first occurrence of any secret key followed by `=` or `:` (optionally
/// surrounded by whitespace). Only the key + separator are matched here;
/// the value extent is computed by [`scan_value_end`].
fn find_next_secret_pair(lower: &[u8]) -> Option<(usize, usize)> {
    for (start, window) in lower.windows(1).enumerate() {
        let _ = window;
        for pattern in SECRET_KEY_PATTERN.iter() {
            let pat = pattern.as_bytes();
            if start + pat.len() > lower.len() {
                continue;
            }
            if &lower[start..start + pat.len()] != pat {
                continue;
            }
            // Word boundary: previous char is not lowercase ASCII alnum.
            // (Uppercase, `_`, ` `, `=`, `:`, `.`, etc. all count as valid
            // boundaries so that compound names like `GITHUB_TOKEN` or
            // `my_token` correctly match the `token` segment.)
            if start > 0 {
                let prev = lower[start - 1];
                if prev.is_ascii_lowercase() || prev.is_ascii_digit() {
                    continue;
                }
            }
            // Next char after the pattern is `=` or `:` (with optional spaces).
            let after = start + pat.len();
            let mut idx = after;
            while idx < lower.len() && lower[idx] == b' ' {
                idx += 1;
            }
            if idx >= lower.len() {
                continue;
            }
            let sep = lower[idx];
            if sep != b'=' && sep != b':' {
                continue;
            }
            // Skip whitespace after separator.
            let mut val_start = idx + 1;
            while val_start < lower.len() && lower[val_start] == b' ' {
                val_start += 1;
            }
            // Match length = bytes from `start` through `val_start` (exclusive of value).
            return Some((start, val_start - start));
        }
    }
    None
}

/// Walks forward from `start` until whitespace, end-of-line, or end-of-input.
fn scan_value_end(bytes: &[u8], start: usize, max: usize) -> usize {
    let mut idx = start;
    while idx < max {
        let b = bytes[idx];
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
            break;
        }
        idx += 1;
    }
    idx
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
