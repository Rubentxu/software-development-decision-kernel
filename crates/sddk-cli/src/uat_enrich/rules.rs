//! Deterministic enrichment rules for building form items.
//!
//! Decision tree per `agents/uat-ux-form.md`:
//!
//! ```text
//! For each scenario without a form:
//! ├── Has plain_steps?
//! │   └── YES → per-step items: action instruction + semantic check
//! │       ├── Machine-checkable step → oracle check (Http/Json/Dom)
//! │       ├── UX subjective → human rating with scale anchors
//! │       ├── Expected textual observable → blind observation
//! │       └── Otherwise → human confirmation (fallback)
//! └── NO plain_steps → scenario-level decision (same as above)
//!
//! Every 5 items → insert Checkpoint
//! P0/P1 blocking checks → evidence_requirement: [Screenshot]
//! Provenance → author: "uat-ux-form" (set in enrich_scenario, not here)
//! ```

mod items;

use sddk_domain::{
    UatFormEvidenceKind as FEVK, UatFormItem, UatFormOracleKind as FOK, UatFormSpec,
    UatFormVisibility as FVIS, UatPriority, UatScenario, UatStep,
};

use items::{
    CheckConfig, insert_checkpoints, mk_check_item, mk_confirm_item, mk_info_item, mk_rating_item,
};

// ─── Public API ───────────────────────────────────────────────────────────────

/// Build form items for a scenario using deterministic rules.
pub fn build_form_for_scenario(scenario: &UatScenario) -> UatFormSpec {
    let p0_p1 = matches!(scenario.priority, UatPriority::P0 | UatPriority::P1);

    let items = if scenario.plain_steps.is_empty() {
        build_scenario_level_items(scenario, p0_p1)
    } else {
        build_per_step_items(scenario, p0_p1)
    };

    let items = insert_checkpoints(items);

    UatFormSpec {
        dsl_version: 1,
        items,
        completion: None,
    }
}

// ─── Scenario-level items (no steps) ────────────────────────────────────────

fn build_scenario_level_items(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    if let Some(items) = detect_machine_check_items(scenario, p0_p1) {
        return items;
    }
    if is_ux_subjective(scenario) {
        return build_rating_items_scenario(scenario, p0_p1);
    }
    if has_expected_textual(scenario) {
        return build_blind_items_scenario(scenario, p0_p1);
    }
    build_confirmation_items_scenario(scenario, p0_p1)
}

// ─── Per-step items (≥1 step → 2 items per step) ───────────────────────────

fn build_per_step_items(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    let mut items = Vec::new();

    for (idx, step) in scenario.plain_steps.iter().enumerate() {
        let step_num = idx + 1;

        // Info item: action instruction for this step
        items.push(mk_info_item(
            &format!("{}-step-{}-action", scenario.id, step_num),
            &step.action,
        ));

        // Check item: semantic check for this step
        if let Some(check_item) = build_check_for_step(scenario, step, step_num, p0_p1) {
            items.push(check_item);
        }
    }

    items
}

fn build_check_for_step(
    scenario: &UatScenario,
    step: &UatStep,
    step_num: usize,
    p0_p1: bool,
) -> Option<UatFormItem> {
    // Try machine check first
    if let Some(oracle) = detect_step_oracle(step) {
        let ev_required = if p0_p1 {
            vec![FEVK::Screenshot]
        } else {
            vec![]
        };
        // Machine checks: expected from oracle-specific rules
        let expected = match oracle {
            FOK::Http if !step.expected.is_empty() => Some(step.expected.clone()),
            FOK::Http => Some("HTTP response OK".to_string()),
            FOK::Json if !step.expected.is_empty() => Some(step.expected.clone()),
            FOK::Json => Some("JSON response matches".to_string()),
            FOK::Dom if !step.expected.is_empty() => Some(step.expected.clone()),
            FOK::Dom => Some("DOM element found".to_string()),
            _ => Some("Check passes".to_string()),
        };
        return Some(mk_check_item(CheckConfig {
            id: format!("{}-step-{}-check", scenario.id, step_num),
            prompt: format!("Verify step {} result", step_num),
            oracle: Some(oracle),
            visibility: FVIS::Visible,
            blocking: true,
            evidence_requirement: ev_required,
            expected,
        }));
    }

    // UX subjective check
    if is_ux_subjective(scenario) {
        let ev_required: Vec<FEVK> = if p0_p1 {
            vec![FEVK::Screenshot]
        } else {
            vec![]
        };
        return Some(mk_rating_item(
            &format!("{}-step-{}-rating", scenario.id, step_num),
            &format!("Rate step {} UX quality (1=Poor, 5=Excellent)", step_num),
            ev_required,
        ));
    }

    // Blind observation check
    if has_step_expected_textual(step) {
        let ev_required: Vec<FEVK> = if p0_p1 {
            vec![FEVK::Screenshot]
        } else {
            vec![]
        };
        return Some(mk_check_item(CheckConfig {
            id: format!("{}-step-{}-blind", scenario.id, step_num),
            prompt: format!("Observe and confirm step {} result", step_num),
            oracle: None,
            visibility: FVIS::Blind,
            blocking: true,
            evidence_requirement: ev_required,
            expected: Some(step.expected.clone()),
        }));
    }

    // Fallback: human confirmation
    let ev_required: Vec<FEVK> = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };
    // For fallback checks without oracle, use step's expected as the criterion
    let expected_for_fallback = if !step.expected.is_empty() {
        Some(step.expected.clone())
    } else {
        Some(format!("Step {} passes", step_num))
    };
    Some(mk_confirm_item(
        &format!("{}-step-{}-confirm", scenario.id, step_num),
        &format!("Confirm step {} passes", step_num),
        FVIS::Visible,
        p0_p1,
        ev_required,
        expected_for_fallback,
    ))
}

