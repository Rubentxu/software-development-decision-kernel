//! E14.2 — 13-smell detector implementation.
//!
//! 13 deterministic smell categories (no LLM):
//! 1.  EXPECTED_ABSENT      — BLOCKER
//! 2.  AMBIGUOUS_INSTRUCTION — WARNING
//! 3.  MACHINE_OBSERVABLE   — WARNING
//! 4.  DUPLICATED_CHECK     — WARNING
//! 5.  NO_RECOVERY_PATH     — WARNING
//! 6.  LEADING_QUESTION     — WARNING
//! 7.  SUBJECTIVE_NO_SCALE  — WARNING
//! 8.  FAIL_NO_EVIDENCE     — BLOCKER
//! 9.  STEP_TOO_LARGE      — WARNING
//! 10. EXCESSIVE_STEPS      — WARNING
//! 11. HIDDEN_PREREQUISITE  — WARNING
//! 12. NO_BRANCHING         — WARNING
//! 13. BLIND_CHECK_WITHOUT_HIDDEN — WARNING

use sddk_domain::UatFormElementKind as FEK;
use sddk_domain::UatFormFlowKind as FFK;
use sddk_domain::UatFormInputKind as FIK;
use sddk_domain::UatFormVisibility as FVIS;
use sddk_domain::{UatPlan, UatScenario};

use super::report::{QualityReport, QualitySmell, QualitySummary, QualityThreshold, SmellLocation};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Detect all 13 smell categories in a UAT plan.
pub fn detect_13_smells(plan: &UatPlan, threshold: QualityThreshold) -> QualityReport {
    let mut smells = Vec::new();
    let mut smell_id = 1u32;

    for feature in &plan.features {
        for scenario in &feature.scenarios {
            detect_scenario_smells(scenario, &mut smells, &mut smell_id, &feature.id);
        }
    }

    let blockers = smells.iter().filter(|s| s.severity == "BLOCKER").count();
    let warnings = smells.iter().filter(|s| s.severity == "WARNING").count();
    let total = smell_id.saturating_sub(1);

    let pass = blockers == 0
        && match threshold {
            QualityThreshold::Blocker => true,
            QualityThreshold::Warning => warnings == 0,
        };

    QualityReport {
        schema_version: 1,
        analyzer: "uat-form-quality".into(),
        model: "heuristic-v1".into(),
        analyzed_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("RFC 3339 formatting cannot fail"),
        plan_ref: String::new(),
        smells,
        summary: QualitySummary {
            total,
            blockers: blockers as u32,
            errors: 0,
            warnings: warnings as u32,
            suggestions: 0,
            pass,
        },
        verdict: if pass {
            "PASS".into()
        } else {
            "NEEDS_REVISION".into()
        },
        threshold_applied: match threshold {
            QualityThreshold::Blocker => "BLOCKER".into(),
            QualityThreshold::Warning => "WARNING".into(),
        },
    }
}

