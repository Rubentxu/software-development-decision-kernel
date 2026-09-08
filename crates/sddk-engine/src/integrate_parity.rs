//! INTEGRATE — Legacy Projection / Parity Check.
//!
//! Cycle: SDD-ADAPTIVE-004 (H8, order 470, context pack
//! `adaptive-sdd`).
//!
//! Pure, deterministic parity checker that compares a legacy
//! projection and an adaptive projection and emits a typed
//! `IntegrateVerdict`. Provides the seam between CONVERGE
//! (`SDD-ADAPTIVE-003`) and Workflow-Lab promotion
//! (`SDD-ADAPTIVE-005`).
//!
//! ## Design
//!
//! - **Closed-set `IntegrateStatus`.** Three values:
//!   `Parity`, `ParityWithIgnored`, `Diverged`.
//! - **Numeric tolerance in basis points.** Consistent with
//!   `LAB-WORKFLOW-002` and `EA-ASSURANCE-001`.
//! - **Ignored keys semantic.** Differing keys in `ignored_keys`
//!   produce `ParityWithIgnored`, never `Diverged`.
//! - **Pure `ParityChecker::check`.** Deterministic; sorted
//!   outputs.
//! - **Read-only.**
//! - **`#[non_exhaustive]`** everywhere.
//!
//! See:
//!
//! - `REQ-IntegrateParity` (spec, accepted)
//! - `ADR-108` (architecture decision, accepted)
//! - `REQ-ConvergeVerification` (dependency).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

// ── LegacyProjection / AdaptiveProjection ────────────────────────────────

/// Legacy projection for a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LegacyProjection {
    /// Surface id.
    pub surface: String,
    /// Source cycle id (legacy).
    pub source_cycle: String,
    /// Captured-at timestamp.
    pub captured_at: String,
    /// Facts.
    pub facts: BTreeMap<String, serde_json::Value>,
}

/// Adaptive projection for a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AdaptiveProjection {
    /// Surface id.
    pub surface: String,
    /// Target cycle id (adaptive).
    pub target_cycle: String,
    /// Captured-at timestamp.
    pub captured_at: String,
    /// Facts.
    pub facts: BTreeMap<String, serde_json::Value>,
}

// ── ParityDiffEntry ──────────────────────────────────────────────────────

/// A single parity diff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ParityDiffEntry {
    /// Key.
    pub key: String,
    /// Legacy value.
    pub legacy: serde_json::Value,
    /// Adaptive value.
    pub adaptive: serde_json::Value,
}

// ── IntegrateStatus ──────────────────────────────────────────────────────

/// Closed-set integration status.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IntegrateStatus {
    /// No diffs.
    Parity,
    /// Diffs exist but only in ignored keys.
    ParityWithIgnored,
    /// Diffs present outside tolerance.
    Diverged,
}

// ── IntegrateVerdict ─────────────────────────────────────────────────────

/// Verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntegrateVerdict {
    /// Surface id.
    pub surface: String,
    /// Status.
    pub status: IntegrateStatus,
    /// Diffs.
    pub diffs: Vec<ParityDiffEntry>,
    /// Ignored keys that differed.
    pub ignored_keys: Vec<String>,
    /// RFC-3339 timestamp supplied by the caller.
    pub evaluated_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl IntegrateVerdict {
    /// Schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── ParityPolicy ─────────────────────────────────────────────────────────

/// Parity policy.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ParityPolicy {
    /// Keys ignored in parity comparison.
    pub ignored_keys: BTreeSet<String>,
    /// Numeric tolerance per key, in basis points.
    pub numeric_tolerance_bps: BTreeMap<String, u32>,
}

// ── IntegrateError ───────────────────────────────────────────────────────

/// Errors emitted by `ParityChecker`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IntegrateError {
    /// Surfaces don't match.
    #[error("surface mismatch: legacy={legacy}, adaptive={adaptive}")]
    SurfaceMismatch {
        /// Legacy surface.
        legacy: String,
        /// Adaptive surface.
        adaptive: String,
    },
    /// A value cannot be parsed.
    #[error("invalid value at key {key}: {reason}")]
    InvalidValue {
        /// Key.
        key: String,
        /// Reason.
        reason: String,
    },
}

// ── ParityChecker ────────────────────────────────────────────────────────

/// Pure checker.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ParityChecker;

