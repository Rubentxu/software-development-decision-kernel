//! verify_kernel_cmd.rs — A4-1: Generic Verify kernel CLI (CLI B of design).
//!
//! Cycle: `p-63676b11dc0ef88f/a4-1-generic-verify`
//! Spec: `docs/architecture/specs/arch-spec-043-generic-verify.md`
//!
//! # Purpose
//!
//! `sddk verify-kernel --domain architecture --claim <contract-id>` drives the
//! Generic Verify kernel for the architecture domain.
//!
//! This is CLI B (as defined in design.md §File Changes):
//! - Takes `--domain` and `--claim` arguments.
//! - Looks up the domain in the registry.
//! - Builds the change basis from git diff (or explicit base).
//! - Calls `VerifyKernel::evaluate` with the claim and observations.
//! - Outputs the `VerificationResult` in text or JSON format.
//!
//! # Anti-encroachment
//!
//! This command does NOT reuse `verify_cmd.rs` (M6.1 shadow facade).
//! It is a new, distinct command family.

use crate::{CliEnvironment, CommandOutput, OutputFormat};
use clap::Parser;
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::observation::ObservationSet;
use sddk_engine::verify_kernel::{ChangeBasis, VerificationClaim, VerifyKernel, default_registry};
use std::path::PathBuf;

/// CLI B: `sddk verify-kernel --domain architecture --claim <contract-id>`
///
/// Drives the Generic Verify kernel for a specific domain.
#[derive(Debug, Clone, Parser)]
pub struct VerifyArgs {
    /// Verification domain (e.g., "architecture").
    #[arg(long)]
    pub domain: String,

    /// Claim identifier (domain-specific).
    /// For architecture: contract ID.
    #[arg(long)]
    pub claim: String,

    /// Base revision for the change basis (default: origin/main, then HEAD~1).
    #[arg(long)]
    pub base: Option<String>,

    /// Scope to units touched since `--base` (git diff).
    #[arg(long)]
    pub changed: bool,

    /// Repository root.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Evaluation time in epoch-ms. Defaults to the current wall clock.
    #[arg(long)]
    pub now_ms: Option<i64>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Static-provider mode: path to the real `cognicode-mcp` binary.
    /// When set together with `--domain static_provider`, the claim
    /// is evaluated against a live provider observation (AIW-S1).
    #[arg(long)]
    pub provider_bin: Option<String>,

    /// Static-provider mode: subject canonical tag to observe
    /// (e.g. `unit:crates/sddk-engine/src/lib.rs`).
    #[arg(long)]
    pub subject: Option<String>,
}

/// Run the `verify` command.
pub fn run_verify(args: VerifyArgs, _environment: &CliEnvironment) -> CommandOutput {
    if args.domain == "static_provider" {
        return run_verify_static_provider(args);
    }

    // 1. Build the domain registry.
    let registry = default_registry();

    // 2. Look up the domain.
    let domain = match registry.lookup(&args.domain) {
        Ok(d) => d,
        Err(_e) => {
            return crate::failure(format!(
                "verify: domain not found: {}\nAvailable domains: {:?}",
                args.domain,
                registry.all_names()
            ));
        }
    };

    // 3. Build the change basis.
    let basis = if args.changed {
        match build_change_basis_from_git(&args.root, args.base.as_deref()) {
            Ok(b) => b,
            Err(e) => {
                return crate::failure(format!("verify: failed to compute change basis: {}", e));
            }
        }
    } else {
        ChangeBasis::new(vec![])
    };

    // 4. Build the claim.
    let claim = VerificationClaim::ArchitectureConformance(
        sddk_engine::verify_kernel::ArchitectureConformanceClaim {
            contract_id: args.claim.clone(),
            basis,
        },
    );

    // 5. Build the observation set (empty for now; future: from CogniCode, Chronos, etc.).
    let observations = ObservationSet::new();

    // 6. Evaluate.
    let result = VerifyKernel::evaluate(&claim, &observations, domain);

    // 7. Output.
    match args.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&result).unwrap_or_else(|e| {
                format!("{{\"error\": \"failed to serialize result: {}\"}}", e)
            });
            CommandOutput {
                status: exit_code_for_result(&result),
                stdout: format!("{}\n", json),
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let text = format_verification_text(&args.domain, &args.claim, &result);
            CommandOutput {
                status: exit_code_for_result(&result),
                stdout: text,
                stderr: String::new(),
            }
        }
    }
}

