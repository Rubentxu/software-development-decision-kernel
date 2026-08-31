//! E14.5 — Pure planner for the generate pipeline.
//!
//! Consumes: requirements markdown, changelog Added/Changed, last-plan continuity,
//! and (if discovery ran) AamModel scenario_candidates.
//! Produces: UatPlan with features/scenarios, NOT empty.
//!
//! Does NOT write files. Returns plan directly for atomic write by caller.

pub mod merge;

use sddk_domain::{UatFeature, UatPlan, UatPlanRelease, UatPriority, UatScenario};

use super::parsing::{
    extract_criteria_from_md, extract_req_ids, parse_changelog_sections,
    scenario_title_from_criterion, step_from_text,
};
use super::planner::merge::merge_plan_features;

/// Planning errors.
#[derive(Debug)]
pub enum PlanError {
    /// No features could be extracted from inputs (empty plan would be produced).
    NoFeaturesExtracted,
    /// Last plan parse/read failed.
    LastPlanParseFailed(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::NoFeaturesExtracted => {
                write!(f, "no features could be extracted from inputs")
            }
            PlanError::LastPlanParseFailed(msg) => {
                write!(f, "last plan parse failed: {}", msg)
            }
        }
    }
}

/// RFC 3339 "now" with nanosecond precision; used for UatProvenance and UatPlan timestamps.
/// Replaces the deleted Hinnant wrapper for this module only.
fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC 3339 formatting cannot fail")
}

/// Planner output: the built UatPlan.
#[derive(Debug)]
pub struct PlanOutput {
    /// The constructed UatPlan.
    pub plan: UatPlan,
}

impl PlanOutput {
    /// Validate that plan has features and scenarios (non-empty).
    pub fn validate_non_empty(&self) -> Result<(), PlanError> {
        if self.plan.features.is_empty() {
            return Err(PlanError::NoFeaturesExtracted);
        }
        let total_scenarios: usize = self.plan.features.iter().map(|f| f.scenarios.len()).sum();
        if total_scenarios == 0 {
            return Err(PlanError::NoFeaturesExtracted);
        }
        Ok(())
    }
}

/// Build a feature from scenarios with given base ID.
fn build_feature(
    scenarios: Vec<UatScenario>,
    feature_id: usize,
    name: String,
    req_ref: Option<String>,
    priority: UatPriority,
) -> UatFeature {
    UatFeature {
        id: format!("F-{:02}", feature_id),
        name,
        requirement_ref: req_ref,
        design_ref: None,
        priority,
        scenarios,
    }
}

/// Build features from criteria list.
fn build_features_from_criteria(
    all_criteria: &[(String, Option<String>)],
) -> (Vec<UatFeature>, usize) {
    let mut features: Vec<UatFeature> = Vec::new();
    let mut feature_scenarios: Vec<UatScenario> = Vec::new();
    let mut scenario_id = 1usize;
    let mut current_req_id: Option<String> = None;
    let mut current_feature_name = "General".to_string();
    let mut feature_scenario_count = 0usize;

    for (criterion, req_id) in all_criteria {
        if current_req_id.is_none() || current_req_id.as_ref() != req_id.as_ref() {
            if !feature_scenarios.is_empty() {
                features.push(build_feature(
                    std::mem::take(&mut feature_scenarios),
                    features.len() + 1,
                    current_feature_name.clone(),
                    current_req_id.clone(),
                    UatPriority::P1,
                ));
                feature_scenario_count = 0;
            }
            current_req_id = req_id.clone();
            current_feature_name = req_id
                .clone()
                .unwrap_or_else(|| "Feature Group".to_string());
        }

        let plain_steps = vec![step_from_text(&format!("Verify: {}", criterion))];
        let provenance = sddk_domain::UatProvenance {
            author: "uat-planner".to_string(),
            created_at: now_rfc3339(),
            last_modified_at: now_rfc3339(),
            origin: sddk_domain::UatOrigin::Spec,
            origin_ref: req_id.clone(),
        };

        let scenario = UatScenario {
            id: format!("S-{:03}", scenario_id),
            title: scenario_title_from_criterion(criterion),
            priority: UatPriority::P1,
            assignee: sddk_domain::UatAssignee::Developer,
            preconditions: Vec::new(),
            plain_steps,
            technical_steps: Vec::new(),
            rationale: None,
            evidence_prompt: None,
            flags: Vec::new(),
            est_minutes: 5,
            context: None,
            evidence: None,
            risk: None,
            automation: None,
            provenance: Some(provenance),
            executor: None,
            evidence_bundle: None,
            oracles: Vec::new(),
            review: None,
            acceptance: None,
            form: None,
            form_checkpoint: None,
            form_completion: None,
            completion: None,
            staleness: None,
        };

        feature_scenarios.push(scenario);
        scenario_id += 1;
        feature_scenario_count += 1;

        if feature_scenario_count >= 5 {
            features.push(build_feature(
                std::mem::take(&mut feature_scenarios),
                features.len() + 1,
                current_feature_name.clone(),
                current_req_id.clone(),
                UatPriority::P1,
            ));
            feature_scenario_count = 0;
        }
    }

    if !feature_scenarios.is_empty() {
        features.push(build_feature(
            feature_scenarios,
            features.len() + 1,
            current_feature_name,
            current_req_id,
            UatPriority::P1,
        ));
    }

    let len = features.len();
    (features, len)
}

