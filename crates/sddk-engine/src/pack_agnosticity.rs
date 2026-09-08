//! Pack-Agnosticity Proof
//!
//! Static + cross-pack checks that prove no kernel module carries
//! pack-kind-specific special cases.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::generic_pack_contracts::{PackManifest, RuntimeVersion};

// ── Audit guard constant ────────────────────────────────────────────────

/// Closed-set size of [`AgnosticityViolation`]. Bump when a variant
/// is added.
pub const AGNOSTICITY_VIOLATION_VARIANT_COUNT: usize = 3;

// ── Enums + records ────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgnosticityViolation {
    PackKindSpecificImport { module: String, offender: String },
    DivergentRuntimeRange { pack_a: String, pack_b: String },
    DivergentManifestSchemaRef { pack_a: String, pack_b: String },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgnosticityProof {
    pub modules_scanned: usize,
    /// Sorted.
    pub forbidden_substrings: Vec<String>,
    /// Sorted by `(module, offender)`.
    pub violations: Vec<AgnosticityViolation>,
    pub proofs_collected_at: String,
}

// ── Trait + default auditor ────────────────────────────────────────────

pub trait AgnosticityAuditor {
    fn audit(
        &self,
        module_bodies: &[(String, String)],
        forbidden_substrings: &[String],
        recorded_at: &str,
    ) -> AgnosticityProof;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultAgnosticityAuditor;

impl AgnosticityAuditor for DefaultAgnosticityAuditor {
    fn audit(
        &self,
        module_bodies: &[(String, String)],
        forbidden_substrings: &[String],
        recorded_at: &str,
    ) -> AgnosticityProof {
        let mut violations: Vec<AgnosticityViolation> = Vec::new();
        let modules_scanned = module_bodies.len();
        for (module, body) in module_bodies {
            for forbidden in forbidden_substrings {
                if body.contains(forbidden) {
                    violations.push(AgnosticityViolation::PackKindSpecificImport {
                        module: module.clone(),
                        offender: forbidden.clone(),
                    });
                }
            }
        }
        violations.sort_by(|a, b| match (a, b) {
            (
                AgnosticityViolation::PackKindSpecificImport {
                    module: m1,
                    offender: o1,
                },
                AgnosticityViolation::PackKindSpecificImport {
                    module: m2,
                    offender: o2,
                },
            ) => m1.cmp(m2).then(o1.cmp(o2)),
            _ => core::cmp::Ordering::Equal,
        });
        let mut forbidden_sorted = forbidden_substrings.to_vec();
        forbidden_sorted.sort();

        AgnosticityProof {
            modules_scanned,
            forbidden_substrings: forbidden_sorted,
            violations,
            proofs_collected_at: recorded_at.to_string(),
        }
    }
}

// ── Cross-pack proof ───────────────────────────────────────────────────

/// Result of the cross-pack runtime / schema_ref check.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPackProof {
    pub verified: bool,
    pub violations: Vec<AgnosticityViolation>,
}

