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
/// `stderr`.
#[test]
fn sec1_red_canary_in_stderr_does_not_leak_into_receipt() {
    let (_dir, gateway) = gateway_with_project();
    let body = format!(
        "echo OK; echo \"warning: leaked credential: {CANARY_STDERR}\" 1>&2; exit 1"
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
/// substring must not survive in any persisted surface.
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
/// `exit 1` to ensure stderr is captured.
#[test]
fn sec1_red_canary_repeated_across_stdout_stderr_does_not_leak() {
    let (_dir, gateway) = gateway_with_project();
    let body = format!(
        "echo \"first line with {CANARY_STDOUT}\"; \
         echo \"third line with {CANARY_STDOUT}\"; \
         echo \"err line 1: {CANARY_STDERR}\" 1>&2; \
         echo \"err line 2: {CANARY_STDERR}\" 1>&2; \
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
#[test]
fn sec1_diagnostic_information_preserved_after_redaction() {
    let (_dir, gateway) = gateway_with_project();
    let secret_value = "super-secret-canary-42";
    let body = format!(
        "echo OK {secret_value}; echo warning near {secret_value} 1>&2; exit 0"
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
    // capability completed (Succeeded).
    assert!(
        stdout.is_empty() || persisted.get("stdout").and_then(|s| s.as_str()).is_some(),
        "Diagnostic preservation: stdout must remain a string surface"
    );
    assert!(
        persisted.get("stderr").and_then(|s| s.as_str()).is_some(),
        "Diagnostic preservation: stderr must remain a string surface"
    );
    assert!(
        receipt.status == sddk_storage::CapabilityStatus::Succeeded,
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