// ─── Machine check detection ─────────────────────────────────────────────────

fn detect_step_oracle(step: &UatStep) -> Option<FOK> {
    let action_lower = step.action.to_lowercase();

    if (action_lower.contains("http://")
        || action_lower.contains("https://")
        || action_lower.contains("/health")
        || action_lower.contains("/api/")
        || action_lower.contains("status code")
        || action_lower.contains("status_code")
        || action_lower.contains("response status")
        || action_lower.contains("status 200")
        || action_lower.contains("status 404")
        || action_lower.contains("response.status"))
        && !action_lower.contains("json")
    {
        return Some(FOK::Http);
    }

    let expected_lower = step.expected.to_lowercase();
    if (action_lower.contains("json") || action_lower.contains("/api/"))
        && (expected_lower.contains("json")
            || expected_lower.contains("body")
            || expected_lower.contains("field")
            || expected_lower.contains("property"))
    {
        return Some(FOK::Json);
    }

    if action_lower.contains("selector")
        || action_lower.contains("css selector")
        || action_lower.contains("xpath")
        || action_lower.contains("dom element")
        || (action_lower.contains("check") && action_lower.contains("element"))
    {
        return Some(FOK::Dom);
    }

    None
}

fn detect_machine_check_items(scenario: &UatScenario, p0_p1: bool) -> Option<Vec<UatFormItem>> {
    let oracle = scenario.plain_steps.iter().find_map(detect_step_oracle)?;

    let ev_required = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };

    // For scenario-level machine checks, use oracle-specific expected
    let expected = match oracle {
        FOK::Http => Some("HTTP response OK".to_string()),
        FOK::Json => Some("JSON response matches".to_string()),
        FOK::Dom => Some("DOM element found".to_string()),
        _ => Some("Check passes".to_string()),
    };

    Some(vec![
        mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
        mk_check_item(CheckConfig {
            id: format!("{}-machine-check", scenario.id),
            prompt: "Verify the expected result automatically".into(),
            oracle: Some(oracle),
            visibility: FVIS::Visible,
            blocking: true,
            evidence_requirement: ev_required,
            expected,
        }),
    ])
}

// ─── UX subjective detection ─────────────────────────────────────────────────

fn is_ux_subjective(scenario: &UatScenario) -> bool {
    let ux_keywords = [
        "helpful",
        "usability",
        "UX",
        "user experience",
        "intuitive",
        "easy to use",
        "design",
        "appearance",
        "look and feel",
        "color",
        "font",
        "layout",
        "navigate",
    ];

    let text_to_check = scenario
        .title
        .to_lowercase()
        .chars()
        .chain(
            scenario
                .context
                .as_ref()
                .and_then(|c| c.user_story.as_ref())
                .map(|s| s.to_lowercase())
                .unwrap_or_default()
                .chars(),
        )
        .collect::<String>();

    ux_keywords.iter().any(|kw| text_to_check.contains(kw))
}

fn has_step_expected_textual(step: &UatStep) -> bool {
    !step.expected.is_empty()
        && step.expected.len() > 3
        && !step.action.to_lowercase().contains("http://")
        && !step.action.to_lowercase().contains("https://")
        && !step.action.to_lowercase().contains("/api/")
        && !step.action.to_lowercase().contains("json")
        && !step.action.to_lowercase().contains("selector")
        && !step.action.to_lowercase().contains("dom")
}

fn has_expected_textual(scenario: &UatScenario) -> bool {
    !scenario.plain_steps.is_empty()
        && scenario.plain_steps.iter().any(has_step_expected_textual)
        && !is_ux_subjective(scenario)
}

// ─── Item builders ───────────────────────────────────────────────────────────

fn build_rating_items_scenario(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    let ev_required: Vec<FEVK> = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };

    vec![
        mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
        mk_rating_item(
            &format!("{}-rating", scenario.id),
            "Rate the UX quality (1=Poor, 5=Excellent)",
            ev_required,
        ),
        mk_confirm_item(
            &format!("{}-confirm", scenario.id),
            "Confirm the UX meets expectations",
            FVIS::Visible,
            p0_p1,
            vec![],
            Some("UX meets expectations".to_string()),
        ),
    ]
}

fn build_blind_items_scenario(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    let ev_required: Vec<FEVK> = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };

    // For scenario-level blind items, derive expected from the first step's expected
    let expected = scenario
        .plain_steps
        .iter()
        .find_map(|s| {
            if has_step_expected_textual(s) {
                Some(s.expected.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Expected result confirmed".to_string());

    vec![
        mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
        mk_check_item(CheckConfig {
            id: format!("{}-blind-check", scenario.id),
            prompt: "Observe and confirm the result".into(),
            oracle: None,
            visibility: FVIS::Blind,
            blocking: true,
            evidence_requirement: ev_required,
            expected: Some(expected),
        }),
    ]
}

fn build_confirmation_items_scenario(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    let ev_required: Vec<FEVK> = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };

    vec![
        mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
        mk_confirm_item(
            &format!("{}-confirm", scenario.id),
            "Verify this scenario passes",
            FVIS::Visible,
            p0_p1,
            ev_required,
            Some(scenario.title.clone()),
        ),
    ]
}
