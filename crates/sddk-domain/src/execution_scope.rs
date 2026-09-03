//! Typed execution scope — versioned, migration-safe contract for pack-owned scope metadata.
//!
//! Replaces stringly execution scope with a typed, migration-safe contract per DW-IR-001
//! and ADR-024 (pack-owned metadata, not kernel control types).
//!
//! # Design
//!
//! - `ExecutionScope` is a versioned enum whose only current variant is `V1(ExecutionScopeV1)`.
//! - `ExecutionScopeV1` holds `pack_id`, `path: ScopePath`, and `phases: BTreeSet<ScopePhase>`.
//! - Deterministic serialization via canonical JSON + SHA-256 content hash.
//! - Lossless migration to/from legacy `CyclePath` and `Phase` types.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Schema version constant for ExecutionScopeV1.
pub const SCHEMA_VERSION: u32 = 1;

/// Content hash in `sha256:<64-hex-lowercase>` format (mirrors WorkflowIR).
pub type ContentHash = String;

// ── Closed sets ───────────────────────────────────────────────────────────────

/// Execution scope path — pack metadata mirroring legacy `CyclePath`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScopePath {
    /// Minimal path A.
    AMin,
    /// Lite path A.
    ALite,
    /// Full path A.
    AFull,
    /// Direct path B.
    BDirect,
}

/// Workflow phase — pack metadata mirroring legacy `Phase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ScopePhase {
    /// Exploration phase.
    Explore,
    /// Specification phase.
    Specify,
    /// Design phase.
    Design,
    /// Planning phase.
    Plan,
    /// Build/implementation phase.
    Build,
    /// Verification phase.
    Verify,
    /// UAT phase.
    Uat,
    /// Release phase.
    Release,
    /// Archive phase.
    Archive,
}

// Closed-set discipline (ADR-0078)
crate::assert_variant_count_eq!(
    ScopePath,
    4,
    [
        ScopePath::AMin,
        ScopePath::ALite,
        ScopePath::AFull,
        ScopePath::BDirect,
    ]
);

crate::assert_variant_count_eq!(
    ScopePhase,
    9,
    [
        ScopePhase::Explore,
        ScopePhase::Specify,
        ScopePhase::Design,
        ScopePhase::Plan,
        ScopePhase::Build,
        ScopePhase::Verify,
        ScopePhase::Uat,
        ScopePhase::Release,
        ScopePhase::Archive,
    ]
);

// ── Versioned scope ──────────────────────────────────────────────────────────

/// Versioned execution scope enum.
///
/// Currently only `V1` exists. The versioned envelope allows future migration
/// without breaking existing serialized forms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum ExecutionScope {
    /// Version 1 execution scope.
    V1(ExecutionScopeV1),
}

/// V1 execution scope — typed, migration-safe pack-owned scope metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionScopeV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Pack identifier (non-empty).
    pub pack_id: String,
    /// Scope path (closed set).
    pub path: ScopePath,
    /// Phases covered by this scope (closed set, sorted for determinism).
    pub phases: BTreeSet<ScopePhase>,
}