pub trait MultiPackProofRunner {
    fn prove(&self, pack_a: &PackManifest, pack_b: &PackManifest) -> MultiPackProof;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultMultiPackProofRunner;

impl MultiPackProofRunner for DefaultMultiPackProofRunner {
    fn prove(&self, pack_a: &PackManifest, pack_b: &PackManifest) -> MultiPackProof {
        let mut violations: Vec<AgnosticityViolation> = Vec::new();
        // Runtime overlap check.
        let lo = max_version(&pack_a.min_runtime_version, &pack_b.min_runtime_version);
        let hi = min_version(&pack_a.max_runtime_version, &pack_b.max_runtime_version);
        if lo > hi {
            violations.push(AgnosticityViolation::DivergentRuntimeRange {
                pack_a: pack_a.pack_id.clone(),
                pack_b: pack_b.pack_id.clone(),
            });
        }
        // Schema ref check.
        if pack_a.schema_ref != pack_b.schema_ref {
            violations.push(AgnosticityViolation::DivergentManifestSchemaRef {
                pack_a: pack_a.pack_id.clone(),
                pack_b: pack_b.pack_id.clone(),
            });
        }
        MultiPackProof {
            verified: violations.is_empty(),
            violations,
        }
    }
}

fn max_version(a: &RuntimeVersion, b: &RuntimeVersion) -> RuntimeVersion {
    if a >= b { a.clone() } else { b.clone() }
}
fn min_version(a: &RuntimeVersion, b: &RuntimeVersion) -> RuntimeVersion {
    if a <= b { a.clone() } else { b.clone() }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_pack_contracts::{EvidenceFormat, PackKind};

    fn auditor() -> DefaultAgnosticityAuditor {
        DefaultAgnosticityAuditor
    }

    fn runner() -> DefaultMultiPackProofRunner {
        DefaultMultiPackProofRunner
    }

    fn manifest(
        id: &str,
        min: (u32, u32, u32),
        max: (u32, u32, u32),
        schema: &str,
    ) -> PackManifest {
        PackManifest {
            pack_id: id.to_string(),
            kind: PackKind::Uat,
            evidence_format: EvidenceFormat::JsonSchemaDraft07,
            min_runtime_version: RuntimeVersion::new(min.0, min.1, min.2),
            max_runtime_version: RuntimeVersion::new(max.0, max.1, max.2),
            schema_ref: schema.to_string(),
            recorded_at: "t0".to_string(),
        }
    }

    // ── S-1: no forbidden substrings ─────────────────────────────────

    #[test]
    fn s1_no_violations_when_clean() {
        let bodies = vec![("mod_a".to_string(), "fn ok() {}".to_string())];
        let forbidden = vec!["PackKind::Uat".to_string()];
        let p = auditor().audit(&bodies, &forbidden, "t0");
        assert_eq!(p.modules_scanned, 1);
        assert!(p.violations.is_empty());
    }

    // ── S-2: forbidden substring detected ─────────────────────────────

    #[test]
    fn s2_violation_on_forbidden_substring() {
        let bodies = vec![(
            "mod_a".to_string(),
            "fn bad() { if x == PackKind::Uat {} }".to_string(),
        )];
        let forbidden = vec!["PackKind::Uat".to_string()];
        let p = auditor().audit(&bodies, &forbidden, "t0");
        assert_eq!(p.violations.len(), 1);
        match &p.violations[0] {
            AgnosticityViolation::PackKindSpecificImport { module, offender } => {
                assert_eq!(module, "mod_a");
                assert_eq!(offender, "PackKind::Uat");
            }
            other => panic!("expected PackKindSpecificImport, got {other:?}"),
        }
    }

    // ── S-3: overlapping runtime range ───────────────────────────────

    #[test]
    fn s3_overlapping_runtime_range_ok() {
        let a = manifest("a", (1, 0, 0), (2, 0, 0), "s://x");
        let b = manifest("b", (1, 5, 0), (3, 0, 0), "s://x");
        let r = runner().prove(&a, &b);
        assert!(r.verified);
    }

    // ── S-4: non-overlapping range ───────────────────────────────────

    #[test]
    fn s4_non_overlapping_runtime_range() {
        let a = manifest("a", (1, 0, 0), (1, 5, 0), "s://x");
        let b = manifest("b", (2, 0, 0), (3, 0, 0), "s://x");
        let r = runner().prove(&a, &b);
        assert!(!r.verified);
        assert_eq!(r.violations.len(), 1);
        assert!(matches!(
            r.violations[0],
            AgnosticityViolation::DivergentRuntimeRange { .. }
        ));
    }

    // ── S-5: same schema_ref ⇒ ok ────────────────────────────────────

    #[test]
    fn s5_same_schema_ref_ok() {
        let a = manifest("a", (1, 0, 0), (2, 0, 0), "s://shared");
        let b = manifest("b", (1, 0, 0), (2, 0, 0), "s://shared");
        let r = runner().prove(&a, &b);
        assert!(r.verified);
    }

    // ── S-6: different schema_ref ⇒ violation ────────────────────────

    #[test]
    fn s6_different_schema_ref() {
        let a = manifest("a", (1, 0, 0), (2, 0, 0), "s://a");
        let b = manifest("b", (1, 0, 0), (2, 0, 0), "s://b");
        let r = runner().prove(&a, &b);
        assert!(!r.verified);
        assert!(matches!(
            r.violations[0],
            AgnosticityViolation::DivergentManifestSchemaRef { .. }
        ));
    }

    // ── S-7: closed-set audit ────────────────────────────────────────

    #[test]
    fn s7_closed_set_audit() {
        assert_eq!(AGNOSTICITY_VIOLATION_VARIANT_COUNT, 3);
        let all = [
            AgnosticityViolation::PackKindSpecificImport {
                module: "m".to_string(),
                offender: "o".to_string(),
            },
            AgnosticityViolation::DivergentRuntimeRange {
                pack_a: "a".to_string(),
                pack_b: "b".to_string(),
            },
            AgnosticityViolation::DivergentManifestSchemaRef {
                pack_a: "a".to_string(),
                pack_b: "b".to_string(),
            },
        ];
        assert_eq!(all.len(), AGNOSTICITY_VIOLATION_VARIANT_COUNT);
    }

    // ── Bonus: determinism ───────────────────────────────────────────

    #[test]
    fn s8_deterministic_output() {
        let bodies = vec![("m".to_string(), "x == PackKind::Uat".to_string())];
        let forbidden = vec!["PackKind::Uat".to_string()];
        let p1 = auditor().audit(&bodies, &forbidden, "t-A");
        let p2 = auditor().audit(&bodies, &forbidden, "t-B");
        assert_eq!(p1.violations, p2.violations);
        assert_ne!(p1.proofs_collected_at, p2.proofs_collected_at);
    }

    // ── Bonus: violations sorted by (module, offender) ───────────────

    #[test]
    fn s9_violations_sorted() {
        let bodies = vec![
            ("b_mod".to_string(), "PackKind::Uat".to_string()),
            ("a_mod".to_string(), "PackKind::Uat".to_string()),
        ];
        let forbidden = vec!["PackKind::Uat".to_string()];
        let p = auditor().audit(&bodies, &forbidden, "t0");
        assert_eq!(p.violations.len(), 2);
        match &p.violations[0] {
            AgnosticityViolation::PackKindSpecificImport { module, .. } => {
                assert_eq!(module, "a_mod");
            }
            _ => panic!("wrong variant"),
        }
    }
}
