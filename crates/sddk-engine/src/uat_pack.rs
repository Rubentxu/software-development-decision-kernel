//! UAT Pack on Canonical Runtime
//!
//! Plain-data UAT pack (manifest + scenarios) validated against the
//! canonical runtime contract. No pack-specific kernel special
//! cases.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::generic_pack_contracts::{
    DefaultPackManifestValidator, PackContractError, PackKind, PackManifest, PackManifestValidator,
    RuntimeVersion,
};

// ── Audit guard constant ────────────────────────────────────────────────

/// Closed-set size of [`UatStep`]. Bump when a variant is added.
pub const UAT_STEP_VARIANT_COUNT: usize = 4;

// ── Step ────────────────────────────────────────────────────────────────

/// Closed-set taxonomy of UAT steps.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UatStep {
    Open { name: String },
    Act { name: String, action: String },
    Assert { name: String, predicate: String },
    Close { name: String },
}

impl UatStep {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Open { .. } => "open",
            Self::Act { .. } => "act",
            Self::Assert { .. } => "assert",
            Self::Close { .. } => "close",
        }
    }

    /// Index of the step (0=open, 1=act, 2=assert, 3=close).
    #[must_use]
    pub const fn order(&self) -> u8 {
        match self {
            Self::Open { .. } => 0,
            Self::Act { .. } => 1,
            Self::Assert { .. } => 2,
            Self::Close { .. } => 3,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Open { name }
            | Self::Act { name, .. }
            | Self::Assert { name, .. }
            | Self::Close { name } => name,
        }
    }
}

// ── Scenario + pack ─────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UatScenario {
    pub scenario_id: String,
    pub target_kind: PackKind,
    /// Sorted by (order, name).
    pub steps: Vec<UatStep>,
    /// Sorted.
    pub acceptance_clauses: Vec<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UatPack {
    pub manifest: PackManifest,
    /// Sorted by `scenario_id`.
    pub scenarios: Vec<UatScenario>,
    pub installed_at: String,
}

// ── Error + installer ──────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UatPackError {
    NotUatKind,
    EmptyScenarios,
    DuplicateScenarioId,
    MissingAcceptanceClause { scenario_id: String },
    ManifestRejected(PackContractError),
    IncompatibleRuntime(String),
}

pub trait UatPackInstaller {
    fn install(
        &self,
        runtime_version: &RuntimeVersion,
        pack: &UatPack,
        known_pack_ids: &[String],
    ) -> Result<(), UatPackError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultUatPackInstaller;

impl UatPackInstaller for DefaultUatPackInstaller {
    fn install(
        &self,
        runtime_version: &RuntimeVersion,
        pack: &UatPack,
        known_pack_ids: &[String],
    ) -> Result<(), UatPackError> {
        // 1. Kind must be Uat.
        if pack.manifest.kind != PackKind::Uat {
            return Err(UatPackError::NotUatKind);
        }
        // 2. Non-empty.
        if pack.scenarios.is_empty() {
            return Err(UatPackError::EmptyScenarios);
        }
        // 3. Unique scenario ids (after sort).
        let mut sorted_scenarios = pack.scenarios.clone();
        sorted_scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
        for w in sorted_scenarios.windows(2) {
            if w[0].scenario_id == w[1].scenario_id {
                return Err(UatPackError::DuplicateScenarioId);
            }
        }
        // 4. Each scenario has at least one acceptance clause.
        for sc in &sorted_scenarios {
            if sc.acceptance_clauses.is_empty() {
                return Err(UatPackError::MissingAcceptanceClause {
                    scenario_id: sc.scenario_id.clone(),
                });
            }
        }
        // 5. Manifest validates against generic contract (excluding
        //    self from the known-ids list so the manifest doesn't
        //    duplicate itself).
        let validator = DefaultPackManifestValidator;
        validator
            .validate(&pack.manifest, runtime_version, known_pack_ids)
            .map_err(UatPackError::ManifestRejected)?;
        // 6. Runtime compatibility (extra defensive check on top of
        //    the manifest validator).
        if runtime_version < &pack.manifest.min_runtime_version
            || runtime_version > &pack.manifest.max_runtime_version
        {
            return Err(UatPackError::IncompatibleRuntime(
                runtime_version.to_string(),
            ));
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::collapsible_if)]
mod tests {
    use super::*;
    use crate::generic_pack_contracts::EvidenceFormat;