impl ParityChecker {
    /// Compare legacy and adaptive projections under `policy`.
    pub fn check(
        &self,
        legacy: &LegacyProjection,
        adaptive: &AdaptiveProjection,
        policy: &ParityPolicy,
        evaluated_at: String,
    ) -> Result<IntegrateVerdict, IntegrateError> {
        if legacy.surface != adaptive.surface {
            return Err(IntegrateError::SurfaceMismatch {
                legacy: legacy.surface.clone(),
                adaptive: adaptive.surface.clone(),
            });
        }

        let mut diffs: Vec<ParityDiffEntry> = Vec::new();
        let mut ignored: Vec<String> = Vec::new();
        let mut keys: BTreeSet<&String> = BTreeSet::new();
        keys.extend(legacy.facts.keys());
        keys.extend(adaptive.facts.keys());
        let keys: Vec<String> = keys.into_iter().cloned().collect();

        for k in keys {
            let l = legacy.facts.get(&k);
            let a = adaptive.facts.get(&k);
            if l == a {
                continue;
            }
            // Numeric tolerance check.
            if let (Some(lv), Some(av)) = (l, a)
                && let (Some(ln), Some(an)) = (lv.as_f64(), av.as_f64())
                && let Some(tol) = policy.numeric_tolerance_bps.get(&k)
                && within_tolerance(ln, an, *tol)
            {
                continue;
            }
            // Ignored key.
            if policy.ignored_keys.contains(&k) {
                ignored.push(k);
                continue;
            }
            diffs.push(ParityDiffEntry {
                key: k.clone(),
                legacy: l.cloned().unwrap_or(serde_json::Value::Null),
                adaptive: a.cloned().unwrap_or(serde_json::Value::Null),
            });
        }

        // Sort outputs deterministically.
        diffs.sort_by(|a, b| a.key.cmp(&b.key));
        ignored.sort();

        let status = if !diffs.is_empty() {
            IntegrateStatus::Diverged
        } else if !ignored.is_empty() {
            IntegrateStatus::ParityWithIgnored
        } else {
            IntegrateStatus::Parity
        };

        Ok(IntegrateVerdict {
            surface: legacy.surface.clone(),
            status,
            diffs,
            ignored_keys: ignored,
            evaluated_at,
            schema_version: IntegrateVerdict::SCHEMA_VERSION,
        })
    }
}

/// Whether `a` and `b` agree within `tol_bps` basis points.
///
/// Symmetric relative error: `|a - b| / ((|a| + |b|) / 2) * 10000`
/// in basis points.
fn within_tolerance(a: f64, b: f64, tol_bps: u32) -> bool {
    if a == b {
        return true;
    }
    let avg = (a.abs() + b.abs()) / 2.0;
    if avg == 0.0 {
        // Both effectively zero — treat as within tolerance for
        // any non-zero tol.
        return true;
    }
    let err_bps = ((a - b).abs() / avg) * 10_000.0;
    (err_bps as u64) <= (tol_bps as u64)
}

// ── Audit guards ─────────────────────────────────────────────────────────

#[allow(unused)]
const INTEGRATE_STATUS_VARIANT_LIST: &[IntegrateStatus] = &[
    IntegrateStatus::Parity,
    IntegrateStatus::ParityWithIgnored,
    IntegrateStatus::Diverged,
];

