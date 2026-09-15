// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_profile/rebuild.rs — T-03 (A3-S4 / AC3)
//
// `rebuild(overlay, &inputs)`: deterministic delete-and-rebuild for the
// ParadigmProfileOverlay (REQ-AC3-013..015). Mirrors AC2's `RebuildInputs` +
// `rebuild` pattern (REQ-AC2-006). Sorted iteration guarantees identical
// bytes across two invocations on identical inputs.

use super::overlay::ParadigmProfileOverlay;
use super::types::RebuildInputs;

/// `rebuild(overlay, &inputs)` clears the overlay, then re-projects the
/// inputs deterministically (REQ-AC3-013). The same inputs MUST yield
/// identical canonical bytes across two invocations.
pub fn rebuild(overlay: &mut ParadigmProfileOverlay, inputs: &RebuildInputs) {
    overlay.clear();

    // Sort and dedup profiles by (anchor locator, profile kind tag) so that
    // insertion order does not affect canonical bytes (REQ-AC3-014).
    let mut sorted_profiles = inputs.profiles.clone();
    sorted_profiles.sort_by(|a, b| {
        a.anchor
            .locator()
            .cmp(&b.anchor.locator())
            .then(a.profile.domain_tag().cmp(b.profile.domain_tag()))
    });
    sorted_profiles.dedup();

    // Same for lenses.
    let mut sorted_lenses = inputs.lenses.clone();
    sorted_lenses.sort_by(|a, b| {
        a.anchor
            .locator()
            .cmp(&b.anchor.locator())
            .then(a.lens.domain_tag().cmp(b.lens.domain_tag()))
    });
    sorted_lenses.dedup();

    // And assessments.
    let mut sorted_assess = inputs.assessments.clone();
    sorted_assess.sort_by(|a, b| {
        a.anchor
            .locator()
            .cmp(&b.anchor.locator())
            .then(a.lens.domain_tag().cmp(b.lens.domain_tag()))
            .then(a.evaluated_at_ms.cmp(&b.evaluated_at_ms))
    });
    sorted_assess.dedup();

    // Project project intent first (if any), then everything else.
    if let Some(project) = &inputs.project {
        overlay.add_project(project);
    }

    // Insert profiles first so anchor nodes exist when later relations
    // reference them.
    for edge in &sorted_profiles {
        overlay.add_profile(edge);
    }
    for edge in &sorted_lenses {
        overlay.add_lens(edge);
    }
    for assess in &sorted_assess {
        overlay.add_assessment(assess);
    }
}
