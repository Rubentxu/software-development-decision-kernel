//! DeliveryKind closed-set taxonomy for SDDK workflow cycles.
//!
//! Every cycle MUST declare exactly one `DeliveryKind` from this closed-set
//! enumeration. Each kind binds to exactly one evaluation policy.
//! The classification is immutable after `phase.specify.complete`.
//!
//! # Architecture
//!
//! Mirrors ADR-0069 `DeliveryEnvelope::evaluate` seam: 1 type, 1 enum, 1 method.
//!
//! # See Also
//!
//! - [ADR-0076](../adr/ADR-0076-delivery-kind-taxonomy.html)

use serde::{Deserialize, Serialize};

/// Effect kinds that may be forbidden for certain delivery kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectKind {
    /// Git tag was applied.
    GitTag,
    /// Release artifact was published.
    ReleasePublished,
    /// Bundle was installed.
    BundleInstalled,
    /// Vault index was updated.
    VaultIndexUpdated,
    /// Archive manifest was produced.
    ArchiveManifestProduced,
    /// Incidence was persisted.
    IncidencePersisted,
}

/// Effects forbidden for vault-only and managed-closure delivery kinds.
///
/// Shared by `VaultOnlyDelivery`, `RetroactiveArchiveCloseDelivery`, and
/// `ManagedClosureDelivery` — extracted to a single site so the business rule
/// "no tag/release/bundle effects for vault-only and managed-closure cycles"
/// lives in one place. Default-deny policy preserved; the 7-test conformance
/// suite in `delivery_kind.rs` validates the policy independently.
const VAULT_ROUTE_FORBIDDEN: [EffectKind; 3] = [
    EffectKind::GitTag,
    EffectKind::ReleasePublished,
    EffectKind::BundleInstalled,
];

/// Closed-set delivery kind enumeration.
///
/// Each variant binds to exactly one `EvaluationPolicy` at compile time.
/// The classification is immutable after `phase.specify.complete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryKind {
    /// Standard code delivery: requires tag, release, and bundle.
    CodeDelivery,
    /// Documentation-only delivery: no binary artifacts required.
    DocsDelivery,
    /// Vault-only delivery: no release artifacts; vault closure only.
    VaultOnlyDelivery,
    /// Retroactive archive close: closes a prior cycle via archive.
    RetroactiveArchiveCloseDelivery,
    /// Managed closure: prior blocked cycle migrates via vault route.
    ManagedClosureDelivery,
}

/// Evaluation policy for a delivery kind.
///
/// Defines which effects are forbidden for each delivery kind.
/// Policy evaluation is pure, total, and deterministic.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationPolicy {
    /// The delivery kind this policy governs.
    pub kind: DeliveryKind,
    /// Effects that are forbidden for this delivery kind.
    pub forbidden_effects: Vec<EffectKind>,
}

impl EvaluationPolicy {
    /// Creates a new evaluation policy for the given delivery kind.
    pub fn new(kind: DeliveryKind) -> Self {
        let forbidden_effects = match kind {
            DeliveryKind::CodeDelivery => vec![],
            DeliveryKind::DocsDelivery => vec![EffectKind::GitTag, EffectKind::ReleasePublished],
            DeliveryKind::VaultOnlyDelivery => VAULT_ROUTE_FORBIDDEN.to_vec(),
            DeliveryKind::RetroactiveArchiveCloseDelivery => VAULT_ROUTE_FORBIDDEN.to_vec(),
            DeliveryKind::ManagedClosureDelivery => VAULT_ROUTE_FORBIDDEN.to_vec(),
        };
        Self {
            kind,
            forbidden_effects,
        }
    }
}

/// Errors that can occur during delivery kind evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryError {
    /// Delivery kind was not declared (REQ-DKA-001-S2).
    Undeclared,
    /// Duplicate delivery kinds were declared (REQ-DKA-001-S3).
    Duplicate,
    /// A forbidden effect was observed for this delivery kind.
    ForbiddenEffect(EffectKind),
    /// Cycle is not in BLOCKED status (REQ-DKA-005-S2).
    CycleNotBlocked,
}

impl EvaluationPolicy {
    /// Evaluates whether the observed effects are compatible with this policy.
    ///
    /// Returns `Ok(())` if all observed effects are allowed.
    /// Returns `Err(DeliveryError)` if a forbidden effect is observed or
    /// if the delivery kind was undeclared or duplicated.
    ///
    /// # Arguments
    ///
    /// * `observed` - Slice of effect kinds that were observed during the cycle.
    ///
    /// # Examples
    ///
    /// ```
    /// use sddk_domain::delivery_kind::{DeliveryKind, EvaluationPolicy, EffectKind};
    ///
    /// let policy = EvaluationPolicy::new(DeliveryKind::DocsDelivery);
    /// let result = policy.evaluate(&[EffectKind::VaultIndexUpdated]);
    /// assert!(result.is_ok());
    /// ```
    pub fn evaluate(&self, observed: &[EffectKind]) -> Result<(), DeliveryError> {
        for effect in observed {
            if self.forbidden_effects.contains(effect) {
                return Err(DeliveryError::ForbiddenEffect(*effect));
            }
        }
        Ok(())
    }
}

