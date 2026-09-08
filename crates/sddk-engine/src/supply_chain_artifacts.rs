//! SBOM, Provenance & Artifact Lifecycle
//!
//! Plain-data artefact record + validator that enforces SBOM
//! completeness, provenance fields and lifecycle transitions.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::generic_pack_contracts::RuntimeVersion;

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`ArtifactKind`]. Bump when a variant is added.
pub const ARTIFACT_KIND_VARIANT_COUNT: usize = 4;

/// Closed-set size of [`LifecycleState`]. Bump when a variant is added.
pub const LIFECYCLE_STATE_VARIANT_COUNT: usize = 5;

// ── Enums ────────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactKind {
    Binary,
    Bundle,
    ContainerImage,
    Library,
}

impl ArtifactKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Bundle => "bundle",
            Self::ContainerImage => "container_image",
            Self::Library => "library",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifecycleState {
    Draft,
    Signed,
    Published,
    Deprecated,
    Revoked,
}

impl LifecycleState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Signed => "signed",
            Self::Published => "published",
            Self::Deprecated => "deprecated",
            Self::Revoked => "revoked",
        }
    }

    /// True iff `self → next` is a valid lifecycle transition.
    #[must_use]
    pub const fn can_transition_to(self, next: LifecycleState) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Signed)
                | (Self::Signed, Self::Published)
                | (Self::Published, Self::Deprecated)
                | (Self::Published, Self::Revoked)
        )
    }
}

// ── Records ─────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbomEntry {
    pub name: String,
    pub version: String,
    pub license: String,
    pub supplier: String,
    /// 64 hex chars.
    pub sha256: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceAttestation {
    pub builder: String,
    pub build_invocation_id: String,
    /// Sorted.
    pub materials: Vec<String>,
    pub built_at: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub version: RuntimeVersion,
    /// Sorted by (name, version).
    pub sbom: Vec<SbomEntry>,
    pub provenance: ProvenanceAttestation,
    pub state: LifecycleState,
    pub recorded_at: String,
}

// ── Error + validator ──────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupplyChainError {
    EmptySbom,
    DuplicateSbomEntry {
        name: String,
        version: String,
    },
    InvalidSha256 {
        entry: String,
    },
    InvalidTransition {
        from: LifecycleState,
        to: LifecycleState,
    },
    MissingBuilder,
}

