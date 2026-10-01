//! External-provider test semantics: **absence is never PASS**.
//!
//! A test that certifies integration with an external provider (Chronos,
//! CogniCode, and any future adapter) can be skipped for exactly two honest
//! reasons: the dependency is absent, or it is not applicable. Both are
//! *outcomes*, not passes.
//!
//! Before this module, `aiw_s5_chronos_real.rs` used the pattern
//!
//! ```ignore
//! let bin = match std::env::var("CHRONOS_MCP_BIN").ok() {
//!     Some(b) => b,
//!     None => return,        // ← the test reports "ok" having run nothing
//! };
//! ```
//!
//! which made `cargo test` print `3 passed` while two of the three tests never
//! executed — a false green of exactly the kind the acceptance-truthfulness
//! work exists to remove. The in-repo convention already existed
//! (`aiw_s1_cognicode_real.rs` uses `#[ignore]` + `expect`), so this module
//! generalises the *state model* rather than inventing a rule.
//!
//! ## The five states
//!
//! An external-provider test resolves to exactly one of these, and the
//! ordinary test run must never report `PassObserved` without a real
//! observation:
//!
//! | State | Meaning | Ordinary run reports |
//! |---|---|---|
//! | [`ExtOutcome::PassObserved`] | The provider was resolved and the contract held | pass |
//! | [`ExtOutcome::FailObserved`] | The provider was resolved and the contract broke | **fail** |
//! | [`ExtOutcome::BlockedExternalDependency`] | The provider binary is absent or unusable | ignored |
//! | [`ExtOutcome::NotRun`] | The EXT profile was not requested | ignored |
//! | [`ExtOutcome::NotApplicable`] | The dependency does not apply to this build | ignored |
//!
//! `FailObserved` is the load-bearing row: it is the only way a resolved
//! provider can turn a test red.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The closed set of honest outcomes for an external-provider test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum ExtOutcome {
    /// The provider resolved and the contract held.
    PassObserved {
        /// Fingerprint of what was observed (binary digest, event counts…).
        observation: String,
    },
    /// The provider resolved and the contract broke.
    FailObserved {
        /// What broke.
        reason: String,
    },
    /// The external dependency is absent or unusable. Never a pass.
    BlockedExternalDependency {
        /// Env var that names the binary.
        env_var: String,
        /// What was found (usually the reason it could not be used).
        found: String,
    },
    /// The EXT profile was not requested at all.
    NotRun {
        /// Why the profile was not requested.
        reason: String,
    },
    /// The dependency does not apply to this build or platform.
    NotApplicable {
        /// Why it does not apply.
        reason: String,
    },
}

impl ExtOutcome {
    /// Stable wire string. The EXT launcher script (`tests/ext_provider_gate.sh`)
    /// emits these exact values; `ext_outcome_states_match_the_launcher_contract`
    /// pins the two sides together.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtOutcome::PassObserved { .. } => "pass_observed",
            ExtOutcome::FailObserved { .. } => "fail_observed",
            ExtOutcome::BlockedExternalDependency { .. } => "blocked_external_dependency",
            ExtOutcome::NotRun { .. } => "not_run",
            ExtOutcome::NotApplicable { .. } => "not_applicable",
        }
    }

    /// Whether the ordinary test run may report this as **ignored** rather
    /// than failed. Every non-observing state qualifies; `FailObserved` and
    /// `PassObserved` do not.
    pub fn is_ignored_in_ordinary_run(&self) -> bool {
        matches!(
            self,
            ExtOutcome::BlockedExternalDependency { .. }
                | ExtOutcome::NotRun { .. }
                | ExtOutcome::NotApplicable { .. }
        )
    }

    /// Whether this outcome may ever be reported as a passing test.
    ///
    /// Only [`ExtOutcome::PassObserved`] may. The other four are explicitly
    /// not passes — this is the whole point of the module.
    pub fn is_pass(&self) -> bool {
        matches!(self, ExtOutcome::PassObserved { .. })
    }

    /// Every state name, for contract tests and receipt schemas.
    pub fn all_state_names() -> [&'static str; 5] {
        [
            "pass_observed",
            "fail_observed",
            "blocked_external_dependency",
            "not_run",
            "not_applicable",
        ]
    }
}

/// Identity of an external provider binary, recorded before any test runs.
///
/// The receipt exists so that a `PassObserved` can be traced to *which* bytes
/// produced it. A pass without this fingerprint is not evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtProviderReceipt {
    /// Provider name as the adapter knows it (e.g. `chronos-mcp`).
    pub provider: String,
    /// Env var that named the binary.
    pub env_var: String,
    /// Resolved absolute path.
    pub path: PathBuf,
    /// `sha256:<hex>` of the binary, when it could be read.
    pub sha256: Option<String>,
    /// Version string reported by the binary, when it reported one.
    pub version: Option<String>,
    /// Capabilities the provider advertised, when it advertised any.
    pub capabilities: Vec<String>,
    /// How the provider was resolved.
    pub resolution: ExtResolution,
}

