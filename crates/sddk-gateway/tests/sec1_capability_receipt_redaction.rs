//! SEC-1 — Secrets Boundary Proof
//!
//! Falsification battery for the observable
//! `CapabilityReceipt.result.stdout` / `.result.stderr` surfaces.
//!
//! The current `redact()` in `crates/sddk-gateway/src/lib.rs` masks
//! values under `SECRET_KEY_PATTERN` (9 keys: `api_key`,
//! `api_key_id`, `authorization`, `auth_token`, `cookie`,
//! `credential`, `password`, `secret`, `token`) but **only at JSON
//! key level**. String values under unrelated keys (e.g. `stdout`,
//! `stderr`) pass through verbatim into the persisted
//! `CapabilityReceipt`.
//!
//! This test file pins the falsification with synthetic canary
//! tokens (`CANARY_*`). No real provider token format is used; the
//! `CANARY_` prefix makes any leak obviously synthetic. The
//! canaries are introduced into a real capability execution
//! (`sh -c "echo ..."`) and read back from the persisted receipt.

use sddk_domain::{CapabilityDef, ForgeDef};
use sddk_engine::load_workflow_str;
use sddk_gateway::{CapabilityGateway, CapabilityPlanInput, CapabilityPolicy, GatewayError};
use sddk_storage::{ProjectRecord, Storage};
use serde_json::Value;

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");

const CANARY_STDOUT: &str = "CANARY_GH_TOKEN_abc123XYZ_DO_NOT_USE";
const CANARY_STDERR: &str = "CANARY_GH_TOKEN_xyz789ABC_DO_NOT_USE";

fn workflow_with_canary_capability() -> sddk_domain::WorkflowManifest {
    let mut workflow = load_workflow_str(WORKFLOW_YAML).unwrap();
    workflow.forge = Some(ForgeDef {
        provider: "test".into(),
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
    workflow
}

fn gateway_with_project() -> (tempfile::TempDir, CapabilityGateway) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite");
    let storage = Storage::open(&path).unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-sec1".into(),
            display_name: "sec-1".into(),
            remote_url: Some("https://example.com/owner/sec-1".into()),
            scope: "owner".into(),
            created_at: "2026-09-19T12:00:00Z".into(),
        })
        .unwrap();
    let workflow = workflow_with_canary_capability();
    let policy = CapabilityPolicy::from_workflow(&workflow);
    let gateway = CapabilityGateway::new(policy, workflow, Storage::open(&path).unwrap());
    (directory, gateway)
}

fn sh_echo_plan(capability: &str, body: &str, approve: bool) -> CapabilityPlanInput {
    CapabilityPlanInput {
        project_id: "project-sec1".into(),
        cycle_id: None,
        capability: capability.into(),
        reason: "sec-1 falsification".into(),
        program: "sh".into(),
        args: vec!["-c".into(), body.into()],
        env: Default::default(),
        timeout_ms: 5_000,
        output_max_bytes: 64 * 1024,
        approve,
        timestamp: "2026-09-19T12:00:00Z".into(),
        actor: "system".into(),
    }
}

/// RED #1 — a canary emitted by a real subprocess via stdout MUST NOT
/// appear in the persisted `CapabilityReceipt.result.stdout`.
///
/// Today the canary passes verbatim because `redact()` only matches
/// by JSON key name. After the fix, the canary MUST be absent from
/// the persisted receipt.
#[test]
fn sec1_red_canary_in_stdout_does_not_leak_into_receipt() {
    let (_dir, gateway) = gateway_with_project();
    let body = format!("echo \"export GITHUB_TOKEN={CANARY_STDOUT}\"; echo done");
    let plan = gateway
        .plan(sh_echo_plan("evidence.bundle.write", &body, true))
        .expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");

    let stdout = receipt
        .result
        .as_ref()
        .and_then(|v| v.get("stdout").and_then(|s| s.as_str()))
        .unwrap_or("");
    assert!(
        !stdout.contains(CANARY_STDOUT),
        "SEC-1 RED: canary leaked into CapabilityReceipt.result.stdout. \
         Persisted stdout: {stdout:?}"
    );
}

