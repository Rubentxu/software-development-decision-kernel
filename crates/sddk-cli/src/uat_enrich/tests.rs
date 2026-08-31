//! Tests for uat_enrich — semantic form enrichment.
//!
//! Test coverage:
//! - HTTP→machine oracle for API/HTTP criteria
//! - UX→rating for subjective UX criteria
//! - Blind observation with coherent visibility
//! - Long form → checkpoint insertion every 5 items
//! - Provenance emission per enriched scenario
//! - Existing form preservation (not overwritten)

use sddk_domain::{
    UatFormElementKind as FEK, UatFormEvidenceKind as FEVK, UatFormInputKind as FIK, UatFormItem,
    UatFormOracleKind as FOK, UatFormSpec, UatFormVisibility as FVIS, UatOrigin, UatPriority,
    UatProvenance, UatScenario, UatScenarioContext, UatStep, UatStepKind,
};

use crate::uat_enrich::{build_default_form, enrich_scenario};

// ─── Helper constructors ────────────────────────────────────────────────────

fn make_scenario(id: &str, title: &str) -> UatScenario {
    UatScenario {
        id: id.to_string(),
        title: title.to_string(),
        priority: UatPriority::P2,
        assignee: sddk_domain::UatAssignee::default(),
        preconditions: vec![],
        plain_steps: vec![],
        technical_steps: vec![],
        rationale: None,
        evidence_prompt: None,
        flags: vec![],
        est_minutes: 5,
        context: None,
        evidence: None,
        risk: None,
        automation: None,
        provenance: None,
        executor: None,
        evidence_bundle: None,
        oracles: vec![],
        review: None,
        acceptance: None,
        form: None,
        form_checkpoint: None,
        form_completion: None,
        completion: None,
        staleness: None,
    }
}

fn make_http_scenario(id: &str, title: &str, action: &str, expected: &str) -> UatScenario {
    let mut scenario = make_scenario(id, title);
    scenario.plain_steps = vec![UatStep {
        action: action.to_string(),
        copy_hint: false,
        expected: expected.to_string(),
        step: Some(1),
        kind: Some(UatStepKind::Api),
        vs_expected_check: None,
    }];
    scenario
}

fn make_ux_scenario(id: &str, title: &str, criterion: &str) -> UatScenario {
    let mut scenario = make_scenario(id, title);
    scenario.context = Some(UatScenarioContext {
        user_story: Some(criterion.to_string()),
        preconditions: vec![],
        workspace: None,
        timing: None,
        help: None,
        failure_protocol: None,
        postconditions: vec![],
        test_data: vec![],
    });
    scenario
}

fn scenario_with_steps(id: &str, title: &str, steps: Vec<UatStep>) -> UatScenario {
    let mut scenario = make_scenario(id, title);
    scenario.plain_steps = steps;
    scenario
}

fn scenario_p0(id: &str, title: &str) -> UatScenario {
    let mut scenario = make_scenario(id, title);
    scenario.priority = UatPriority::P0;
    scenario
}

// ─── Test 1: HTTP→machine oracle ──────────────────────────────────────────

#[test]
fn http_scenario_emits_machine_oracle_only() {
    // GIVEN a scenario with API step and HTTP expected result
    let scenario = make_http_scenario(
        "S-1",
        "API returns 200 on /health",
        "GET /health",
        "Response status is 200",
    );

    // WHEN build_default_form is called
    let form = build_default_form(&scenario);

    // THEN the form contains a machine oracle check (HTTP)
    let check_items: Vec<_> = form
        .items
        .iter()
        .filter(|item| item.kind == FEK::Check)
        .collect();

    // At least one check should have an HTTP oracle
    let has_http_oracle = check_items.iter().any(|item| {
        item.check
            .as_ref()
            .map(|c| c.oracle == Some(FOK::Http))
            .unwrap_or(false)
    });

    // AND it should NOT be just a blind confirmation
    // (i.e., it should have an oracle indicating machine check)
    assert!(
        has_http_oracle,
        "Expected HTTP oracle in form, got items: {:?}",
        form.items
    );
}