/// Pure planner: build UatPlan from requirements, changelog, last_plan, and AAM candidates.
/// Does NOT write files. Returns PlanOutput for atomic write by caller.
pub fn build_plan(
    release: &str,
    requirements: &Option<std::path::PathBuf>,
    changelog: &Option<std::path::PathBuf>,
    last_plan: &Option<std::path::PathBuf>,
    aam_scenario_candidates: &[crate::uat_discover::AamScenarioCandidate],
) -> Result<PlanOutput, PlanError> {
    let mut all_criteria: Vec<(String, Option<String>)> = Vec::new();

    // Consume requirements markdown
    if let Some(req_dir) = requirements
        && req_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(req_dir)
    {
        let mut files: Vec<_> = entries.flatten().collect();
        files.sort_by_key(|e| e.file_name());
        for entry in files {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let req_ids = extract_req_ids(&content);
                for criterion in extract_criteria_from_md(&content) {
                    let req_id = req_ids.first().cloned();
                    all_criteria.push((criterion, req_id));
                }
            }
        }
    }

    // Consume changelog Added/Changed sections
    if let Some(cl) = changelog
        && cl.exists()
        && let Ok(content) = std::fs::read_to_string(cl)
    {
        let (added, changed) = parse_changelog_sections(&content);
        for criterion in added.into_iter().chain(changed) {
            all_criteria.push((criterion, None));
        }
    }

    // Consume last-plan for continuity
    let last_plan_ref: Option<UatPlan> = if let Some(lp) = last_plan {
        if lp.exists() {
            let content = std::fs::read_to_string(lp)
                .map_err(|e| PlanError::LastPlanParseFailed(format!("read failed: {}", e)))?;
            let prev_plan: UatPlan = serde_saphyr::from_str(&content)
                .map_err(|e| PlanError::LastPlanParseFailed(format!("parse failed: {}", e)))?;
            Some(prev_plan)
        } else {
            return Err(PlanError::LastPlanParseFailed(
                "last_plan file not found".to_string(),
            ));
        }
    } else {
        None
    };

    // Consume AamModel scenario_candidates from discovery
    let mut discovery_scenarios = Vec::new();
    for candidate in aam_scenario_candidates {
        let plain_steps: Vec<sddk_domain::UatStep> = candidate
            .plain_steps
            .iter()
            .map(|s| step_from_text(s))
            .collect();

        let provenance = sddk_domain::UatProvenance {
            author: "uat-discovery".to_string(),
            created_at: candidate
                .provenance
                .created_at
                .clone()
                .unwrap_or_else(now_rfc3339),
            last_modified_at: now_rfc3339(),
            origin: sddk_domain::UatOrigin::Regression,
            origin_ref: candidate.flow_ref.clone(),
        };

        let scenario = UatScenario {
            id: format!("S-D{:03}", discovery_scenarios.len() + 1),
            title: candidate.title.clone(),
            priority: UatPriority::P2,
            assignee: sddk_domain::UatAssignee::Developer,
            preconditions: Vec::new(),
            plain_steps,
            technical_steps: Vec::new(),
            rationale: None,
            evidence_prompt: None,
            flags: Vec::new(),
            est_minutes: candidate.estimated_duration_minutes.unwrap_or(5),
            context: None,
            evidence: None,
            risk: None,
            automation: None,
            provenance: Some(provenance),
            executor: None,
            evidence_bundle: None,
            oracles: Vec::new(),
            review: None,
            acceptance: None,
            form: None,
            form_checkpoint: None,
            form_completion: None,
            completion: None,
            staleness: None,
        };
        discovery_scenarios.push(scenario);
    }

    // Build features from criteria
    let (mut new_features, _) = build_features_from_criteria(&all_criteria);

    // Add discovery scenarios as a dedicated feature
    if !discovery_scenarios.is_empty() {
        new_features.push(build_feature(
            discovery_scenarios,
            new_features.len() + 1,
            "Discovered Flows".to_string(),
            None,
            UatPriority::P2,
        ));
    }

    // Merge with last_plan using merge module
    let features: Vec<UatFeature> = merge_plan_features(new_features, last_plan_ref.as_ref());

    // If no features, return error (atomic: no partial output)
    if features.is_empty() {
        return Err(PlanError::NoFeaturesExtracted);
    }

    // Build last_uat_release from previous plan
    let last_uat_release = last_plan_ref.as_ref().map(|p| p.release.candidate.clone());

    let now = now_rfc3339();
    let plan = UatPlan {
        schema_version: sddk_domain::LATEST_PLAN_SCHEMA_VERSION,
        release: UatPlanRelease {
            candidate: release.to_string(),
            project: None,
            last_uat_release,
        },
        generated_by: "uat-planner".to_string(),
        generated_at: now,
        features,
        runner_mode: None,
        approval: None,
    };

    let output = PlanOutput { plan };
    output.validate_non_empty()?;
    Ok(output)
}