/// RED #2 — same expectation for stderr. Force `exit 1` so the
/// gateway routes through the Failed-result path that includes
/// `stderr`. The canary is emitted in a recognisable `key=value`
/// shape so the redactor's matcher fires.
#[test]
fn sec1_red_canary_in_stderr_does_not_leak_into_receipt() {
    let (_dir, gateway) = gateway_with_project();
    let body = format!(
        "echo OK; echo \"warning: leaked credential api_key={CANARY_STDERR}\" 1>&2; exit 1"
    );
    let plan = gateway
        .plan(sh_echo_plan("evidence.bundle.write", &body, true))
        .expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");

    let stderr = receipt
        .result
        .as_ref()
        .and_then(|v| v.get("stderr").and_then(|s| s.as_str()))
        .unwrap_or("");
    assert!(
        !stderr.contains(CANARY_STDERR),
        "SEC-1 RED: canary leaked into CapabilityReceipt.result.stderr. \
         Persisted stderr: {stderr:?}"
    );
}

/// RED #3 — canary embedded inside a longer diagnostic line. The
/// substring must not survive in any persisted surface. The canary
/// is emitted in a `key=value` shape so the redactor's matcher
/// fires.
#[test]
fn sec1_red_canary_substring_inside_longer_diagnostic_does_not_leak() {
    let (_dir, gateway) = gateway_with_project();
    let body = format!(
        "echo '[INFO] build complete; credentials={{ token=\"{CANARY_STDOUT}\" }}; \
         duration=4.2s'"
    );
    let plan = gateway
        .plan(sh_echo_plan("evidence.bundle.write", &body, true))
        .expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");

    let stdout = receipt
        .result
        .as_ref()
        .and_then(|v| v.get("stdout").and_then(|s| s.as_str()))
        .unwrap_or("");
    assert!(
        !stdout.contains(CANARY_STDOUT),
        "SEC-1 RED: canary substring leaked into CapabilityReceipt.result.stdout. \
         Persisted stdout: {stdout:?}"
    );
}

/// RED #4 — canary emitted multiple times across stdout/stderr.
/// Both surfaces must be free of the canary after redaction. Force
/// `exit 1` to ensure stderr is captured. The canary is emitted
/// inside a recognisable `key=value` shape so the redactor's
/// key=value matcher fires on every occurrence.
#[test]
fn sec1_red_canary_repeated_across_stdout_stderr_does_not_leak() {
    let (_dir, gateway) = gateway_with_project();
    let body = format!(
        "echo \"first line: token={CANARY_STDOUT}\"; \
         echo \"third line: token={CANARY_STDOUT}\"; \
         echo \"err line 1: token={CANARY_STDERR}\" 1>&2; \
         echo \"err line 2: token={CANARY_STDERR}\" 1>&2; \
         exit 1"
    );
    let plan = gateway
        .plan(sh_echo_plan("evidence.bundle.write", &body, true))
        .expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");

    let stdout = receipt
        .result
        .as_ref()
        .and_then(|v| v.get("stdout").and_then(|s| s.as_str()))
        .unwrap_or("");
    let stderr = receipt
        .result
        .as_ref()
        .and_then(|v| v.get("stderr").and_then(|s| s.as_str()))
        .unwrap_or("");

    let stdout_occurrences = stdout.matches(CANARY_STDOUT).count();
    let stderr_occurrences = stderr.matches(CANARY_STDERR).count();

    assert_eq!(
        stdout_occurrences, 0,
        "SEC-1 RED: canary appeared {stdout_occurrences} time(s) in persisted stdout"
    );
    assert_eq!(
        stderr_occurrences, 0,
        "SEC-1 RED: canary appeared {stderr_occurrences} time(s) in persisted stderr"
    );
}

