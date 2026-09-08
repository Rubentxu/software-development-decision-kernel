//! Security, Upgrade, Rollback & Operator Docs
//!
//! Plain-data operational artefacts (security posture + upgrade /
//! rollback plans + operator doc bundle) and a validator.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::generic_pack_contracts::RuntimeVersion;

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`SecurityPosture`]. Bump when a variant is added.
pub const SECURITY_POSTURE_VARIANT_COUNT: usize = 3;

/// Closed-set size of [`OperatorDocError`]. Bump when a variant is added.
pub const OPERATOR_DOC_ERROR_VARIANT_COUNT: usize = 9;

// ── Enums + records ────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityPosture {
    Standard,
    Hardened,
    AirGapped,
}

impl SecurityPosture {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Hardened => "hardened",
            Self::AirGapped => "air_gapped",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeStep {
    pub step_id: String,
    pub from_version: RuntimeVersion,
    pub to_version: RuntimeVersion,
    pub downtime_seconds: u32,
    pub rollback_supported: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradePlan {
    pub plan_id: String,
    pub target_version: RuntimeVersion,
    /// Sorted by step_id.
    pub steps: Vec<UpgradeStep>,
    pub recorded_at: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackStep {
    pub step_id: String,
    pub from_version: RuntimeVersion,
    pub to_version: RuntimeVersion,
    pub requires_data_migration: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackPlan {
    pub plan_id: String,
    pub target_version: RuntimeVersion,
    /// Sorted by step_id.
    pub steps: Vec<RollbackStep>,
    pub recorded_at: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorDocSection {
    pub heading: String,
    pub body: String,
    /// Sorted.
    pub required_in_postures: Vec<SecurityPosture>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorDocBundle {
    pub bundle_id: String,
    /// Sorted by heading.
    pub sections: Vec<OperatorDocSection>,
    pub security_posture: SecurityPosture,
    pub upgrade_plan: UpgradePlan,
    pub rollback_plan: RollbackPlan,
    pub recorded_at: String,
}

// ── Error + validator ──────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorDocError {
    EmptySections,
    DuplicateHeading,
    EmptyBody,
    MissingSectionForPosture { heading: String },
    InvalidUpgradeDirection,
    InvalidRollbackDirection,
    DuplicateStepId,
    EmptyUpgradeSteps,
    EmptyRollbackSteps,
}

pub trait OperatorDocValidator {
    fn validate(&self, bundle: &OperatorDocBundle) -> Result<(), OperatorDocError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultOperatorDocValidator;

impl OperatorDocValidator for DefaultOperatorDocValidator {
    fn validate(&self, bundle: &OperatorDocBundle) -> Result<(), OperatorDocError> {
        // Sections: non-empty, sorted, no duplicate headings, no empty body.
        if bundle.sections.is_empty() {
            return Err(OperatorDocError::EmptySections);
        }
        let mut sorted_sections = bundle.sections.clone();
        sorted_sections.sort_by(|a, b| a.heading.cmp(&b.heading));
        for w in sorted_sections.windows(2) {
            if w[0].heading == w[1].heading {
                return Err(OperatorDocError::DuplicateHeading);
            }
        }
        for s in &sorted_sections {
            if s.body.is_empty() {
                return Err(OperatorDocError::EmptyBody);
            }
        }
        // Required sections present.
        for s in &sorted_sections {
            if s.required_in_postures.contains(&bundle.security_posture) && s.body.is_empty() {
                return Err(OperatorDocError::MissingSectionForPosture {
                    heading: s.heading.clone(),
                });
            }
        }
        // Upgrade plan: non-empty, direction valid, no duplicate step ids.
        if bundle.upgrade_plan.steps.is_empty() {
            return Err(OperatorDocError::EmptyUpgradeSteps);
        }
        let mut sorted_up = bundle.upgrade_plan.steps.clone();
        sorted_up.sort_by(|a, b| a.step_id.cmp(&b.step_id));
        for w in sorted_up.windows(2) {
            if w[0].step_id == w[1].step_id {
                return Err(OperatorDocError::DuplicateStepId);
            }
        }
        for s in &sorted_up {
            if s.to_version <= s.from_version {
                return Err(OperatorDocError::InvalidUpgradeDirection);
            }
        }
        // Rollback plan: non-empty, direction valid, no duplicate step ids.
        if bundle.rollback_plan.steps.is_empty() {
            return Err(OperatorDocError::EmptyRollbackSteps);
        }
        let mut sorted_rb = bundle.rollback_plan.steps.clone();
        sorted_rb.sort_by(|a, b| a.step_id.cmp(&b.step_id));
        for w in sorted_rb.windows(2) {
            if w[0].step_id == w[1].step_id {
                return Err(OperatorDocError::DuplicateStepId);
            }
        }
        for s in &sorted_rb {
            if s.to_version >= s.from_version {
                return Err(OperatorDocError::InvalidRollbackDirection);
            }
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn validator() -> DefaultOperatorDocValidator {
        DefaultOperatorDocValidator
    }

    fn section(heading: &str) -> OperatorDocSection {
        OperatorDocSection {
            heading: heading.to_string(),
            body: "Some docs here.".to_string(),
            required_in_postures: vec![SecurityPosture::Standard],
        }
    }

    fn upgrade_step(id: &str) -> UpgradeStep {
        UpgradeStep {
            step_id: id.to_string(),
            from_version: RuntimeVersion::new(1, 0, 0),
            to_version: RuntimeVersion::new(1, 1, 0),
            downtime_seconds: 60,
            rollback_supported: true,
        }
    }

    fn rollback_step(id: &str) -> RollbackStep {
        RollbackStep {
            step_id: id.to_string(),
            from_version: RuntimeVersion::new(1, 1, 0),
            to_version: RuntimeVersion::new(1, 0, 0),
            requires_data_migration: false,
        }
    }

    fn bundle() -> OperatorDocBundle {
        OperatorDocBundle {
            bundle_id: "b1".to_string(),
            sections: vec![section("Install")],
            security_posture: SecurityPosture::Standard,
            upgrade_plan: UpgradePlan {
                plan_id: "u1".to_string(),
                target_version: RuntimeVersion::new(1, 1, 0),
                steps: vec![upgrade_step("s1")],
                recorded_at: "t0".to_string(),
            },
            rollback_plan: RollbackPlan {
                plan_id: "r1".to_string(),
                target_version: RuntimeVersion::new(1, 0, 0),
                steps: vec![rollback_step("s1")],
                recorded_at: "t0".to_string(),
            },
            recorded_at: "t0".to_string(),
        }
    }

    #[test]
    fn s1_valid_bundle_passes() {
        assert_eq!(validator().validate(&bundle()), Ok(()));
    }

    #[test]
    fn s2_empty_sections_rejected() {
        let mut b = bundle();
        b.sections.clear();
        assert_eq!(
            validator().validate(&b),
            Err(OperatorDocError::EmptySections)
        );
    }

    #[test]
    fn s3_duplicate_heading_rejected() {
        let mut b = bundle();
        b.sections.push(section("Install"));
        assert_eq!(
            validator().validate(&b),
            Err(OperatorDocError::DuplicateHeading)
        );
    }

    #[test]
    fn s4_empty_body_rejected() {
        let mut b = bundle();
        b.sections[0].body = String::new();
        assert_eq!(validator().validate(&b), Err(OperatorDocError::EmptyBody));
    }

    #[test]
    fn s5_invalid_upgrade_direction_rejected() {
        let mut b = bundle();
        b.upgrade_plan.steps[0].to_version = b.upgrade_plan.steps[0].from_version.clone();
        assert_eq!(
            validator().validate(&b),
            Err(OperatorDocError::InvalidUpgradeDirection)
        );
    }

    #[test]
    fn s6_invalid_rollback_direction_rejected() {
        let mut b = bundle();
        b.rollback_plan.steps[0].to_version = b.rollback_plan.steps[0].from_version.clone();
        assert_eq!(
            validator().validate(&b),
            Err(OperatorDocError::InvalidRollbackDirection)
        );
    }

    #[test]
    fn s7_duplicate_step_id_rejected() {
        let mut b = bundle();
        b.upgrade_plan.steps.push(upgrade_step("s1"));
        assert_eq!(
            validator().validate(&b),
            Err(OperatorDocError::DuplicateStepId)
        );
    }

    #[test]
    fn s8_empty_upgrade_steps_rejected() {
        let mut b = bundle();
        b.upgrade_plan.steps.clear();
        assert_eq!(
            validator().validate(&b),
            Err(OperatorDocError::EmptyUpgradeSteps)
        );
    }

    #[test]
    fn s9_closed_set_audit() {
        assert_eq!(SECURITY_POSTURE_VARIANT_COUNT, 3);
        assert_eq!(OPERATOR_DOC_ERROR_VARIANT_COUNT, 9);
        let postures = [
            SecurityPosture::Standard,
            SecurityPosture::Hardened,
            SecurityPosture::AirGapped,
        ];
        assert_eq!(postures.len(), SECURITY_POSTURE_VARIANT_COUNT);
    }
}