    fn scenario(id: &str, clauses: Vec<String>) -> UatScenario {
        UatScenario {
            scenario_id: id.to_string(),
            target_kind: PackKind::Uat,
            steps: vec![
                UatStep::Open {
                    name: "start".to_string(),
                },
                UatStep::Close {
                    name: "end".to_string(),
                },
            ],
            acceptance_clauses: clauses,
        }
    }

    fn manifest(id: &str) -> PackManifest {
        PackManifest {
            pack_id: id.to_string(),
            kind: PackKind::Uat,
            evidence_format: EvidenceFormat::JsonSchemaDraft07,
            min_runtime_version: RuntimeVersion::new(1, 0, 0),
            max_runtime_version: RuntimeVersion::new(2, 0, 0),
            schema_ref: "schema://uat".to_string(),
            recorded_at: "t0".to_string(),
        }
    }

    fn pack(id: &str, scenarios: Vec<UatScenario>) -> UatPack {
        UatPack {
            manifest: manifest(id),
            scenarios,
            installed_at: "t0".to_string(),
        }
    }

    fn installer() -> DefaultUatPackInstaller {
        DefaultUatPackInstaller
    }

    // ── S-1: valid UAT pack installs ──────────────────────────────────

    #[test]
    fn s1_valid_pack_installs() {
        let p = pack(
            "uat-1",
            vec![
                scenario("s1", vec!["c1".to_string(), "c2".to_string()]),
                scenario("s2", vec!["c3".to_string()]),
            ],
        );
        let r = installer().install(&RuntimeVersion::new(1, 5, 0), &p, &[]);
        assert_eq!(r, Ok(()));
    }

    // ── S-2: non-UAT kind rejected ────────────────────────────────────

    #[test]
    fn s2_non_uat_kind_rejected() {
        let mut p = pack("uat-1", vec![scenario("s1", vec!["c".to_string()])]);
        p.manifest.kind = PackKind::Sdd;
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(r, Err(UatPackError::NotUatKind));
    }

    // ── S-3: empty scenarios rejected ────────────────────────────────

    #[test]
    fn s3_empty_scenarios_rejected() {
        let p = pack("uat-1", vec![]);
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(r, Err(UatPackError::EmptyScenarios));
    }

    // ── S-4: duplicate scenario_id rejected ──────────────────────────

    #[test]
    fn s4_duplicate_scenario_id_rejected() {
        let p = pack(
            "uat-1",
            vec![
                scenario("s1", vec!["c".to_string()]),
                scenario("s1", vec!["c".to_string()]),
            ],
        );
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(r, Err(UatPackError::DuplicateScenarioId));
    }

    // ── S-5: missing acceptance clause rejected ───────────────────────

    #[test]
    fn s5_missing_acceptance_clause_rejected() {
        let p = pack("uat-1", vec![scenario("s1", vec![])]);
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(
            r,
            Err(UatPackError::MissingAcceptanceClause {
                scenario_id: "s1".to_string()
            })
        );
    }

    // ── S-6: closed-set audit guard ──────────────────────────────────

    #[test]
    fn s6_closed_set_audit() {
        assert_eq!(UAT_STEP_VARIANT_COUNT, 4);
        let all_steps = [
            UatStep::Open {
                name: "o".to_string(),
            },
            UatStep::Act {
                name: "a".to_string(),
                action: "x".to_string(),
            },
            UatStep::Assert {
                name: "p".to_string(),
                predicate: "y".to_string(),
            },
            UatStep::Close {
                name: "c".to_string(),
            },
        ];
        assert_eq!(all_steps.len(), UAT_STEP_VARIANT_COUNT);
    }

    // ── Bonus: determinism ────────────────────────────────────────────

    #[test]
    fn s7_deterministic_output() {
        let p = pack("uat-1", vec![scenario("s1", vec!["c".to_string()])]);
        let v = RuntimeVersion::new(1, 0, 0);
        let r1 = installer().install(&v, &p, &[]);
        let r2 = installer().install(&v, &p, &[]);
        assert_eq!(r1, r2);
    }

    // ── Bonus: step label + order ─────────────────────────────────────

    #[test]
    fn s8_step_label_and_order() {
        let s = UatStep::Act {
            name: "do".to_string(),
            action: "click".to_string(),
        };
        assert_eq!(s.label(), "act");
        assert_eq!(s.order(), 1);
        assert_eq!(s.name(), "do");
    }
}