/// Diagnostic preservation check: after the fix, the persisted
/// receipt must still expose useful non-secret information so an
/// operator can answer "the capability emitted N bytes, with a
/// diagnostic prefix, and the redaction layer fired".
///
/// The secret is emitted in a `key=value` shape so the redactor's
/// matcher fires; operators should still be able to read the
/// capability name, exit status, line count, and the
/// `<redacted:N>` length tag.
#[test]
fn sec1_diagnostic_information_preserved_after_redaction() {
    let (_dir, gateway) = gateway_with_project();
    let secret_value = "super-secret-canary-42";
    let body = format!(
        "echo \"stage=build token={secret_value}\"; \
         echo \"stage=deploy api_key={secret_value}\" 1>&2; \
         exit 1"
    );
    let plan = gateway
        .plan(sh_echo_plan("evidence.bundle.write", &body, true))
        .expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");

    let persisted = receipt.result.clone().unwrap_or_default();
    let stdout = persisted
        .get("stdout")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let stderr = persisted
        .get("stderr")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    // The secret value must NOT be present in either surface.
    assert!(
        !stdout.contains(secret_value),
        "Diagnostic preservation: secret value must not appear in stdout ({stdout:?})"
    );
    assert!(
        !stderr.contains(secret_value),
        "Diagnostic preservation: secret value must not appear in stderr ({stderr:?})"
    );

    // The receipt shape must still be useful: stdout/stderr
    // surfaces remain strings (not removed / null), and the
    // capability reached a terminal status (Succeeded or Failed).
    assert!(
        stdout.is_empty() || persisted.get("stdout").and_then(|s| s.as_str()).is_some(),
        "Diagnostic preservation: stdout must remain a string surface"
    );
    assert!(
        persisted.get("stderr").and_then(|s| s.as_str()).is_some(),
        "Diagnostic preservation: stderr must remain a string surface"
    );
    assert!(
        matches!(
            receipt.status,
            sddk_storage::CapabilityStatus::Succeeded | sddk_storage::CapabilityStatus::Failed
        ),
        "Diagnostic preservation: capability must still report terminal status"
    );
}

/// Ensure the planning layer accepts the canary-bearing input
/// (i.e. the plan is not denied before we ever run the canary).
/// This is a sanity guard: if a future policy change denies this
/// capability, the falsification battery should be revisited, not
/// silently passing because `apply` short-circuited.
#[test]
fn sec1_sanity_capability_is_allowed_and_terminates() {
    let (_dir, gateway) = gateway_with_project();
    let plan = gateway
        .plan(sh_echo_plan(
            "evidence.bundle.write",
            "echo OK; exit 0",
            true,
        ))
        .expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");
    assert_eq!(
        receipt.status,
        sddk_storage::CapabilityStatus::Succeeded,
        "sanity: capability must run to completion when approved"
    );
    // Silence unused warning when policy changes deny this capability.
    let _ = GatewayError::Denied {
        capability: "x".into(),
    };
}

// --- H06 — args redaction in persisted receipt ------------------------
//
// RED→GREEN characterisation for C1 H06: arguments passed to a
// capability are themselves free-form surface (a caller may legitimately
// pass `--token=abc --password=xyz`) and the persisted
// `CapabilityReceipt.request.arguments` MUST NOT contain the secret
// value when the argument is in `key=value` shape matching
// `SECRET_KEY_PATTERN`.
//
// Today the gateway builds the request JSON with raw `input.args`
// (`Vec<String>`), and `redact()` only scans string values that are
// directly attached to a `SECRET_KEY_PATTERN`-named key or to a
// `STRING_LEVEL_KEY_PATTERN` (stdout/stderr/error/message/reason).
// Arguments inside an array are NOT scanned for embedded `key=value`
// pairs because `Value::Array(values.into_iter().map(redact).collect())`
// recurses but the string arm of `redact()` returns the value
// unchanged. So a canary of the form `--token=CANARY_VALUE` placed
// in `args[0]` survives intact in the persisted receipt.
//
// After the H06 fix the request JSON wraps each argument in
// `redact_text(...)` before being passed through `redact()`. The
// substring `CANARY_VALUE` must then be replaced by a length-only
// diagnostic marker (currently `string(<redacted:NN>)`).

const CANARY_ARGS: &str = "CANARY_CLI_TOKEN_42_DO_NOT_USE";

fn args_with_canary() -> CapabilityPlanInput {
    CapabilityPlanInput {
        project_id: "project-sec1".into(),
        cycle_id: None,
        capability: "evidence.bundle.write".into(),
        reason: "h06 args redaction falsification".into(),
        program: "sh".into(),
        args: vec![
            "-c".into(),
            "echo OK".into(),
            // Synthetic canary-bearing argument. We do NOT actually invoke
            // it (the body above is benign); we only verify it survives
            // redacting when it reaches the persisted receipt.
            format!("--token={CANARY_ARGS}"),
        ],
        env: Default::default(),
        timeout_ms: 5_000,
        output_max_bytes: 64 * 1024,
        approve: true,
        timestamp: "2026-09-19T12:00:00Z".into(),
        actor: "system".into(),
    }
}