fn detect_scenario_smells(
    scenario: &UatScenario,
    smells: &mut Vec<QualitySmell>,
    smell_id: &mut u32,
    feature_id: &str,
) {
    let Some(form) = &scenario.form else {
        return;
    };

    let preconditions_text = scenario.preconditions.join(" ").to_lowercase();

    for item in &form.items {
        if item.kind == FEK::Check {
            let check = item.check.as_ref();

            // 1. EXPECTED_ABSENT — BLOCKER
            let has_expected = check.as_ref().and_then(|c| c.expected.as_ref()).is_some();
            let has_oracle = check.as_ref().and_then(|c| c.oracle.as_ref()).is_some();
            if !has_expected && !has_oracle {
                push_smell(
                    smells,
                    smell_id,
                    "EXPECTED_ABSENT",
                    "BLOCKER",
                    feature_id,
                    &scenario.id,
                    item.id.as_deref(),
                    None,
                    "Add check.expected or check.oracle",
                );
            }

            // 2. AMBIGUOUS_INSTRUCTION — WARNING
            if let Some(prompt) = check.as_ref().map(|c| c.prompt.as_str()) {
                let vague = [
                    "correcto",
                    "adecuado",
                    "bien",
                    "normal",
                    "apropiado",
                    "razonable",
                ];
                if vague.iter().any(|v| prompt.to_lowercase().contains(v)) {
                    push_smell(
                        smells,
                        smell_id,
                        "AMBIGUOUS_INSTRUCTION",
                        "WARNING",
                        feature_id,
                        &scenario.id,
                        item.id.as_deref(),
                        Some(&prompt[..prompt.len().min(60)]),
                        "Replace vague term with operational criterion",
                    );
                }
            }

            // 3. MACHINE_OBSERVABLE — WARNING
            if let Some(prompt) = check.as_ref().map(|c| c.prompt.as_str()) {
                let machine_terms = [
                    "http", "status", "dom", "element", "response", "api", "json", "header",
                ];
                let has_machine = machine_terms
                    .iter()
                    .any(|t| prompt.to_lowercase().contains(t));
                if has_machine && has_oracle {
                    push_smell(
                        smells,
                        smell_id,
                        "MACHINE_OBSERVABLE",
                        "WARNING",
                        feature_id,
                        &scenario.id,
                        item.id.as_deref(),
                        None,
                        "Machine can verify this — use oracle instead of human prompt",
                    );
                }
            }

            // 6. LEADING_QUESTION — WARNING
            if let Some(prompt) = check.as_ref().map(|c| c.prompt.as_str()) {
                let leading_triggers = ["¿es", "¿está", "¿tiene", "¿no "];
                let lower = prompt.to_lowercase();
                if leading_triggers.iter().any(|t| lower.contains(t)) && lower.contains('?') {
                    push_smell(
                        smells,
                        smell_id,
                        "LEADING_QUESTION",
                        "WARNING",
                        feature_id,
                        &scenario.id,
                        item.id.as_deref(),
                        None,
                        "Question suggests desired answer — rephrase neutrally",
                    );
                }
            }

            // 7. SUBJECTIVE_NO_SCALE — WARNING
            let kind = check.as_ref().map(|c| c.kind).unwrap_or(FIK::Confirm);
            if matches!(kind, FIK::Rating | FIK::Text) {
                let has_options = check
                    .as_ref()
                    .map(|c| !c.options.is_empty())
                    .unwrap_or(false);
                if !has_options {
                    push_smell(
                        smells,
                        smell_id,
                        "SUBJECTIVE_NO_SCALE",
                        "WARNING",
                        feature_id,
                        &scenario.id,
                        item.id.as_deref(),
                        None,
                        "Subjective check needs a scale (options/anchors on the input kind)",
                    );
                }
            }

            // 8. FAIL_NO_EVIDENCE — BLOCKER
            let ev_required = check
                .as_ref()
                .map(|c| !c.evidence_requirement.is_empty())
                .unwrap_or(false);
            let is_blocking = check.as_ref().map(|c| c.blocking).unwrap_or(true);
            if is_blocking && !ev_required {
                push_smell(
                    smells,
                    smell_id,
                    "FAIL_NO_EVIDENCE",
                    "BLOCKER",
                    feature_id,
                    &scenario.id,
                    item.id.as_deref(),
                    None,
                    "Add evidence_requirement (e.g. screenshot) to blocking checks",
                );
            }

            // 13. BLIND_CHECK_WITHOUT_HIDDEN — WARNING
            if check
                .as_ref()
                .map(|c| c.visibility == FVIS::Blind)
                .unwrap_or(false)
            {
                let has_hidden_expected =
                    check.as_ref().and_then(|c| c.expected.as_ref()).is_some();
                if !has_hidden_expected {
                    push_smell(
                        smells,
                        smell_id,
                        "BLIND_CHECK_WITHOUT_HIDDEN",
                        "WARNING",
                        feature_id,
                        &scenario.id,
                        item.id.as_deref(),
                        None,
                        "Blind checks require check.expected to be set",
                    );
                }
            }
        }

        // 9. STEP_TOO_LARGE — WARNING (runs for ALL item kinds, not just Check)
        if let Some(text) = item.text.as_ref() {
            let separators = &[',', 'y'];
            let count = separators
                .iter()
                .fold(0, |acc, sep| acc + text.matches(*sep).count());
            if count > 3 {
                push_smell(
                    smells,
                    smell_id,
                    "STEP_TOO_LARGE",
                    "WARNING",
                    feature_id,
                    &scenario.id,
                    item.id.as_deref(),
                    None,
                    "Step has >3 distinct actions — split into separate items",
                );
            }
        }
    }

    // 10. EXCESSIVE_STEPS — WARNING (>12 items without checkpoint)
    if form.items.len() > 12 {
        let has_checkpoint = form.items.iter().any(|i| i.kind == FEK::Checkpoint);
        if !has_checkpoint {
            push_smell(
                smells,
                smell_id,
                "EXCESSIVE_STEPS",
                "WARNING",
                feature_id,
                &scenario.id,
                None,
                None,
                &format!(
                    "Scenario has {} items — add checkpoints every ~5 items",
                    form.items.len()
                ),
            );
        }
    }

    // 5. NO_RECOVERY_PATH — WARNING
    // Heuristic: 2+ blocking checks but none have recovery flow (Retry/Block/Repeat/Branch)
    let blocking_checks: Vec<_> = form
        .items
        .iter()
        .filter(|i| i.kind == FEK::Check && i.check.as_ref().map(|c| c.blocking).unwrap_or(true))
        .collect();
    if blocking_checks.len() >= 2 {
        let has_recovery = blocking_checks.iter().any(|i| {
            matches!(
                i.flow,
                Some(FFK::Retry | FFK::Block | FFK::Repeat | FFK::Branch)
            )
        });
        if !has_recovery {
            push_smell(
                smells,
                smell_id,
                "NO_RECOVERY_PATH",
                "WARNING",
                feature_id,
                &scenario.id,
                None,
                None,
                "Multiple blocking checks with no Retry/Block/Repeat/Branch flow",
            );
        }
    }

    // 11. HIDDEN_PREREQUISITE — WARNING (env var in text but not in preconditions)
    // Always check, regardless of whether preconditions is empty
    for item in &form.items {
        let text_to_check = item
            .text
            .as_deref()
            .or_else(|| item.check.as_ref().map(|c| c.prompt.as_str()))
            .unwrap_or("");

        for word in text_to_check.split_whitespace() {
            if word.starts_with('$')
                && word
                    .chars()
                    .nth(1)
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                let var_upper = word.trim_start_matches('$').to_uppercase();
                // Check if this var is declared in preconditions
                let declared =
                    preconditions_text.contains(&var_upper) || preconditions_text.contains(word);
                if !declared {
                    push_smell(
                        smells,
                        smell_id,
                        "HIDDEN_PREREQUISITE",
                        "WARNING",
                        feature_id,
                        &scenario.id,
                        item.id.as_deref(),
                        None,
                        &format!("Uses {word} not declared in preconditions"),
                    );
                    break; // Only one smell per scenario for this category
                }
            }
        }
    }

    // 12. NO_BRANCHING — WARNING
    let check_count = form.items.iter().filter(|i| i.kind == FEK::Check).count();
    let has_flow = form.items.iter().any(|i| i.flow.is_some());
    if check_count > 3 && !has_flow {
        push_smell(
            smells,
            smell_id,
            "NO_BRANCHING",
            "WARNING",
            feature_id,
            &scenario.id,
            None,
            None,
            ">3 checks with no flow.goto — consider branching for error paths",
        );
    }

    // 4. DUPLICATED_CHECK — WARNING
    // Heuristic: consecutive checks with same oracle+expected
    let items = &form.items;
    for i in 0..items.len() {
        if items[i].kind != FEK::Check {
            continue;
        }
        let check_i = items[i].check.as_ref();
        let (oracle_i, expected_i) = (
            check_i.and_then(|c| c.oracle.map(|o| format!("{:?}", o))),
            check_i.and_then(|c| c.expected.clone()),
        );
        for item_j in items.iter().skip(i + 1).take(3) {
            if item_j.kind != FEK::Check {
                continue;
            }
            let check_j = item_j.check.as_ref();
            let (oracle_j, expected_j) = (
                check_j.and_then(|c| c.oracle.map(|o| format!("{:?}", o))),
                check_j.and_then(|c| c.expected.clone()),
            );
            if oracle_i == oracle_j && expected_i == expected_j {
                push_smell(
                    smells,
                    smell_id,
                    "DUPLICATED_CHECK",
                    "WARNING",
                    feature_id,
                    &scenario.id,
                    item_j.id.as_deref(),
                    None,
                    "Duplicate check — same oracle and expected as another check",
                );
                break;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_smell(
    smells: &mut Vec<QualitySmell>,
    smell_id: &mut u32,
    smell_id_str: &str,
    severity: &str,
    feature_id: &str,
    scenario_id: &str,
    item_id: Option<&str>,
    snippet: Option<&str>,
    suggestion: &str,
) {
    smells.push(QualitySmell {
        id: format!("FQ-{:03}", smell_id),
        smell_id: smell_id_str.into(),
        severity: severity.into(),
        location: SmellLocation {
            feature_id: feature_id.into(),
            scenario_id: scenario_id.into(),
            item_id: item_id.map(String::from),
            field: None,
        },
        snippet: snippet.map(String::from),
        suggestion: suggestion.into(),
        auto_fixable: false,
    });
    *smell_id += 1;
}
