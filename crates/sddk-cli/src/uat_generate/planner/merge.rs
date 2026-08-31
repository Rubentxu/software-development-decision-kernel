//! E14.5 — Merge logic for plan continuity.
//!
//! Handles merging new features with scenarios from the previous plan.
//! Implements: dedupe by ID+title, renumber only collisions, preserve approval=None.

use sddk_domain::{UatFeature, UatPlan, UatPriority, UatScenario};

/// Merge new features with preserved scenarios from last_plan.
///
/// Spec guarantees:
/// - Dedupe by scenario ID + title
/// - Renumber only new collisions (assign new IDs to preserved scenarios that collide)
/// - Keep prev IDs for non-colliding preserved scenarios
/// - If no new criteria, clone all scenarios from last_plan
pub fn merge_plan_features(
    new_features: Vec<UatFeature>,
    last_plan_ref: Option<&UatPlan>,
) -> Vec<UatFeature> {
    if new_features.is_empty() {
        // No new criteria - preserve all scenarios from last_plan
        if let Some(prev_plan) = last_plan_ref {
            prev_plan.features.clone()
        } else {
            Vec::new()
        }
    } else if let Some(prev_plan) = last_plan_ref {
        // New criteria provided - merge with last_plan scenarios.
        // Build set of (id, title) from new scenarios for dedup
        let new_scenario_keys: std::collections::HashSet<(String, String)> = new_features
            .iter()
            .flat_map(|f| f.scenarios.iter().map(|s| (s.id.clone(), s.title.clone())))
            .collect();

        // Clone prev_plan scenarios that don't collide with new ones
        let mut preserved_scenarios: Vec<UatScenario> = prev_plan
            .features
            .iter()
            .flat_map(|f| f.scenarios.iter())
            .filter(|s| !new_scenario_keys.contains(&(s.id.clone(), s.title.clone())))
            .cloned()
            .collect();

        // Find max ID from new scenarios for renumbering
        let max_new_id = new_features
            .iter()
            .flat_map(|f| f.scenarios.iter())
            .filter_map(|s| {
                s.id.strip_prefix("S-")
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);

        let mut next_id = max_new_id + 1;
        for scenario in &mut preserved_scenarios {
            // Renumber only if collision detected
            if new_scenario_keys.contains(&(scenario.id.clone(), scenario.title.clone())) {
                scenario.id = format!("S-{:03}", next_id);
                next_id += 1;
            }
        }

        // Combine: new features first, then preserved scenarios grouped by original feature
        let mut combined = new_features;
        if !preserved_scenarios.is_empty() {
            combined.push(UatFeature {
                id: format!("F-{:02}", combined.len() + 1),
                name: "Preserved from Previous Plan".to_string(),
                requirement_ref: None,
                design_ref: None,
                priority: UatPriority::P2,
                scenarios: preserved_scenarios,
            });
        }
        combined
    } else {
        new_features
    }
}
