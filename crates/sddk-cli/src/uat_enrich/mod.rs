//! E14.3 — UX Form Agent: semantic form enrichment with deterministic rules.
//!
//! Design: `design.md §Decision: enrich semantic transform`.
//! Spec: `REQ-E14-EnrichForms-Semantic-Transform`.
//!
//! Decision tree for scenarios without existing form:
//! - Machine check: HTTP/API/DOM/JSON criteria → `UatFormOracleKind::Http/Json/Dom`
//! - Rating: UX subjective criteria → `UatFormInputKind::Rating` with scale anchors
//! - Blind observation: expected textual observable → `UatFormVisibility::Blind`
//! - Human confirmation: fallback when no other rule applies
//! - Checkpoint: every 5 items when total > 5
//! - P0/P1: blocking checks require `[Screenshot]` evidence
//! - Provenance: `UatProvenance` with author="uat-ux-form", additive fields

mod rules;

#[cfg(test)]
mod tests;

// Re-export domain types for tests
#[allow(unused_imports)]
pub use sddk_domain::{
    UatFormCheck, UatFormElementKind as FEK, UatFormEvidenceKind as FEVK, UatFormInputKind as FIK,
    UatFormItem, UatFormOracleKind as FOK, UatFormSpec, UatFormVisibility as FVIS, UatPriority,
};

/// Build a default form for a scenario using deterministic enrichment rules.
/// Returns the existing form if the scenario already has one (preservation rule).
#[allow(dead_code)]
pub fn build_default_form(scenario: &sddk_domain::UatScenario) -> UatFormSpec {
    if scenario.form.is_some() {
        // Preservation rule: don't overwrite existing forms
        return scenario.form.clone().unwrap();
    }

    rules::build_form_for_scenario(scenario)
}

/// Enrich a scenario with form AND provenance.
/// Does NOT touch existing forms (preservation rule).
/// For new scenarios, assigns form and provenance using REAL UatProvenance.
pub fn enrich_scenario(scenario: &mut sddk_domain::UatScenario) {
    // Step 1: preserve existing form if present
    if scenario.form.is_some() {
        // Only set provenance if form already exists (provenance was not set before)
        if scenario.provenance.is_none() {
            scenario.provenance = Some(build_provenance(scenario));
        }
        return;
    }

    // Step 2: build form for scenario without form
    let form = rules::build_form_for_scenario(scenario);
    scenario.form = Some(form);

    // Step 3: set provenance (only if not already set)
    if scenario.provenance.is_none() {
        scenario.provenance = Some(build_provenance(scenario));
    }
}

/// Build a UatProvenance for the enriched scenario.
fn build_provenance(scenario: &sddk_domain::UatScenario) -> sddk_domain::UatProvenance {
    use sddk_domain::UatOrigin;

    // Preserve existing origin if set, otherwise use Regression as default
    let origin = scenario
        .provenance
        .as_ref()
        .map(|p| p.origin)
        .unwrap_or(UatOrigin::Regression);

    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC 3339 formatting cannot fail");
    sddk_domain::UatProvenance {
        author: "uat-ux-form".to_string(),
        created_at: now.clone(),
        last_modified_at: now,
        origin,
        origin_ref: Some(scenario.id.clone()),
    }
}