impl ExtProviderReceipt {
    /// A one-line human description of what was found, for assertion messages.
    pub fn found_description(&self) -> String {
        if self.path.as_os_str().is_empty() {
            format!("no path resolved for {}", self.provider)
        } else {
            format!(
                "{} ({}, sha256={})",
                self.path.display(),
                match self.resolution {
                    ExtResolution::EnvVarResolved => "from env var",
                    ExtResolution::PathResolved => "from PATH",
                    ExtResolution::Absent => "absent",
                },
                self.sha256.as_deref().unwrap_or("none")
            )
        }
    }
}

/// How an external provider binary was located.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtResolution {
    /// Named by its env var and present on disk.
    EnvVarResolved,
    /// Found on `PATH` without an env var.
    PathResolved,
    /// Not found.
    Absent,
}

/// Resolves an external provider binary into a receipt plus an outcome.
///
/// Never returns `PassObserved`: resolution is a precondition, not an
/// observation. The caller must still run the contract and decide.
pub fn resolve_provider(provider: &str, env_var: &str) -> (ExtProviderReceipt, ExtOutcome) {
    let found = std::env::var(env_var).ok().filter(|s| !s.is_empty());
    let path = match found {
        Some(p) => PathBuf::from(p),
        None => {
            // Fall back to PATH resolution: an operator who installed the
            // provider should not have to also name it.
            match which(provider) {
                Some(p) => p,
                None => {
                    let receipt = ExtProviderReceipt {
                        provider: provider.to_owned(),
                        env_var: env_var.to_owned(),
                        path: PathBuf::new(),
                        sha256: None,
                        version: None,
                        capabilities: Vec::new(),
                        resolution: ExtResolution::Absent,
                    };
                    return (
                        receipt,
                        ExtOutcome::BlockedExternalDependency {
                            env_var: env_var.to_owned(),
                            found: "not on PATH and env var unset or empty".to_owned(),
                        },
                    );
                }
            }
        }
    };

    if !path.is_file() {
        let receipt = ExtProviderReceipt {
            provider: provider.to_owned(),
            env_var: env_var.to_owned(),
            path: path.clone(),
            sha256: None,
            version: None,
            capabilities: Vec::new(),
            resolution: ExtResolution::Absent,
        };
        return (
            receipt,
            ExtOutcome::BlockedExternalDependency {
                env_var: env_var.to_owned(),
                found: format!("{} is not a readable file", path.display()),
            },
        );
    }

    let resolution = if std::env::var(env_var).is_ok() {
        ExtResolution::EnvVarResolved
    } else {
        ExtResolution::PathResolved
    };
    let receipt = ExtProviderReceipt {
        provider: provider.to_owned(),
        env_var: env_var.to_owned(),
        sha256: sha256_of(&path),
        path,
        version: None,
        capabilities: Vec::new(),
        resolution,
    };
    // Resolved but not yet observed.
    (
        receipt,
        ExtOutcome::NotRun {
            reason: "binary resolved; run the EXT profile to observe".to_owned(),
        },
    )
}

/// `sha256:<hex>` of a file, or `None` when it cannot be read.
pub fn sha256_of(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(&bytes);
    Some(format!("sha256:{digest:x}"))
}

fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pass_observed_is_a_pass() {
        assert!(
            ExtOutcome::PassObserved {
                observation: "x".into()
            }
            .is_pass()
        );
        for not_pass in [
            ExtOutcome::FailObserved { reason: "x".into() },
            ExtOutcome::BlockedExternalDependency {
                env_var: "E".into(),
                found: "x".into(),
            },
            ExtOutcome::NotRun { reason: "x".into() },
            ExtOutcome::NotApplicable { reason: "x".into() },
        ] {
            assert!(!not_pass.is_pass(), "{not_pass:?} must never be a pass");
        }
    }

    #[test]
    fn every_non_observed_state_is_ignored_in_the_ordinary_run() {
        assert!(
            ExtOutcome::BlockedExternalDependency {
                env_var: "E".into(),
                found: "x".into()
            }
            .is_ignored_in_ordinary_run()
        );
        assert!(ExtOutcome::NotRun { reason: "x".into() }.is_ignored_in_ordinary_run());
        assert!(
            !ExtOutcome::PassObserved {
                observation: "x".into()
            }
            .is_ignored_in_ordinary_run()
        );
    }

    /// The launcher script and this enum are two halves of one contract.
    /// If a state is renamed on one side, this fails.
    #[test]
    fn ext_outcome_states_match_the_launcher_contract() {
        for name in ExtOutcome::all_state_names() {
            let on_disk = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/ext_provider_gate.sh");
            let script = std::fs::read_to_string(&on_disk)
                .expect("tests/ext_provider_gate.sh must exist and be readable");
            assert!(
                script.contains(name),
                "launcher does not know the state `{name}`"
            );
        }
    }

    /// A provider named by an env var pointing at nothing is BLOCKED, and the
    /// receipt says so — never a silent success.
    #[test]
    fn absent_provider_resolves_to_blocked_not_to_pass() {
        let (receipt, outcome) = resolve_provider(
            "sddk-no-such-provider-for-tests",
            "SDDK_NO_SUCH_PROVIDER_BIN",
        );
        assert_eq!(receipt.resolution, ExtResolution::Absent);
        assert!(matches!(
            outcome,
            ExtOutcome::BlockedExternalDependency { .. }
        ));
        assert!(!outcome.is_pass());
        assert!(outcome.is_ignored_in_ordinary_run());
        assert!(receipt.sha256.is_none(), "an absent binary has no digest");
    }
}
