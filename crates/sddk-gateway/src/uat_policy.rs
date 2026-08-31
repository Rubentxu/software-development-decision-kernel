//! UAT capability policy (ADR-014, ADR-0005) — executor kinds mapped to
//! declared capabilities with default risk (R2/R3).
//!
//! El control plane UAT ejecuta efectos externos (browser, red, scripts).
//! Bajo default-deny (ADR-0005), cada efecto debe estar declarado en el
//! workflow (`forge.capabilities`) o requerir approval humano explícito.
//! Este módulo mapea `UatExecutorKind` → capability canónica y evalua la
//! autorización contra `CapabilityPolicy`.

use sddk_domain::UatExecutorKind;
use thiserror::Error;

use crate::policy::{CapabilityPolicy, PolicyDecision, Risk};

/// Canonical capability names for UAT executor effects.
pub mod uat_capability {
    /// Browser automation via Playwright (sensor/actuador).
    pub const BROWSER: &str = "uat.browser";
    /// HTTP/API interaction (status oracles, Api executor).
    pub const NETWORK: &str = "uat.network";
    /// Local command execution (Cli/Script executors).
    pub const SCRIPT: &str = "uat.script";
    /// Agentic computer-use (Fara) — observe→think→act.
    pub const AGENT: &str = "uat.agent";
}

/// Failure modes of the UAT policy gate.
#[derive(Debug, Error)]
pub enum UatPolicyError {
    /// The capability is not declared in the workflow.
    #[error(
        "capability `{capability}` is not declared in workflow/workflow.yaml; add it under forge.capabilities or re-run with --approve"
    )]
    NotDeclared {
        /// Capability name.
        capability: &'static str,
    },
    /// The capability requires explicit human approval.
    #[error(
        "capability `{capability}` requires human approval (risk {risk:?}); re-run with --approve"
    )]
    ApprovalRequired {
        /// Capability name.
        capability: &'static str,
        /// Declared risk.
        risk: Risk,
    },
}

/// Default risk per executor kind, used when the workflow declares the
/// capability without an explicit `risk` field.
pub fn default_risk(kind: UatExecutorKind) -> Risk {
    match kind {
        // Browser can read/write the page under test: R2.
        UatExecutorKind::Playwright => Risk::Medium,
        // HTTP/API reads and writes external state: R2.
        UatExecutorKind::Api => Risk::Medium,
        // Local command execution: R3.
        UatExecutorKind::Cli | UatExecutorKind::Script => Risk::High,
        // Agentic autonomous browser/computer use: R3+.
        UatExecutorKind::ComputerUse => Risk::High,
        // Human via wizard: no automated effect.
        UatExecutorKind::Human => Risk::Low,
    }
}

/// Canonical capability name for an executor kind.
pub fn capability_name(kind: UatExecutorKind) -> &'static str {
    match kind {
        UatExecutorKind::Playwright => uat_capability::BROWSER,
        UatExecutorKind::Api => uat_capability::NETWORK,
        UatExecutorKind::Cli | UatExecutorKind::Script => uat_capability::SCRIPT,
        UatExecutorKind::ComputerUse => uat_capability::AGENT,
        UatExecutorKind::Human => uat_capability::SCRIPT,
    }
}

/// Evaluates whether an executor kind may run under the policy.
///
/// - Declared + not requiring approval → allowed.
/// - Declared + requires approval + `approve` → allowed.
/// - Declared + requires approval + no `approve` → `ApprovalRequired`.
/// - Not declared → `NotDeclared` (default-deny, ADR-0005).
pub fn authorize_uat(
    kind: UatExecutorKind,
    policy: &CapabilityPolicy,
    approve: bool,
) -> Result<PolicyDecision, UatPolicyError> {
    let capability = capability_name(kind);
    let decision = policy.authorize(capability, approve);
    if !decision.allowed {
        return Err(if decision.definition.is_none() {
            UatPolicyError::NotDeclared { capability }
        } else {
            let risk = decision
                .definition
                .as_ref()
                .map(|d| d.risk)
                .unwrap_or_else(|| default_risk(kind));
            UatPolicyError::ApprovalRequired { capability, risk }
        });
    }
    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::WorkflowManifest;

    fn manifest_with(capabilities: Vec<(&str, &str, &str)>) -> WorkflowManifest {
        // Serialize a minimal workflow YAML and parse it back.
        let mut yaml = String::from(
            "schema_version: 1\nworkflow:\n  id: test-workflow\n  version: 1.0.0\n  description: test\nstatuses: []\nphases: []\ntransitions: []\n",
        );
        if !capabilities.is_empty() {
            yaml.push_str("forge:\n  provider: mock\n  capabilities:\n");
            for (name, risk, consequence) in capabilities {
                yaml.push_str(&format!(
                    "    {name}:\n      risk: {risk}\n      consequence: {consequence}\n"
                ));
            }
        }
        serde_saphyr::from_str(&yaml).expect("minimal workflow parses")
    }

    #[test]
    fn declared_medium_creates_is_allowed_without_approval() {
        // Medium + Creates no exige approval (semántica del gateway, ADR-0005):
        // el browser es R2 pero no modifica estado externo persistente.
        let workflow = manifest_with(vec![("uat.browser", "medium", "creates")]);
        let policy = CapabilityPolicy::from_workflow(&workflow);
        assert!(authorize_uat(UatExecutorKind::Playwright, &policy, false).is_ok());
    }

    #[test]
    fn declared_high_capability_requires_approval() {
        let workflow = manifest_with(vec![("uat.script", "high", "creates")]);
        let policy = CapabilityPolicy::from_workflow(&workflow);
        let err = authorize_uat(UatExecutorKind::Cli, &policy, false).unwrap_err();
        assert!(matches!(err, UatPolicyError::ApprovalRequired { .. }));
        assert!(authorize_uat(UatExecutorKind::Cli, &policy, true).is_ok());
    }

    #[test]
    fn undeclared_capability_is_denied_even_with_approve() {
        let workflow = manifest_with(vec![]);
        let policy = CapabilityPolicy::from_workflow(&workflow);
        let err = authorize_uat(UatExecutorKind::Playwright, &policy, true).unwrap_err();
        assert!(matches!(err, UatPolicyError::NotDeclared { .. }));
    }

    #[test]
    fn declared_low_capability_is_allowed() {
        let workflow = manifest_with(vec![("uat.script", "low", "creates")]);
        let policy = CapabilityPolicy::from_workflow(&workflow);
        assert!(authorize_uat(UatExecutorKind::Cli, &policy, false).is_ok());
    }

    #[test]
    fn default_risk_mapping_is_sane() {
        assert_eq!(default_risk(UatExecutorKind::Playwright), Risk::Medium);
        assert_eq!(default_risk(UatExecutorKind::Cli), Risk::High);
        assert_eq!(default_risk(UatExecutorKind::ComputerUse), Risk::High);
        assert_eq!(default_risk(UatExecutorKind::Human), Risk::Low);
    }
}