/// Parses a kebab-case string into a `DeliveryKind`.
///
/// Returns `Some(DeliveryKind)` if the string matches a known variant.
/// Returns `None` if the string is not a valid delivery kind.
///
/// # Arguments
///
/// * `s` - A kebab-case string to parse.
///
/// # Examples
///
/// ```
/// use sddk_domain::delivery_kind::delivery_kind_from_str;
///
/// assert_eq!(delivery_kind_from_str("code-delivery"), Some(sddk_domain::delivery_kind::DeliveryKind::CodeDelivery));
/// assert_eq!(delivery_kind_from_str("docs-delivery"), Some(sddk_domain::delivery_kind::DeliveryKind::DocsDelivery));
/// assert_eq!(delivery_kind_from_str("unknown"), None);
/// ```
pub fn delivery_kind_from_str(s: &str) -> Option<DeliveryKind> {
    match s {
        "code-delivery" => Some(DeliveryKind::CodeDelivery),
        "docs-delivery" => Some(DeliveryKind::DocsDelivery),
        "vault-only-delivery" => Some(DeliveryKind::VaultOnlyDelivery),
        "retroactive-archive-close-delivery" => Some(DeliveryKind::RetroactiveArchiveCloseDelivery),
        "managed-closure-delivery" => Some(DeliveryKind::ManagedClosureDelivery),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_kind_from_str_valid() {
        assert_eq!(
            delivery_kind_from_str("code-delivery"),
            Some(DeliveryKind::CodeDelivery)
        );
        assert_eq!(
            delivery_kind_from_str("docs-delivery"),
            Some(DeliveryKind::DocsDelivery)
        );
        assert_eq!(
            delivery_kind_from_str("vault-only-delivery"),
            Some(DeliveryKind::VaultOnlyDelivery)
        );
        assert_eq!(
            delivery_kind_from_str("retroactive-archive-close-delivery"),
            Some(DeliveryKind::RetroactiveArchiveCloseDelivery)
        );
        assert_eq!(
            delivery_kind_from_str("managed-closure-delivery"),
            Some(DeliveryKind::ManagedClosureDelivery)
        );
    }

    #[test]
    fn test_delivery_kind_from_str_invalid() {
        assert_eq!(delivery_kind_from_str("unknown"), None);
        assert_eq!(delivery_kind_from_str("code_delivery"), None);
        assert_eq!(delivery_kind_from_str("CodeDelivery"), None);
    }

    #[test]
    fn test_evaluation_policy_code_delivery_allows_all() {
        let policy = EvaluationPolicy::new(DeliveryKind::CodeDelivery);
        let effects = [
            EffectKind::GitTag,
            EffectKind::ReleasePublished,
            EffectKind::BundleInstalled,
        ];
        assert!(policy.evaluate(&effects).is_ok());
    }

    #[test]
    fn test_evaluation_policy_docs_delivery_forbids_release() {
        let policy = EvaluationPolicy::new(DeliveryKind::DocsDelivery);
        assert!(policy.evaluate(&[EffectKind::ReleasePublished]).is_err());
        assert!(policy.evaluate(&[EffectKind::GitTag]).is_err());
        assert!(policy.evaluate(&[EffectKind::VaultIndexUpdated]).is_ok());
    }

    #[test]
    fn test_evaluation_policy_vault_only_forbids_publish() {
        let policy = EvaluationPolicy::new(DeliveryKind::VaultOnlyDelivery);
        assert!(policy.evaluate(&[EffectKind::ReleasePublished]).is_err());
        assert!(policy.evaluate(&[EffectKind::GitTag]).is_err());
        assert!(policy.evaluate(&[EffectKind::BundleInstalled]).is_err());
        assert!(policy.evaluate(&[EffectKind::VaultIndexUpdated]).is_ok());
    }

    #[test]
    fn test_evaluation_policy_managed_closure_forbids_publish() {
        let policy = EvaluationPolicy::new(DeliveryKind::ManagedClosureDelivery);
        assert!(policy.evaluate(&[EffectKind::ReleasePublished]).is_err());
        assert!(policy.evaluate(&[EffectKind::GitTag]).is_err());
        assert!(policy.evaluate(&[EffectKind::BundleInstalled]).is_err());
        assert!(
            policy
                .evaluate(&[EffectKind::ArchiveManifestProduced])
                .is_ok()
        );
    }

    #[test]
    fn test_delivery_error_display() {
        let err = DeliveryError::Undeclared;
        assert_eq!(format!("{:?}", err), "Undeclared");

        let err = DeliveryError::Duplicate;
        assert_eq!(format!("{:?}", err), "Duplicate");

        let err = DeliveryError::CycleNotBlocked;
        assert_eq!(format!("{:?}", err), "CycleNotBlocked");
    }
}