#[allow(unused)]
const INTEGRATE_ERROR_VARIANT_LIST: &[IntegrateError] = &[
    IntegrateError::SurfaceMismatch {
        legacy: String::new(),
        adaptive: String::new(),
    },
    IntegrateError::InvalidValue {
        key: String::new(),
        reason: String::new(),
    },
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy(surface: &str) -> LegacyProjection {
        LegacyProjection {
            surface: surface.to_string(),
            source_cycle: "L".to_string(),
            captured_at: "t".to_string(),
            facts: BTreeMap::new(),
        }
    }

    fn adaptive(surface: &str) -> AdaptiveProjection {
        AdaptiveProjection {
            surface: surface.to_string(),
            target_cycle: "A".to_string(),
            captured_at: "t".to_string(),
            facts: BTreeMap::new(),
        }
    }

    // ── S-1: No diffs ⇒ Parity ─────────────────────────────────────────

    #[test]
    fn s1_no_diffs_parity() {
        let mut l = legacy("s");
        let mut a = adaptive("s");
        l.facts.insert("k".to_string(), serde_json::json!(1));
        a.facts.insert("k".to_string(), serde_json::json!(1));
        let policy = ParityPolicy::default();
        let v = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        assert_eq!(v.status, IntegrateStatus::Parity);
    }

    // ── S-2: Numeric within tolerance ⇒ Parity ──────────────────────────

    #[test]
    fn s2_numeric_within_tolerance() {
        let mut l = legacy("s");
        let mut a = adaptive("s");
        l.facts
            .insert("count".to_string(), serde_json::json!(100.0));
        a.facts.insert("count".to_string(), serde_json::json!(99.0));
        let mut policy = ParityPolicy::default();
        policy
            .numeric_tolerance_bps
            .insert("count".to_string(), 200);
        let v = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        assert_eq!(v.status, IntegrateStatus::Parity);
    }

    // ── S-3: Numeric outside tolerance ⇒ Diverged ───────────────────────

    #[test]
    fn s3_numeric_outside_tolerance() {
        let mut l = legacy("s");
        let mut a = adaptive("s");
        l.facts
            .insert("count".to_string(), serde_json::json!(100.0));
        a.facts.insert("count".to_string(), serde_json::json!(50.0));
        let mut policy = ParityPolicy::default();
        policy
            .numeric_tolerance_bps
            .insert("count".to_string(), 200);
        let v = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        assert_eq!(v.status, IntegrateStatus::Diverged);
    }

    // ── S-4: Diff in ignored key ⇒ ParityWithIgnored ────────────────────

    #[test]
    fn s4_ignored_key() {
        let mut l = legacy("s");
        let mut a = adaptive("s");
        l.facts.insert("extra".to_string(), serde_json::json!("a"));
        a.facts.insert("extra".to_string(), serde_json::json!("b"));
        let mut policy = ParityPolicy::default();
        policy.ignored_keys.insert("extra".to_string());
        let v = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        assert_eq!(v.status, IntegrateStatus::ParityWithIgnored);
        assert_eq!(v.ignored_keys, vec!["extra".to_string()]);
        assert!(v.diffs.is_empty());
    }

    // ── S-5: Surface mismatch ⇒ error ──────────────────────────────────

    #[test]
    fn s5_surface_mismatch() {
        let l = legacy("a");
        let a = adaptive("b");
        let policy = ParityPolicy::default();
        let err = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap_err();
        assert!(matches!(err, IntegrateError::SurfaceMismatch { .. }));
    }

    // ── S-6: Adaptive missing key ⇒ Diverged ───────────────────────────

    #[test]
    fn s6_adaptive_missing() {
        let mut l = legacy("s");
        let mut a = adaptive("s");
        l.facts.insert("a".to_string(), serde_json::json!(1));
        l.facts.insert("b".to_string(), serde_json::json!(2));
        a.facts.insert("a".to_string(), serde_json::json!(1));
        let policy = ParityPolicy::default();
        let v = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        assert_eq!(v.status, IntegrateStatus::Diverged);
        assert!(v.diffs.iter().any(|d| d.key == "b"));
    }

    // ── S-7: Determinism ───────────────────────────────────────────────

    #[test]
    fn s7_determinism() {
        let mut l = legacy("s");
        let mut a = adaptive("s");
        l.facts.insert("a".to_string(), serde_json::json!(1));
        a.facts.insert("a".to_string(), serde_json::json!(2));
        let policy = ParityPolicy::default();
        let p1 = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        let p2 = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        assert_eq!(
            serde_json::to_string(&p1).unwrap(),
            serde_json::to_string(&p2).unwrap()
        );
    }

    // ── S-8: diffs sorted ──────────────────────────────────────────────

    #[test]
    fn s8_diffs_sorted() {
        let mut l = legacy("s");
        let mut a = adaptive("s");
        l.facts.insert("z".to_string(), serde_json::json!(1));
        l.facts.insert("a".to_string(), serde_json::json!(1));
        l.facts.insert("m".to_string(), serde_json::json!(1));
        a.facts.insert("z".to_string(), serde_json::json!(2));
        a.facts.insert("a".to_string(), serde_json::json!(2));
        a.facts.insert("m".to_string(), serde_json::json!(2));
        let policy = ParityPolicy::default();
        let v = ParityChecker
            .check(&l, &a, &policy, "t".to_string())
            .unwrap();
        let keys: Vec<&str> = v.diffs.iter().map(|d| d.key.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    // ── Extra: ParityPolicy default ────────────────────────────────────

    #[test]
    fn policy_default() {
        let p = ParityPolicy::default();
        assert!(p.ignored_keys.is_empty());
        assert!(p.numeric_tolerance_bps.is_empty());
    }
}