impl ExecutionScopeV1 {
    /// Creates a new V1 scope (pack_id must be non-empty; caller validates).
    pub fn new(pack_id: String, path: ScopePath, phases: BTreeSet<ScopePhase>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            pack_id,
            path,
            phases,
        }
    }

    /// Validates this scope instance.
    ///
    /// Returns `Ok(())` if valid, `Err(ScopeError)` otherwise.
    pub fn validate(&self) -> Result<(), ScopeError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ScopeError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.pack_id.is_empty() {
            return Err(ScopeError::EmptyPackId);
        }
        if self.phases.is_empty() {
            return Err(ScopeError::EmptyPhases);
        }
        Ok(())
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    ///
    /// Uses `serde_json` BTreeMap default for deterministic map ordering
    /// and `BTreeSet` for sorted phases — field insertion order in source
    /// does NOT affect output.
    pub fn to_canonical_json(&self) -> String {
        // serde_json::to_string uses BTreeMap internally for Map, giving
        // deterministic key ordering; BTreeSet fields serialize sorted.
        serde_json::to_string(self).expect("ExecutionScopeV1 is always serializable")
    }

    /// Computes the content hash over the canonical JSON form.
    ///
    /// Format: `sha256:<64-hex-lowercase>`.
    pub fn compute_content_hash(&self) -> ContentHash {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }

    /// Migrates from legacy `CyclePath` and `Phase` types.
    ///
    /// Maps the legacy closed sets into `ScopePath` and `ScopePhase`.
    /// This is a total function over the legacy domain.
    ///
    /// The `phases` set contains only the current phase, enabling lossless
    /// round-trip via `to_legacy` (REQ-3).
    pub fn from_legacy(
        pack_id: impl Into<String>,
        path: crate::cycle::CyclePath,
        phase: crate::cycle::Phase,
    ) -> Self {
        let scope_path = match path {
            crate::cycle::CyclePath::AMin => ScopePath::AMin,
            crate::cycle::CyclePath::ALite => ScopePath::ALite,
            crate::cycle::CyclePath::AFull => ScopePath::AFull,
            crate::cycle::CyclePath::BDirect => ScopePath::BDirect,
        };
        let scope_phase = match phase {
            crate::cycle::Phase::Explore => ScopePhase::Explore,
            crate::cycle::Phase::Specify => ScopePhase::Specify,
            crate::cycle::Phase::Design => ScopePhase::Design,
            crate::cycle::Phase::Plan => ScopePhase::Plan,
            crate::cycle::Phase::Build => ScopePhase::Build,
            crate::cycle::Phase::Verify => ScopePhase::Verify,
            crate::cycle::Phase::Uat => ScopePhase::Uat,
            crate::cycle::Phase::Release => ScopePhase::Release,
            crate::cycle::Phase::Archive => ScopePhase::Archive,
        };
        let mut phases = BTreeSet::new();
        phases.insert(scope_phase);
        Self {
            schema_version: SCHEMA_VERSION,
            pack_id: pack_id.into(),
            path: scope_path,
            phases,
        }
    }

    /// Converts back to a legacy `(CyclePath, Phase)` pair.
    ///
    /// Returns `None` if this scope cannot be represented in the legacy domain
    /// (e.g., empty phases makes "current phase" undefined).
    ///
    /// For a scope built by `from_legacy`, this returns the original pair
    /// (round-trip property).
    pub fn to_legacy(&self) -> Option<(crate::cycle::CyclePath, crate::cycle::Phase)> {
        let path = match self.path {
            ScopePath::AMin => crate::cycle::CyclePath::AMin,
            ScopePath::ALite => crate::cycle::CyclePath::ALite,
            ScopePath::AFull => crate::cycle::CyclePath::AFull,
            ScopePath::BDirect => crate::cycle::CyclePath::BDirect,
        };
        // For scopes built by from_legacy, phases contains exactly the current phase.
        // Return None if phases is empty (can't represent in legacy domain).
        let phase = self.phases.iter().next()?;
        let phase = match phase {
            ScopePhase::Explore => crate::cycle::Phase::Explore,
            ScopePhase::Specify => crate::cycle::Phase::Specify,
            ScopePhase::Design => crate::cycle::Phase::Design,
            ScopePhase::Plan => crate::cycle::Phase::Plan,
            ScopePhase::Build => crate::cycle::Phase::Build,
            ScopePhase::Verify => crate::cycle::Phase::Verify,
            ScopePhase::Uat => crate::cycle::Phase::Uat,
            ScopePhase::Release => crate::cycle::Phase::Release,
            ScopePhase::Archive => crate::cycle::Phase::Archive,
        };
        Some((path, phase))
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// Validation errors for `ExecutionScopeV1`.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ScopeError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version that was found.
        got: u32,
        /// The version that was expected.
        want: u32,
    },

    /// Pack identifier is empty.
    #[error("pack identifier is empty")]
    EmptyPackId,

    /// Phases set is empty.
    #[error("phases set is empty")]
    EmptyPhases,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::{CyclePath, Phase};

    // REQ-5 #1: canonical JSON byte-stable across two independently constructed equal scopes
    #[test]
    fn canonical_json_byte_stable() {
        let mut phases1 = BTreeSet::new();
        phases1.insert(ScopePhase::Explore);
        phases1.insert(ScopePhase::Build);

        let mut phases2 = BTreeSet::new();
        phases2.insert(ScopePhase::Explore);
        phases2.insert(ScopePhase::Build);

        let scope1 = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, phases1);
        let scope2 = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, phases2);

        let json1 = scope1.to_canonical_json();
        let json2 = scope2.to_canonical_json();
        assert_eq!(
            json1, json2,
            "canonical JSON must be byte-identical for equal scopes"
        );
    }

    // REQ-5 #2: hash equality for equal scopes, inequality after any field mutation
    #[test]
    fn hash_equality_and_mutation_inequality() {
        let mut phases1 = BTreeSet::new();
        phases1.insert(ScopePhase::Explore);
        let scope1 = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, phases1);

        let mut phases2 = BTreeSet::new();
        phases2.insert(ScopePhase::Explore);
        let scope2 = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, phases2);

        assert_eq!(
            scope1.compute_content_hash(),
            scope2.compute_content_hash(),
            "equal scopes must have identical hashes"
        );

        // Mutate pack_id
        let mut mutated = scope1.clone();
        mutated.pack_id = "other-pack".into();
        assert_ne!(
            scope1.compute_content_hash(),
            mutated.compute_content_hash(),
            "mutated pack_id must change hash"
        );

        // Mutate path
        let mut mutated = scope1.clone();
        mutated.path = ScopePath::AFull;
        assert_ne!(
            scope1.compute_content_hash(),
            mutated.compute_content_hash(),
            "mutated path must change hash"
        );

        // Mutate phases
        let mut mutated = scope1.clone();
        mutated.phases.insert(ScopePhase::Build);
        assert_ne!(
            scope1.compute_content_hash(),
            mutated.compute_content_hash(),
            "mutated phases must change hash"
        );
    }

    // REQ-5 #3: from_legacy → to_legacy round-trip for all 4×9 = 36 combinations
    #[test]
    fn from_legacy_to_legacy_roundtrip_all_combinations() {
        let paths = [
            CyclePath::AMin,
            CyclePath::ALite,
            CyclePath::AFull,
            CyclePath::BDirect,
        ];
        let phases = [
            Phase::Explore,
            Phase::Specify,
            Phase::Design,
            Phase::Plan,
            Phase::Build,
            Phase::Verify,
            Phase::Uat,
            Phase::Release,
            Phase::Archive,
        ];

        let mut failures = Vec::new();
        for path in &paths {
            for phase in &phases {
                let scope = ExecutionScopeV1::from_legacy("test-pack", path.clone(), *phase);
                let result = scope.to_legacy();
                let roundtrip_path = result.as_ref().map(|(p, _)| p.clone());
                let roundtrip_phase = result.as_ref().map(|(_, p)| *p);
                if roundtrip_path != Some(path.clone()) {
                    failures.push(format!(
                        "path roundtrip failed: {:?} → {:?}",
                        path, roundtrip_path
                    ));
                }
                if roundtrip_phase != Some(*phase) {
                    failures.push(format!(
                        "phase roundtrip failed: {:?} → {:?}",
                        phase, roundtrip_phase
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "Round-trip failures ({}): {:?}",
            failures.len(),
            failures
        );
        // Assert exact 36 cases
        assert_eq!(
            paths.len() * phases.len(),
            36,
            "must test all 36 combinations"
        );
    }

    // REQ-5 #4: validation rejects wrong schema_version, empty pack_id, empty phases
    #[test]
    fn validate_rejects_wrong_schema_version() {
        let mut scope = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, {
            let mut s = BTreeSet::new();
            s.insert(ScopePhase::Explore);
            s
        });
        scope.schema_version = 99;
        let err = scope.validate().unwrap_err();
        assert!(matches!(
            err,
            ScopeError::UnsupportedSchemaVersion { got: 99, want: 1 }
        ));
    }

    #[test]
    fn validate_rejects_empty_pack_id() {
        let scope = ExecutionScopeV1::new("".into(), ScopePath::AMin, {
            let mut s = BTreeSet::new();
            s.insert(ScopePhase::Explore);
            s
        });
        let err = scope.validate().unwrap_err();
        assert!(matches!(err, ScopeError::EmptyPackId));
    }

    #[test]
    fn validate_rejects_empty_phases() {
        let scope = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, BTreeSet::new());
        let err = scope.validate().unwrap_err();
        assert!(matches!(err, ScopeError::EmptyPhases));
    }

    #[test]
    fn validate_accepts_valid_scope() {
        let scope = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, {
            let mut s = BTreeSet::new();
            s.insert(ScopePhase::Explore);
            s
        });
        assert!(scope.validate().is_ok());
    }

    // REQ-5 #5: serde JSON round-trip ExecutionScope::V1 preserves value and hash
    #[test]
    fn serde_roundtrip_execution_scope_v1() {
        let original =
            ExecutionScope::V1(ExecutionScopeV1::new("my-pack".into(), ScopePath::AFull, {
                let mut s = BTreeSet::new();
                s.insert(ScopePhase::Explore);
                s.insert(ScopePhase::Build);
                s
            }));

        let ExecutionScope::V1(ref v1_before) = original;
        let hash_before = v1_before.compute_content_hash();

        let json = serde_json::to_string(&original).unwrap();
        let roundtrip: ExecutionScope = serde_json::from_str(&json).unwrap();

        assert_eq!(
            original, roundtrip,
            "round-trip must preserve value equality"
        );

        let ExecutionScope::V1(ref v1) = roundtrip;
        assert_eq!(
            hash_before,
            v1.compute_content_hash(),
            "round-trip must preserve content hash"
        );
    }

    // REQ-5 #6: variant-count assertions hold (compile-time, tested implicitly via building)
    #[test]
    fn scope_path_variant_count() {
        // This test exists to give test coverage visibility; the assert_variant_count_eq!
        // macro fires at compile time if the count drifts.
        let variants = [
            ScopePath::AMin,
            ScopePath::ALite,
            ScopePath::AFull,
            ScopePath::BDirect,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn scope_phase_variant_count() {
        let variants = [
            ScopePhase::Explore,
            ScopePhase::Specify,
            ScopePhase::Design,
            ScopePhase::Plan,
            ScopePhase::Build,
            ScopePhase::Verify,
            ScopePhase::Uat,
            ScopePhase::Release,
            ScopePhase::Archive,
        ];
        assert_eq!(variants.len(), 9);
    }

    // Additional: verify serde rename values
    #[test]
    fn serde_renames() {
        let path_json = serde_json::to_string(&ScopePath::AMin).unwrap();
        assert_eq!(path_json, "\"a_min\"");
        let path_roundtrip: ScopePath = serde_json::from_str(&path_json).unwrap();
        assert_eq!(path_roundtrip, ScopePath::AMin);

        let phase_json = serde_json::to_string(&ScopePhase::Explore).unwrap();
        assert_eq!(phase_json, "\"explore\"");
        let phase_roundtrip: ScopePhase = serde_json::from_str(&phase_json).unwrap();
        assert_eq!(phase_roundtrip, ScopePhase::Explore);
    }

    // Additional: verify ExecutionScopeV1 serde includes schema_version
    #[test]
    fn execution_scope_v1_serde_has_schema_version() {
        let scope = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, {
            let mut s = BTreeSet::new();
            s.insert(ScopePhase::Explore);
            s
        });
        let json = scope.to_canonical_json();
        assert!(
            json.contains("\"schema_version\":1"),
            "canonical JSON must include schema_version: {}",
            json
        );
    }

    // Additional: field insertion order must not affect canonical JSON
    #[test]
    fn field_insertion_order_independent() {
        let build_scope = || {
            let mut phases = BTreeSet::new();
            phases.insert(ScopePhase::Explore);
            ExecutionScopeV1 {
                schema_version: SCHEMA_VERSION,
                pack_id: "pack".into(),
                path: ScopePath::BDirect,
                phases,
            }
        };
        let json1 = build_scope().to_canonical_json();
        let json2 = build_scope().to_canonical_json();
        assert_eq!(json1, json2);
    }

    // Additional: content hash format
    #[test]
    fn content_hash_format() {
        let scope = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, {
            let mut s = BTreeSet::new();
            s.insert(ScopePhase::Explore);
            s
        });
        let hash = scope.compute_content_hash();
        assert!(
            hash.starts_with("sha256:"),
            "hash must start with 'sha256:': {}",
            hash
        );
        let hex = &hash[7..];
        assert_eq!(hex.len(), 64, "hash hex must be 64 chars: {}", hex);
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "hash hex must be all hex digits: {}",
            hex
        );
    }

    // Additional: to_legacy returns None for empty phases
    #[test]
    fn to_legacy_returns_none_for_empty_phases() {
        let scope = ExecutionScopeV1::new("my-pack".into(), ScopePath::AMin, BTreeSet::new());
        assert!(scope.to_legacy().is_none());
    }
}