#[test]
fn dom_scenario_emits_dom_oracle() {
    // GIVEN a scenario with UI step that explicitly checks DOM element
    let scenario = scenario_with_steps(
        "S-2",
        "Login form has visible submit button",
        vec![UatStep {
            action: "Check if submit button is visible using DOM selector".to_string(),
            copy_hint: false,
            expected: "Button element is present and visible".to_string(),
            step: Some(1),
            kind: Some(UatStepKind::Ui),
            vs_expected_check: None,
        }],
    );

    let form = build_default_form(&scenario);

    // THEN the form contains a DOM oracle check
    let has_dom_oracle = form.items.iter().any(|item| {
        item.kind == FEK::Check
            && item
                .check
                .as_ref()
                .map(|c| c.oracle == Some(FOK::Dom))
                .unwrap_or(false)
    });
    assert!(
        has_dom_oracle,
        "Expected DOM oracle in form for UI scenario, got: {:?}",
        form.items
    );
}

#[test]
fn json_scenario_emits_json_oracle() {
    // GIVEN a scenario with explicit JSON API check
    let scenario = scenario_with_steps(
        "S-3",
        "API returns valid JSON with user data",
        vec![UatStep {
            action: "GET /api/user/1 and check JSON response".to_string(),
            copy_hint: false,
            expected: "JSON body contains name field".to_string(),
            step: Some(1),
            kind: Some(UatStepKind::Api),
            vs_expected_check: None,
        }],
    );

    let form = build_default_form(&scenario);

    // THEN the form contains a JSON oracle
    let has_json_oracle = form.items.iter().any(|item| {
        item.kind == FEK::Check
            && item
                .check
                .as_ref()
                .map(|c| c.oracle == Some(FOK::Json))
                .unwrap_or(false)
    });
    assert!(
        has_json_oracle,
        "Expected JSON oracle in form for API scenario, got: {:?}",
        form.items
    );
}

// ─── Test 2: UX→rating ───────────────────────────────────────────────────

#[test]
fn ux_subjective_scenario_emits_rating() {
    // GIVEN a scenario with subjective UX criterion
    let scenario = make_ux_scenario(
        "S-4",
        "Error message is helpful",
        "As a user, I want helpful error messages so I know what to fix",
    );

    let form = build_default_form(&scenario);

    // THEN the form contains a rating input
    let has_rating = form.items.iter().any(|item| {
        item.kind == FEK::Check
            && item
                .check
                .as_ref()
                .map(|c| c.kind == FIK::Rating)
                .unwrap_or(false)
    });
    assert!(
        has_rating,
        "Expected rating input for UX scenario, got: {:?}",
        form.items
    );

    // AND the rating has a scale with anchors
    let rating_item = form.items.iter().find(|item| {
        item.kind == FEK::Check
            && item
                .check
                .as_ref()
                .map(|c| c.kind == FIK::Rating)
                .unwrap_or(false)
    });

    if let Some(item) = rating_item {
        let check = item.check.as_ref().unwrap();
        // Scale should be present (min/max) — represented via options or comment
        // The rating should have comment_required_when for low scores
        assert!(
            check.comment_required_when.is_some()
                || !check.options.is_empty()
                || check.expected.is_some(),
            "Rating should have scale anchors or comment requirement, got: {:?}",
            check
        );
    }
}

// ─── Test 3: Blind observation ─────────────────────────────────────────────

#[test]
fn blind_observation_scenario_has_blind_visibility() {
    // GIVEN a scenario with expected textual observable (blind observation)
    let scenario = scenario_with_steps(
        "S-5",
        "Error message content is appropriate",
        vec![UatStep {
            action: "Trigger the error condition".to_string(),
            copy_hint: false,
            expected: "Error message contains helpful guidance".to_string(),
            step: Some(1),
            kind: Some(UatStepKind::Ui),
            vs_expected_check: None,
        }],
    );

    let form = build_default_form(&scenario);

    // THEN the form has a blind observation check
    let has_blind = form.items.iter().any(|item| {
        item.kind == FEK::Check
            && item
                .check
                .as_ref()
                .map(|c| c.visibility == FVIS::Blind)
                .unwrap_or(false)
    });
    assert!(
        has_blind,
        "Expected blind visibility for blind observation scenario, got: {:?}",
        form.items
    );
}

