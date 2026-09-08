//! GA Publication + Compatibility Contract Freeze
//!
//! Terminal cycle of H12 / of the entire 31-cycle roadmap.
//! Publishes the GA artefact and freezes the first stable
//! compatibility contract.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::generic_pack_contracts::RuntimeVersion;
use crate::release_readiness::{ReleaseReadinessReport, ReadinessVerdict};

// ── Audit guard constant ────────────────────────────────────────────────

/// Closed-set size of [`CompatibilityStability`]. Bump when a variant
/// is added.
pub const COMPATIBILITY_STABILITY_VARIANT_COUNT: usize = 3;

// ── Enums + records ────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompatibilityStability {
    Experimental,
    Preview,
    Stable,
}

impl CompatibilityStability {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Preview => "preview",
            Self::Stable => "stable",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StabilityWindow {
    pub stability: CompatibilityStability,
    pub declared_at: String,
    pub from_version: RuntimeVersion,
    pub notes: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityContract {
    pub contract_id: String,
    pub runtime_version: RuntimeVersion,
    pub stability: CompatibilityStability,
    /// Bumped on any breaking change. Must be >= 1 for a GA-stable
    /// contract.
    pub semantics_version: u32,
    /// Caller-supplied.
    pub supported_until: String,
    pub recorded_at: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaPublication {
    pub contract: CompatibilityContract,
    pub readiness: ReleaseReadinessReport,
    pub signed_off_by: String,
    pub published_at: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GaError {
    ReadinessNotPass,
    NotFrozen,
    SignerMissing,
    EmptySigner,
    UnsupportedStabilityForGa,
}

pub trait GaPublisher {
    fn publish(
        &self,
        contract: &CompatibilityContract,
        readiness: &ReleaseReadinessReport,
        signed_off_by: &str,
        published_at: &str,
    ) -> Result<GaPublication, GaError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultGaPublisher;

impl GaPublisher for DefaultGaPublisher {
    fn publish(
        &self,
        contract: &CompatibilityContract,
        readiness: &ReleaseReadinessReport,
        signed_off_by: &str,
        published_at: &str,
    ) -> Result<GaPublication, GaError> {
        if signed_off_by.is_empty() {
            return Err(GaError::EmptySigner);
        }
        if contract.stability != CompatibilityStability::Stable {
            return Err(GaError::UnsupportedStabilityForGa);
        }
        if contract.semantics_version == 0 {
            return Err(GaError::NotFrozen);
        }
        if readiness.overall != ReadinessVerdict::Pass {
            return Err(GaError::ReadinessNotPass);
        }
        Ok(GaPublication {
            contract: contract.clone(),
            readiness: readiness.clone(),
            signed_off_by: signed_off_by.to_string(),
            published_at: published_at.to_string(),
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_readiness::{DefaultReleaseReadinessMatrix, ReleaseReadinessMatrix};

    fn publisher() -> DefaultGaPublisher {
        DefaultGaPublisher
    }

    fn contract() -> CompatibilityContract {
        CompatibilityContract {
            contract_id: "cc-1".to_string(),
            runtime_version: RuntimeVersion::new(1, 0, 0),
            stability: CompatibilityStability::Stable,
            semantics_version: 1,
            supported_until: "2099-01-01".to_string(),
            recorded_at: "t0".to_string(),
        }
    }

    fn pass_readiness() -> ReleaseReadinessReport {
        DefaultReleaseReadinessMatrix.evaluate(
            &RuntimeVersion::new(1, 0, 0),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            "t0",
        )
    }

    #[test]
    fn s1_valid_publication() {
        let r = publisher().publish(&contract(), &pass_readiness(), "alice", "t-issued");
        assert!(r.is_ok());
        let pub_ = r.unwrap();
        assert_eq!(pub_.signed_off_by, "alice");
        assert_eq!(pub_.contract.contract_id, "cc-1");
    }

    #[test]
    fn s2_readiness_not_pass() {
        let mut readiness = pass_readiness();
        readiness.overall = ReadinessVerdict::Fail;
        let r = publisher().publish(&contract(), &readiness, "alice", "t");
        assert_eq!(r, Err(GaError::ReadinessNotPass));
    }

    #[test]
    fn s3_unsupported_stability() {
        let mut c = contract();
        c.stability = CompatibilityStability::Preview;
        let r = publisher().publish(&c, &pass_readiness(), "alice", "t");
        assert_eq!(r, Err(GaError::UnsupportedStabilityForGa));
    }

    #[test]
    fn s4_empty_signer() {
        let r = publisher().publish(&contract(), &pass_readiness(), "", "t");
        assert_eq!(r, Err(GaError::EmptySigner));
    }

    #[test]
    fn s5_closed_set_audit() {
        assert_eq!(COMPATIBILITY_STABILITY_VARIANT_COUNT, 3);
        let all = [
            CompatibilityStability::Experimental,
            CompatibilityStability::Preview,
            CompatibilityStability::Stable,
        ];
        assert_eq!(all.len(), COMPATIBILITY_STABILITY_VARIANT_COUNT);
    }

    #[test]
    fn s6_deterministic() {
        let r1 = publisher().publish(&contract(), &pass_readiness(), "alice", "t-A");
        let r2 = publisher().publish(&contract(), &pass_readiness(), "alice", "t-B");
        let p1 = r1.unwrap();
        let p2 = r2.unwrap();
        assert_eq!(p1.contract, p2.contract);
        assert_ne!(p1.published_at, p2.published_at);
    }

    #[test]
    fn s7_not_frozen_semantics_version_zero() {
        let mut c = contract();
        c.semantics_version = 0;
        let r = publisher().publish(&c, &pass_readiness(), "alice", "t");
        assert_eq!(r, Err(GaError::NotFrozen));
    }
}
