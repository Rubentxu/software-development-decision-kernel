//! Contract tests for gate classification — RED phase.
//!
//! Tests the GateKind/RecoveryAction enums, RecoveryHint structured payload,
//! and the closed registry loaded from `gates/classifications.toml`.
//!
//! Per [[REQ-Gate-Classification-Discriminator]] and [[REQ-Recovery-Action-Contract]]:
//! - GateKind ∈ {Security, Process, Mixed}
//! - RecoveryAction ∈ {RecoverForward, FailClosed, Advisory}
//! - RecoveryHint carries RFC 9457-shaped {recovery_command, hint}
//! - Registry is closed; unknown gate names fail to load

use sddk_domain::models::gate_classification::{
    GateClassification, GateClassificationError, GateKind, RecoveryAction, RecoveryHint,
    WaiverAuthority, load_classifications,
};
use std::path::{Path, PathBuf};

/// Returns the path to `gates/classifications.toml` relative to the crate root.
fn classifications_toml_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gates/classifications.toml")
}

// ── GateKind parsing ────────────────────────────────────────────────────────────

#[test]
fn gate_kind_security_from_str() {
    assert_eq!("security".parse::<GateKind>().unwrap(), GateKind::Security);
}

#[test]
fn gate_kind_process_from_str() {
    assert_eq!("process".parse::<GateKind>().unwrap(), GateKind::Process);
}

#[test]
fn gate_kind_mixed_from_str() {
    assert_eq!("mixed".parse::<GateKind>().unwrap(), GateKind::Mixed);
}

#[test]
fn gate_kind_unknown_from_str_err() {
    let result: Result<GateKind, String> = "unknown".parse();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid gate kind"));
}

// ── RecoveryAction parsing ─────────────────────────────────────────────────────

#[test]
fn recovery_action_recover_forward_from_str() {
    assert_eq!(
        "recover_forward".parse::<RecoveryAction>().unwrap(),
        RecoveryAction::RecoverForward
    );
}

#[test]
fn recovery_action_fail_closed_from_str() {
    assert_eq!(
        "fail_closed".parse::<RecoveryAction>().unwrap(),
        RecoveryAction::FailClosed
    );
}

#[test]
fn recovery_action_advisory_from_str() {
    assert_eq!(
        "advisory".parse::<RecoveryAction>().unwrap(),
        RecoveryAction::Advisory
    );
}

#[test]
fn recovery_action_unknown_from_str_err() {
    let result: Result<RecoveryAction, String> = "unknown".parse();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid recovery action"));
}

// ── RecoveryHint RFC 9457 shape ────────────────────────────────────────────────

#[test]
fn recovery_hint_construct() {
    let hint = RecoveryHint {
        recovery_command: "sddk cycle replan --restage-to=Apply".to_string(),
        hint: "Replan to Apply after fixing gate configuration".to_string(),
    };
    assert_eq!(
        hint.recovery_command,
        "sddk cycle replan --restage-to=Apply"
    );
    assert_eq!(hint.hint, "Replan to Apply after fixing gate configuration");
}

// ── Registry loading ───────────────────────────────────────────────────────────

#[test]
fn load_classifications_from_toml() {
    // gates/classifications.toml ships Wave-1 gates as Process-classified
    let toml_path = classifications_toml_path();
    let result = load_classifications(&toml_path);
    assert!(result.is_ok(), "registry should load: {:?}", result.err());
    let classifications = result.unwrap();
    assert!(!classifications.is_empty(), "Wave-1 gates must be present");
}

#[test]
fn load_classifications_wave1_gates_are_process() {
    let toml_path = classifications_toml_path();
    let classifications = load_classifications(&toml_path).unwrap();

    // Wave-1 budget gates must default to Process class
    let wave1_gates = [
        "gate-uat",
        "gate-budget",
        "gate-debt-verification",
        "gate-delivery-quality",
        "gate-release-clean",
    ];
    for gate_name in wave1_gates {
        let classification = classifications.get(gate_name);
        assert!(
            classification.is_some(),
            "Wave-1 gate {gate_name} must be in registry"
        );
        assert_eq!(
            classification.unwrap().class,
            GateKind::Process,
            "Wave-1 gate {gate_name} must be classified Process"
        );
    }
}

#[test]
fn load_classifications_recoverable_flag() {
    let toml_path = classifications_toml_path();
    let classifications = load_classifications(&toml_path).unwrap();

    // Wave-1 Process gates are recoverable (RecoverForward)
    let wave1_gates = [
        "gate-uat",
        "gate-budget",
        "gate-debt-verification",
        "gate-delivery-quality",
        "gate-release-clean",
    ];
    for gate_name in wave1_gates {
        let classification = classifications.get(gate_name).unwrap();
        assert!(
            classification.recoverable,
            "Wave-1 gate {gate_name} must be recoverable"
        );
        assert_eq!(
            classification.recovery_action,
            Some(RecoveryAction::RecoverForward),
            "Wave-1 gate {gate_name} must have RecoverForward action"
        );
    }
}

#[test]
fn load_classifications_waiver_authority_optional() {
    let toml_path = classifications_toml_path();
    let classifications = load_classifications(&toml_path).unwrap();

    // Waiver authority is optional; Wave-1 gates may omit it
    for (_gate_name, classification) in &classifications {
        if let Some(ref waiver) = classification.waiver_authority {
            assert!(
                matches!(
                    waiver,
                    WaiverAuthority::Lead | WaiverAuthority::Security | WaiverAuthority::Owner
                ),
                "waiver_authority must be Lead, Security, or Owner"
            );
            assert!(
                classification.waiver_expiry_days.is_some(),
                "if waiver_authority is set, waiver_expiry_days must also be set"
            );
            assert!(
                classification.waiver_expiry_days.unwrap() <= 30,
                "waiver_expiry_days must be ≤ 30 per REQ-Process-Gate-Recoverable-Default"
            );
        }
    }
}

#[test]
fn load_classifications_nonexistent_path_err() {
    let result = load_classifications(Path::new("nonexistent/classifications.toml"));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        GateClassificationError::RegistryNotFound(_)
    ));
}