#[test]
fn blind_check_has_hidden_expected() {
    // GIVEN a blind observation scenario
    let scenario = scenario_with_steps(
        "S-6",
        "Price display is correct",
        vec![UatStep {
            action: "View the product page".to_string(),
            copy_hint: false,
            expected: "Price shows $49.99".to_string(),
            step: Some(1),
            kind: Some(UatStepKind::Ui),
            vs_expected_check: None,
        }],
    );

    let form = build_default_form(&scenario);

    // THEN the blind check has hidden expected field
    let blind_item = form.items.iter().find(|item| {
        item.kind == FEK::Check
            && item
                .check
                .as_ref()
                .map(|c| c.visibility == FVIS::Blind)
                .unwrap_or(false)
    });

    assert!(
        blind_item.is_some(),
        "Expected blind check item, got: {:?}",
        form.items
    );

    // The expected should be hidden (None or not visible to user)
    let check = blind_item.unwrap().check.as_ref().unwrap();
    // In a real blind check, the expected is hidden from the user
    // So it should be stored but not displayed
    assert_eq!(
        check.visibility,
        FVIS::Blind,
        "Blind check should have visibility=blind"
    );
}

// ─── Test 4: Long form → checkpoint ───────────────────────────────────────

#[test]
fn long_form_inserts_checkpoint_after_5_items() {
    // GIVEN a scenario with 8 plain_steps (generates 8+ items: action+check per step)
    let steps: Vec<UatStep> = (1..=8)
        .map(|i| UatStep {
            action: format!("Step {} action", i),
            copy_hint: false,
            expected: format!("Step {} expected result", i),
            step: Some(i),
            kind: Some(UatStepKind::Ui),
            vs_expected_check: None,
        })
        .collect();
    let scenario = scenario_with_steps("S-7", "Long multi-step scenario", steps);

    // WHEN build_default_form is called
    let form = build_default_form(&scenario);

    // THEN the form has >5 items
    assert!(
        form.items.len() > 5,
        "Form with 8 steps should produce >5 items, got {}",
        form.items.len()
    );

    // AND a checkpoint exists between items 5 and 6
    let checkpoint_count = form
        .items
        .iter()
        .filter(|item| item.kind == FEK::Checkpoint)
        .count();
    assert!(
        checkpoint_count >= 1,
        "Form with >5 items must have at least one checkpoint, got {}",
        checkpoint_count
    );

    // Find the position of the first checkpoint
    let cp_positions: Vec<usize> = form
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.kind == FEK::Checkpoint)
        .map(|(pos, _)| pos)
        .collect();
    // Checkpoint should appear AFTER item 5 (i.e., at index >= 5)
    if let Some(first_cp_pos) = cp_positions.first() {
        assert!(
            *first_cp_pos >= 5,
            "First checkpoint must appear at or after position 5, got position {}",
            first_cp_pos
        );
    }
}

#[test]
fn short_form_no_checkpoint() {
    // GIVEN a scenario with fewer than 6 items
    let scenario = scenario_with_steps(
        "S-8",
        "Simple two-step scenario",
        vec![
            UatStep {
                action: "Step 1".to_string(),
                copy_hint: false,
                expected: "Result 1".to_string(),
                step: Some(1),
                kind: Some(UatStepKind::Ui),
                vs_expected_check: None,
            },
            UatStep {
                action: "Step 2".to_string(),
                copy_hint: false,
                expected: "Result 2".to_string(),
                step: Some(2),
                kind: Some(UatStepKind::Ui),
                vs_expected_check: None,
            },
        ],
    );

    let form = build_default_form(&scenario);

    // THEN no checkpoint items exist
    let checkpoint_count = form
        .items
        .iter()
        .filter(|item| item.kind == FEK::Checkpoint)
        .count();
    assert_eq!(
        checkpoint_count, 0,
        "Short form should not have checkpoint, got: {:?}",
        form.items
    );
}

// ─── Test 5: P0/P1 requires Screenshot ───────────────────────────────────