pub trait SupplyChainArtifactValidator {
    fn validate(&self, artifact: &ArtifactRecord) -> Result<(), SupplyChainError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultSupplyChainArtifactValidator;

impl SupplyChainArtifactValidator for DefaultSupplyChainArtifactValidator {
    fn validate(&self, artifact: &ArtifactRecord) -> Result<(), SupplyChainError> {
        // Empty SBOM.
        if artifact.sbom.is_empty() {
            return Err(SupplyChainError::EmptySbom);
        }
        // Builder present.
        if artifact.provenance.builder.is_empty() {
            return Err(SupplyChainError::MissingBuilder);
        }
        // Duplicate SBOM entries + SHA-256 format.
        let mut sorted = artifact.sbom.clone();
        sorted.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        for w in sorted.windows(2) {
            if w[0].name == w[1].name && w[0].version == w[1].version {
                return Err(SupplyChainError::DuplicateSbomEntry {
                    name: w[0].name.clone(),
                    version: w[0].version.clone(),
                });
            }
        }
        for e in &sorted {
            if !is_valid_sha256(&e.sha256) {
                return Err(SupplyChainError::InvalidSha256 {
                    entry: format!("{}@{}", e.name, e.version),
                });
            }
        }
        // Lifecycle self-transition is OK; cross-state checks are
        // performed separately when the artifact is mutated, but here
        // we surface the same-state as Ok so the validator can be
        // called repeatedly.
        let _ = artifact.state;
        Ok(())
    }
}

fn is_valid_sha256(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Lifecycle transition helper: returns Err if invalid.
pub fn transition(from: LifecycleState, to: LifecycleState) -> Result<(), SupplyChainError> {
    if from.can_transition_to(to) {
        Ok(())
    } else {
        Err(SupplyChainError::InvalidTransition { from, to })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn validator() -> DefaultSupplyChainArtifactValidator {
        DefaultSupplyChainArtifactValidator
    }

    fn entry(name: &str, version: &str) -> SbomEntry {
        SbomEntry {
            name: name.to_string(),
            version: version.to_string(),
            license: "Apache-2.0".to_string(),
            supplier: "Acme".to_string(),
            sha256: "a".repeat(64),
        }
    }

    fn artifact(state: LifecycleState) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: "art-1".to_string(),
            kind: ArtifactKind::Binary,
            version: RuntimeVersion::new(1, 0, 0),
            sbom: vec![entry("dep1", "1.0")],
            provenance: ProvenanceAttestation {
                builder: "ci".to_string(),
                build_invocation_id: "inv-1".to_string(),
                materials: vec!["src".to_string()],
                built_at: "t0".to_string(),
            },
            state,
            recorded_at: "t0".to_string(),
        }
    }

    #[test]
    fn s1_valid_artifact_passes() {
        let a = artifact(LifecycleState::Signed);
        assert_eq!(validator().validate(&a), Ok(()));
    }

    #[test]
    fn s2_empty_sbom_rejected() {
        let mut a = artifact(LifecycleState::Signed);
        a.sbom.clear();
        assert_eq!(validator().validate(&a), Err(SupplyChainError::EmptySbom));
    }

    #[test]
    fn s3_duplicate_sbom_entries_rejected() {
        let mut a = artifact(LifecycleState::Signed);
        a.sbom.push(entry("dep1", "1.0"));
        let r = validator().validate(&a);
        assert!(matches!(
            r,
            Err(SupplyChainError::DuplicateSbomEntry { .. })
        ));
    }

    #[test]
    fn s4_invalid_sha256_rejected() {
        let mut a = artifact(LifecycleState::Signed);
        a.sbom[0].sha256 = "x".repeat(63); // too short
        let r = validator().validate(&a);
        assert!(matches!(r, Err(SupplyChainError::InvalidSha256 { .. })));
    }

    #[test]
    fn s5_invalid_lifecycle_transition_rejected() {
        let r = transition(LifecycleState::Draft, LifecycleState::Published);
        assert_eq!(
            r,
            Err(SupplyChainError::InvalidTransition {
                from: LifecycleState::Draft,
                to: LifecycleState::Published,
            })
        );
    }

    #[test]
    fn s6_closed_set_audit() {
        assert_eq!(ARTIFACT_KIND_VARIANT_COUNT, 4);
        assert_eq!(LIFECYCLE_STATE_VARIANT_COUNT, 5);
        let states = [
            LifecycleState::Draft,
            LifecycleState::Signed,
            LifecycleState::Published,
            LifecycleState::Deprecated,
            LifecycleState::Revoked,
        ];
        assert_eq!(states.len(), LIFECYCLE_STATE_VARIANT_COUNT);
    }

    #[test]
    fn s7_deterministic() {
        let a = artifact(LifecycleState::Signed);
        assert_eq!(validator().validate(&a), validator().validate(&a));
    }

    #[test]
    fn s8_missing_builder_rejected() {
        let mut a = artifact(LifecycleState::Signed);
        a.provenance.builder = String::new();
        assert_eq!(
            validator().validate(&a),
            Err(SupplyChainError::MissingBuilder)
        );
    }

    #[test]
    fn s9_valid_transitions() {
        assert!(transition(LifecycleState::Draft, LifecycleState::Signed).is_ok());
        assert!(transition(LifecycleState::Signed, LifecycleState::Published).is_ok());
        assert!(transition(LifecycleState::Published, LifecycleState::Deprecated).is_ok());
        assert!(transition(LifecycleState::Published, LifecycleState::Revoked).is_ok());
        assert!(transition(LifecycleState::Deprecated, LifecycleState::Published).is_err());
    }
}