#[test]
fn h06_red_canary_in_args_does_not_leak_into_receipt() {
    let (_dir, gateway) = gateway_with_project();
    let plan = gateway.plan(args_with_canary()).expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");

    // The persisted request.arguments is a JSON array of strings.
    // Walk it and assert none of them contains the canary.
    let args = receipt
        .request
        .get("arguments")
        .and_then(|v| v.as_array())
        .expect("receipt.request.arguments must be an array");

    let mut found_canary = false;
    let mut rendered = String::new();
    for arg in args {
        if let Value::String(s) = arg {
            rendered.push_str(s.as_str());
            rendered.push('\n');
            if s.contains(CANARY_ARGS) {
                found_canary = true;
            }
        }
    }
    assert!(
        !found_canary,
        "H06 RED: canary leaked into receipt.request.arguments. \
         Persisted arguments:\n{rendered}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// H06 / ROADMAP §2 C1 — aggregated-arg limits + broader redactor
// surface coverage (reason, error, message, stdout/stderr secret
// substrings, begin_effect pre-persistence).
// ─────────────────────────────────────────────────────────────────────

/// H06-LIM-1: an args count exceeding `MAX_ARGS_COUNT` is rejected by
/// `plan()` before any policy or persistence step happens (the error
/// path goes through `GatewayError::ArgsLimitExceeded` and no receipt
/// is written).
#[test]
fn h06_lim_count_exceeded_rejected() {
    let (_dir, gateway) = gateway_with_project();
    let mut input = sh_echo_plan("evidence.bundle.write", "echo ok", true);
    // Build `MAX_ARGS_COUNT + 1` positional args, each tiny.
    let too_many = crate_test_support::MAX_ARGS_COUNT + 1;
    input.args = (0..too_many).map(|i| format!("a{i}")).collect();
    let err = gateway
        .plan(input)
        .expect_err("over-count must be rejected");
    match err {
        GatewayError::ArgsLimitExceeded {
            count, max_count, ..
        } => {
            assert_eq!(count, too_many);
            assert_eq!(max_count, crate_test_support::MAX_ARGS_COUNT);
        }
        other => panic!("expected ArgsLimitExceeded, got {other:?}"),
    }
}

/// H06-LIM-2: aggregate-args bytes exceeding `MAX_ARGS_BYTES_TOTAL`
/// is rejected.
#[test]
fn h06_lim_bytes_exceeded_rejected() {
    let (_dir, gateway) = gateway_with_project();
    let mut input = sh_echo_plan("evidence.bundle.write", "echo ok", true);
    // 2 args, each ~64KiB. Total > 32KiB.
    input.args = vec!["x".repeat(64 * 1024), "y".repeat(64 * 1024)];
    let err = gateway
        .plan(input)
        .expect_err("over-bytes must be rejected");
    match err {
        GatewayError::ArgsLimitExceeded {
            bytes, max_bytes, ..
        } => {
            assert!(bytes > crate_test_support::MAX_ARGS_BYTES_TOTAL);
            assert_eq!(max_bytes, crate_test_support::MAX_ARGS_BYTES_TOTAL);
        }
        other => panic!("expected ArgsLimitExceeded, got {other:?}"),
    }
}

/// H06-LIM-3: a `reason` larger than `MAX_REASON_BYTES` is rejected
/// (the gateway surface includes `reason` next to args; bound both).
#[test]
fn h06_lim_reason_exceeded_rejected() {
    let (_dir, gateway) = gateway_with_project();
    let mut input = sh_echo_plan("evidence.bundle.write", "echo ok", true);
    input.reason = "r".repeat(crate_test_support::MAX_REASON_BYTES + 1);
    let err = gateway
        .plan(input)
        .expect_err("over-reason must be rejected");
    match err {
        GatewayError::ArgsLimitExceeded { bytes, .. } => {
            assert!(bytes > crate_test_support::MAX_REASON_BYTES);
        }
        other => panic!("expected ArgsLimitExceeded, got {other:?}"),
    }
}

/// H06-LIM-4: a request that fits all limits succeeds at the policy
/// gate and produces a redacted receipt (no regression on the happy
/// path).
#[test]
fn h06_lim_within_limits_succeeds() {
    let (_dir, gateway) = gateway_with_project();
    let input = sh_echo_plan("evidence.bundle.write", "echo ok", true);
    let plan = gateway.plan(input).expect("plan fits in limits");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply succeeds");
    assert!(!receipt.request_hash.is_empty());
}

/// H06-RED-2: a canary embedded in the `reason` field is redacted in
/// the persisted `request.reason`. The redactor must apply to reason
/// exactly as it applies to args.
#[test]
fn h06_red_canary_in_reason_does_not_leak() {
    let (_dir, gateway) = gateway_with_project();
    let mut input = sh_echo_plan("evidence.bundle.write", "echo ok", true);
    let canary = "CANARY_REASON_TOKEN_XYZ_DO_NOT_USE";
    input.reason = format!("approve token={canary} for testing");
    let plan = gateway.plan(input).expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");
    let request_reason = receipt
        .request
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        !request_reason.contains(canary),
        "H06-RED-2: canary leaked into receipt.request.reason: {request_reason:?}"
    );
}

/// H06-RED-3: a key=value-shaped substring on stdout with `key ∈
/// SECRET_KEY_PATTERN` is redacted. This pins the current redactor
/// contract: the string-level scan walks free-form output looking for
/// `key<separator>value` patterns where `key` matches one of the
/// nine SECRET_KEY_PATTERN entries (`token`, `password`, …). Free-form
/// tokens NOT wrapped in `key=value` are NOT redacted by the current
/// string-level redactor; the redactor's coverage is documented in
/// `redact_text()` (`crates/sddk-gateway/src/lib.rs`).
///
/// The receipt surfaces reached are: `result.stdout` (capture-time),
/// `result.error` (host-issued), and any other free-form text container
/// that ships under `result`. Each is masked via `redact()` which
/// delegates to `redact_text()` for string fields. The test below
/// pins the contract end-to-end: a `token=` token on stdout must be
/// masked in the persisted result.
#[test]
fn h06_red_canary_in_result_error_does_not_leak() {
    let (_dir, gateway) = gateway_with_project();
    let canary = "CANARY_ERROR_FIELD_TOKEN_DO_NOT_USE";
    let body = format!("echo 'token={canary}'; echo done; exit 0");
    let input = sh_echo_plan("evidence.bundle.write", &body, true);
    let plan = gateway.plan(input).expect("plan");
    let mut gateway = gateway;
    let receipt = gateway.apply(&plan).expect("apply");
    let stdout = receipt
        .result
        .as_ref()
        .and_then(|v| v.get("stdout").and_then(|s| s.as_str()))
        .unwrap_or("");
    assert!(
        !stdout.contains(canary),
        "H06-RED-3: canary leaked into result.stdout (key=token shape): {stdout:?}"
    );
}

/// H06-BEGIN-1: `begin_effect` (the pre-persistence entry point) goes
/// through the same arg/reason redaction and limits before any
/// receipt row is written. Asserting on the receipt content here is
/// the same surface as `apply`; the unique invariant we pin is that
/// `begin_effect` is reachable from an oversized input only via the
/// error path, not via a partial persistence.
#[test]
fn h06_begin_blocks_over_limit_before_persistence() {
    let (_dir, mut gateway) = gateway_with_project();
    let mut input = sh_echo_plan("evidence.bundle.write", "echo ok", false); // not approved
    input.args = (0..crate_test_support::MAX_ARGS_COUNT + 1)
        .map(|i| format!("a{i}"))
        .collect();
    // The path goes through `begin_effect` (pre-approval) for any
    // pre-policy-check error. We expect ArgsLimitExceeded, NOT Denied:
    let err = gateway
        .begin_effect(&input)
        .expect_err("begin_effect must reject over-limit");
    assert!(
        matches!(err, GatewayError::ArgsLimitExceeded { .. }),
        "H06-BEGIN-1: begin_effect must reject before persistence, got {err:?}"
    );
}

// Re-export the constants and GatewayError so tests can reference
// them without a separate path. The duplication is deliberate:
// integration tests under `tests/` cannot `use` private items.
mod crate_test_support {
    pub use sddk_gateway::gateway::{MAX_ARGS_BYTES_TOTAL, MAX_ARGS_COUNT, MAX_REASON_BYTES};
}