/// Static-provider verification path (AIW-S1): spawn the real
/// provider, observe the subject, evaluate the claim against the
/// resulting non-empty observation set.
fn run_verify_static_provider(args: VerifyArgs) -> CommandOutput {
    use sddk_engine::architecture_graph::SoftwareUnitRef;
    use sddk_engine::code_intelligence_port::CodeIntelligencePort;
    use sddk_engine::code_intelligence_port_mcp::CogniCodeMcpAdapter;
    use sddk_engine::evidence_ref::EvidenceRef;
    use sddk_engine::observation::types::{
        ObservationBasis, ObservationOrigin, ObservationSet as CanonicalSet, ObservationStance,
        ObservationSubject, SoftwareEntityRef, SoftwareRelation,
    };
    use sddk_engine::semantic_kind::CoreRelationKind;
    use sddk_engine::verify_kernel::adapter_static_provider::{
        StaticProviderClaim, StaticProviderDomain,
    };
    use sddk_engine::verify_kernel::types::VerificationClaim;
    use std::time::Duration;

    let bin = match args.provider_bin.as_deref() {
        Some(b) if !b.is_empty() => b.to_string(),
        _ => {
            return crate::failure(
                "verify: --domain static_provider requires --provider-bin <cognicode-mcp path>"
                    .to_string(),
            );
        }
    };
    let subject_tag = match args.subject.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return crate::failure(
                "verify: --domain static_provider requires --subject <tag>".to_string(),
            );
        }
    };

    // Spawn + negotiate (fails closed: Unavailable / ProtocolMajorMismatch).
    let provider = match CogniCodeMcpAdapter::spawn(
        &bin,
        args.root.to_string_lossy().as_ref(),
        "verify-cmd",
        Duration::from_secs(180),
    ) {
        Ok(p) => p,
        Err(e) => return crate::failure(format!("verify: provider spawn failed: {e}")),
    };

    // Observe: find_usages over the subject symbol.
    let symbol = subject_tag
        .rsplit(':')
        .next()
        .unwrap_or(&subject_tag)
        .to_string();
    let basis = sddk_engine::code_intelligence_port::AnalysisBasis {
        provider_build: "cognicode-mcp/verify-cmd".to_string(),
        protocol_major: 2025,
        protocol_minor: 3,
        capability_snapshot: provider.capabilities(),
        analyzer_set_digest: provider.capabilities().analyzer_set_digest,
        source_revision: "verify-cmd".to_string(),
        request_scope: subject_tag.clone(),
    };
    let request = sddk_engine::code_intelligence_port::ScopeRequest {
        added_units: vec![],
        removed_units: vec![],
        modified_units: vec![symbol.clone()],
    };
    let result = match provider.analyze_scope(&basis, &request) {
        Ok(r) => r,
        Err(e) => return crate::failure(format!("verify: provider analyze failed: {e}")),
    };

    // Normalize the provider payload into canonical observations.
    // One observation per usage reference found: Affirms that the
    // subject-unit relation exists.
    let mut set = CanonicalSet::new();
    let mut usage_count = 0usize;
    if let Some(obs) = result.observations.units.get(&symbol)
        && let Ok(payload) = serde_json::from_str::<serde_json::Value>(&obs[0].text)
        && let Some(usages) = payload.get("usages").and_then(|u| u.as_array())
    {
        let input_digest = result.digest.0.clone();
        // Aggregate subject observation: the symbol unit itself,
        // with usage count in the evidence locator. This is the
        // subject the claim names (unit:symbol:<name>).
        let unit_subject = SoftwareUnitRef::new(format!("symbol:{symbol}"));
        let unit_basis = ObservationBasis::for_provider_result("verify-cmd", &input_digest);
        let unit_evidence = EvidenceRef::new(
            sddk_engine::evidence_ref::EvidenceKind::Adhoc,
            format!(
                "cognicode-mcp://find_usages/{symbol}#usages={}",
                usages.len()
            ),
        );
        set.insert(
            sddk_engine::observation::types::SoftwareObservation::declare(
                ObservationSubject::Unit(unit_subject),
                ObservationStance::Affirms,
                unit_evidence,
                ObservationOrigin::StaticProvider,
                unit_basis,
                None,
                "cognicode-mcp",
            ),
        );
        for u in usages {
            let file = u.get("file").and_then(|f| f.as_str()).unwrap_or("?");
            let line = u.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
            let from = SoftwareEntityRef::Unit(SoftwareUnitRef::new(format!("rust:{file}:{line}")));
            let to = SoftwareEntityRef::Unit(SoftwareUnitRef::new(format!("symbol:{symbol}")));
            let relation = SoftwareRelation::new(from, CoreRelationKind::DependsOn, to);
            let _ = &relation;
            let obasis = ObservationBasis::for_provider_result("verify-cmd", &input_digest);
            let evidence = EvidenceRef::new(
                sddk_engine::evidence_ref::EvidenceKind::Adhoc,
                format!("cognicode-mcp://find_usages/{symbol}#{file}:{line}"),
            );
            let observation = sddk_engine::observation::types::SoftwareObservation::declare(
                ObservationSubject::SoftwareRelation(relation),
                ObservationStance::Affirms,
                evidence,
                ObservationOrigin::StaticProvider,
                obasis,
                None,
                "cognicode-mcp",
            );
            set.insert(observation);
            usage_count += 1;
        }
    }
    if usage_count == 0 {
        return crate::failure(
            "verify: provider produced no usable usage observations (empty or malformed payload)"
                .to_string(),
        );
    }

    // Evaluate the claim against the non-empty set.
    let claim = VerificationClaim::StaticProvider(StaticProviderClaim {
        subject_tag: subject_tag.clone(),
        contract_id: args.claim.clone(),
    });
    let domain = StaticProviderDomain;
    let verdict = sddk_engine::verify_kernel::engine::VerifyKernel::evaluate(&claim, &set, &domain);

    let text = format!(
        "verify(static_provider): claim={} subject={}\n  observations: {} (usages found: {})\n  observation_set: {}\n  verdict: {:?}\n",
        args.claim,
        subject_tag,
        set.canonical_digest(),
        usage_count,
        set.observations().len(),
        verdict,
    );
    CommandOutput {
        status: match verdict {
            sddk_engine::verify_kernel::types::VerificationResult::Verified => 0,
            _ => 1,
        },
        stdout: text,
        stderr: String::new(),
    }
}