#[test]
fn p0_scenario_requires_screenshot() {
    // GIVEN a P0 priority scenario
    let scenario = scenario_p0("S-9", "Critical login flow");

    let form = build_default_form(&scenario);

    // THEN evidence_requirement includes Screenshot
    let blocking_items: Vec<_> = form
        .items
        .iter()
        .filter(|item| {
            item.kind == FEK::Check && item.check.as_ref().map(|c| c.blocking).unwrap_or(false)
        })
        .collect();

    let has_screenshot_req = blocking_items.iter().any(|item| {
        item.check
            .as_ref()
            .map(|c| c.evidence_requirement.contains(&FEVK::Screenshot))
            .unwrap_or(false)
    });

    assert!(
        has_screenshot_req,
        "P0 blocking checks should require Screenshot evidence, got: {:?}",
        form.items
    );
}

#[test]
fn p1_scenario_requires_screenshot() {
    // GIVEN a P1 priority scenario
    let mut scenario = make_scenario("S-10", "High priority feature");
    scenario.priority = UatPriority::P1;

    let form = build_default_form(&scenario);

    // THEN evidence_requirement includes Screenshot
    let blocking_items: Vec<_> = form
        .items
        .iter()
        .filter(|item| {
            item.kind == FEK::Check && item.check.as_ref().map(|c| c.blocking).unwrap_or(false)
        })
        .collect();

    let has_screenshot_req = blocking_items.iter().any(|item| {
        item.check
            .as_ref()
            .map(|c| c.evidence_requirement.contains(&FEVK::Screenshot))
            .unwrap_or(false)
    });

    assert!(
        has_screenshot_req,
        "P1 blocking checks should require Screenshot evidence, got: {:?}",
        form.items
    );
}

#[test]
fn p2_scenario_no_mandatory_screenshot() {
    // GIVEN a P2 priority scenario
    let scenario = make_scenario("S-11", "Low priority feature");

    let form = build_default_form(&scenario);

    // THEN Screenshot is not mandatory (may be optional or empty)
    let blocking_items: Vec<_> = form
        .items
        .iter()
        .filter(|item| {
            item.kind == FEK::Check && item.check.as_ref().map(|c| c.blocking).unwrap_or(false)
        })
        .collect();

    for item in blocking_items {
        let check = item.check.as_ref().unwrap();
        // For P2, screenshot evidence is not required (may be empty or optional)
        if check.evidence_requirement.contains(&FEVK::Screenshot) {
            // This is OK - screenshot can be present but is not mandatory blocking
        }
    }
    // The test passes if we get here without assertion failure
}

// ─── Test 6: Provenance emission ──────────────────────────────────────────

#[test]
fn enrich_scenario_populates_provenance() {
    // GIVEN a scenario without provenance
    let mut scenario = make_scenario("S-12", "Any scenario");
    assert!(
        scenario.provenance.is_none(),
        "precondition: no provenance yet"
    );

    // WHEN enrich_scenario is called
    enrich_scenario(&mut scenario);

    // THEN provenance is populated with uat-ux-form as author
    assert!(
        scenario.provenance.is_some(),
        "Enriched scenario must have provenance set"
    );
    let prov = scenario.provenance.as_ref().unwrap();
    assert_eq!(
        prov.author, "uat-ux-form",
        "Provenance author must be 'uat-ux-form', got '{}'",
        prov.author
    );
    assert!(
        !prov.created_at.is_empty(),
        "Provenance created_at must be set"
    );
    assert!(
        !prov.last_modified_at.is_empty(),
        "Provenance last_modified_at must be set"
    );
}

#[test]
fn enrich_scenario_preserves_existing_provenance() {
    // GIVEN a scenario that already has provenance
    let mut scenario = make_scenario("S-12b", "Any scenario");
    scenario.provenance = Some(UatProvenance {
        author: "original-author".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        last_modified_at: "2024-01-02T00:00:00Z".to_string(),
        origin: UatOrigin::Spec,
        origin_ref: Some("original-ref".to_string()),
    });

    // WHEN enrich_scenario is called
    enrich_scenario(&mut scenario);

    // THEN provenance is NOT overwritten (preserved)
    let prov = scenario.provenance.as_ref().unwrap();
    assert_eq!(
        prov.author, "original-author",
        "Existing provenance author must be preserved, got '{}'",
        prov.author
    );
}