/// Build the change basis from git diff.
fn build_change_basis_from_git(root: &PathBuf, base: Option<&str>) -> Result<ChangeBasis, String> {
    use std::process::Command;

    // Resolve base revision.
    let base_ref: String = if let Some(b) = base {
        b.to_string()
    } else {
        // Try origin/main first.
        match Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "origin/main"])
            .output()
        {
            Ok(output) if output.status.success() => {
                let ref_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ref_str.is_empty() {
                    ref_str
                } else {
                    // Fallback to HEAD~1
                    fallback_to_head(root)?
                }
            }
            _ => {
                // Fallback to HEAD~1
                fallback_to_head(root)?
            }
        }
    };

    // Get diff.
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", &base_ref, "HEAD"])
        .output()
        .map_err(|e| format!("git diff failed: {}", e))?;

    if !output.status.success() {
        return Err("git diff returned non-zero".to_string());
    }

    let changed_files = String::from_utf8_lossy(&output.stdout);
    let units: Vec<SoftwareUnitRef> = changed_files
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| SoftwareUnitRef::new(l.trim().to_string()))
        .collect();

    Ok(ChangeBasis::new(units).with_base(base_ref))
}

/// Fallback to HEAD~1 when origin/main is not available.
fn fallback_to_head(root: &PathBuf) -> Result<String, String> {
    use std::process::Command;

    Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD~1"])
        .output()
        .map_err(|_| "git unavailable".to_string())
        .and_then(|output| {
            if output.status.success() {
                let ref_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ref_str.is_empty() {
                    Ok(ref_str)
                } else {
                    Err("empty HEAD~1".to_string())
                }
            } else {
                Err("git rev-parse HEAD~1 failed".to_string())
            }
        })
}

/// Exit code mapping: 0 = pass, 1 = fail, 2 = error.
fn exit_code_for_result(result: &sddk_engine::verify_kernel::VerificationResult) -> i32 {
    use sddk_engine::verify_kernel::VerificationResult::*;
    match result {
        Verified => 0,
        Contradicted { .. } | NotApplicable => 0, // NotApplicable is not a failure.
        Unknown { .. } | Stale { .. } => 1,       // Gap or stale = needs attention.
    }
}

/// Format verification result as human-readable text.
fn format_verification_text(
    domain: &str,
    claim: &str,
    result: &sddk_engine::verify_kernel::VerificationResult,
) -> String {
    use sddk_engine::verify_kernel::VerificationResult::*;
    let mut out = String::new();
    out.push_str(&format!("Domain: {}\n", domain));
    out.push_str(&format!("Claim: {}\n", claim));
    out.push_str("Result: ");
    match result {
        Verified => {
            out.push_str("VERIFIED\n");
        }
        Contradicted { reason } => {
            out.push_str(&format!("CONTRADICTED — {}\n", reason));
        }
        Unknown { gap } => {
            out.push_str(&format!("UNKNOWN — {}\n", gap));
        }
        Stale { basis } => {
            out.push_str(&format!("STALE — basis: {}\n", basis));
        }
        NotApplicable => {
            out.push_str("NOT APPLICABLE\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_verified_is_zero() {
        let result = sddk_engine::verify_kernel::VerificationResult::Verified;
        assert_eq!(exit_code_for_result(&result), 0);
    }

    #[test]
    fn exit_code_contradicted_is_zero() {
        let result = sddk_engine::verify_kernel::VerificationResult::Contradicted {
            reason: sddk_engine::verify_kernel::ContradictionReason::OwnershipViolation,
        };
        assert_eq!(exit_code_for_result(&result), 0);
    }

    #[test]
    fn exit_code_unknown_is_one() {
        let result = sddk_engine::verify_kernel::VerificationResult::Unknown {
            gap: sddk_engine::verify_kernel::EvidenceGap::NoEvidenceProvided,
        };
        assert_eq!(exit_code_for_result(&result), 1);
    }

    #[test]
    fn exit_code_stale_is_one() {
        let result = sddk_engine::verify_kernel::VerificationResult::Stale {
            basis: sddk_engine::verify_kernel::BasisHash::SENTINEL,
        };
        assert_eq!(exit_code_for_result(&result), 1);
    }

    #[test]
    fn exit_code_not_applicable_is_zero() {
        let result = sddk_engine::verify_kernel::VerificationResult::NotApplicable;
        assert_eq!(exit_code_for_result(&result), 0);
    }
}