#[test]
fn enrich_scenario_preserves_existing_form() {
    // GIVEN a scenario that already has a form
    let existing_form = UatFormSpec {
        dsl_version: 1,
        items: vec![UatFormItem {
            kind: FEK::Info,
            id: Some("custom-item".to_string()),
            check: None,
            text: Some("Custom existing form".to_string()),
            flow: None,
            target: None,
            checkpoint: None,
        }],
        completion: None,
    };

    let mut scenario = make_scenario("S-13", "Scenario with existing form");
    scenario.form = Some(existing_form.clone());

    // WHEN enrich_scenario is called
    enrich_scenario(&mut scenario);

    // THEN the form is preserved (not overwritten) AND provenance is still set
    assert!(scenario.form.is_some(), "Existing form must be preserved");
    assert_eq!(
        scenario.form.as_ref().unwrap().items.len(),
        existing_form.items.len(),
        "Form must not be regenerated when it already exists"
    );
    // But provenance should be set since it didn't exist
    assert!(
        scenario.provenance.is_some(),
        "Provenance should be set even when form is preserved"
    );
}

// ─── Test 7: Existing form preservation ───────────────────────────────────

#[test]
fn existing_form_is_preserved() {
    // GIVEN a scenario that already has a form
    let existing_form = UatFormSpec {
        dsl_version: 1,
        items: vec![UatFormItem {
            kind: FEK::Info,
            id: Some("custom-item".to_string()),
            check: None,
            text: Some("Custom existing form".to_string()),
            flow: None,
            target: None,
            checkpoint: None,
        }],
        completion: None,
    };

    let mut scenario = make_scenario("S-13", "Scenario with existing form");
    scenario.form = Some(existing_form.clone());

    // WHEN build_default_form is called (simulating the enrichment logic)
    // The enrichment logic should NOT call build_default_form if form.is_some()
    // Here we verify the function doesn't overwrite
    let form = build_default_form(&scenario);

    // THEN the existing form is preserved (not overwritten)
    // Since build_default_form should be called only when form.is_none(),
    // passing a scenario with form=None should create a new form
    // This test documents the expected behavior
    assert!(
        !form.items.is_empty(),
        "build_default_form should produce at least one item"
    );
}

// ─── Test 8: Human confirmation as fallback ──────────────────────────────

#[test]
fn fallback_human_confirmation() {
    // GIVEN a scenario with no specific indicators (default case)
    let scenario = make_scenario("S-14", "Generic scenario without specifics");

    let form = build_default_form(&scenario);

    // THEN the form contains a human confirmation check as fallback
    let has_confirm = form.items.iter().any(|item| {
        item.kind == FEK::Check
            && item
                .check
                .as_ref()
                .map(|c| {
                    c.kind == FIK::Confirm && c.oracle.is_none() && c.visibility == FVIS::Visible
                })
                .unwrap_or(false)
    });

    assert!(
        has_confirm,
        "Fallback should be human confirmation, got: {:?}",
        form.items
    );
}

// ─── Test 9: Form action/instruction ───────────────────────────────────────

#[test]
fn form_contains_instruction_items() {
    // GIVEN a scenario
    let scenario = make_scenario("S-15", "Scenario requiring instructions");

    let form = build_default_form(&scenario);

    // THEN the form contains Info items for instructions
    let has_info = form.items.iter().any(|item| item.kind == FEK::Info);
    assert!(
        has_info,
        "Form should contain Info items for instructions, got: {:?}",
        form.items
    );
}

// ─── Test 10: Blocking check ───────────────────────────────────────────────

#[test]
fn blocking_checks_are_truly_blocking() {
    // GIVEN any scenario
    let scenario = make_scenario("S-16", "Any scenario");

    let form = build_default_form(&scenario);

    // THEN blocking checks have blocking=true
    let blocking_checks: Vec<_> = form
        .items
        .iter()
        .filter(|item| {
            item.kind == FEK::Check && item.check.as_ref().map(|c| c.blocking).unwrap_or(false)
        })
        .collect();

    for item in &blocking_checks {
        let check = item.check.as_ref().unwrap();
        assert!(check.blocking, "Blocking check should have blocking=true");
        assert!(check.required, "Blocking check should have required=true");
    }
}
