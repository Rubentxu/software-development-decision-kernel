//! Item constructors and checkpoint insertion for enrichment rules.

use sddk_domain::{
    UatFormCheck, UatFormElementKind as FEK, UatFormEvidenceKind as FEVK, UatFormInputKind as FIK,
    UatFormItem, UatFormOracleKind as FOK, UatFormVisibility as FVIS,
};

pub struct CheckConfig {
    pub id: String,
    pub prompt: String,
    pub oracle: Option<FOK>,
    pub visibility: FVIS,
    pub blocking: bool,
    pub evidence_requirement: Vec<FEVK>,
    pub expected: Option<String>,
}

pub fn mk_info_item(id: &str, text: &str) -> UatFormItem {
    UatFormItem {
        kind: FEK::Info,
        id: Some(id.to_string()),
        check: None,
        text: Some(text.to_string()),
        flow: None,
        target: None,
        checkpoint: None,
    }
}

pub fn mk_check_item(cfg: CheckConfig) -> UatFormItem {
    UatFormItem {
        kind: FEK::Check,
        id: Some(cfg.id),
        check: Some(UatFormCheck {
            kind: FIK::Confirm,
            prompt: cfg.prompt,
            oracle: cfg.oracle,
            visibility: cfg.visibility,
            required: cfg.blocking,
            blocking: cfg.blocking,
            confidence_requirement: None,
            evidence_requirement: cfg.evidence_requirement,
            comment_required_when: None,
            options: vec![],
            expected: cfg.expected,
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

pub fn mk_rating_item(id: &str, prompt: &str, evidence_requirement: Vec<FEVK>) -> UatFormItem {
    UatFormItem {
        kind: FEK::Check,
        id: Some(id.to_string()),
        check: Some(UatFormCheck {
            kind: FIK::Rating,
            prompt: prompt.to_string(),
            oracle: None,
            visibility: FVIS::Visible,
            required: true,
            blocking: true,
            confidence_requirement: None,
            evidence_requirement,
            comment_required_when: Some("below_3".to_string()),
            options: vec![
                "1 - Poor".to_string(),
                "2 - Below Average".to_string(),
                "3 - Average".to_string(),
                "4 - Good".to_string(),
                "5 - Excellent".to_string(),
            ],
            expected: None,
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

pub fn mk_confirm_item(
    id: &str,
    prompt: &str,
    visibility: FVIS,
    p0_p1: bool,
    evidence_requirement: Vec<FEVK>,
    expected: Option<String>,
) -> UatFormItem {
    let ev = if p0_p1 && evidence_requirement.is_empty() {
        vec![FEVK::Screenshot]
    } else {
        evidence_requirement
    };

    UatFormItem {
        kind: FEK::Check,
        id: Some(id.to_string()),
        check: Some(UatFormCheck {
            kind: FIK::Confirm,
            prompt: prompt.to_string(),
            oracle: None,
            visibility,
            required: true,
            blocking: true,
            confidence_requirement: None,
            evidence_requirement: ev,
            comment_required_when: None,
            options: vec![],
            expected,
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

pub fn insert_checkpoints(items: Vec<UatFormItem>) -> Vec<UatFormItem> {
    if items.len() <= 5 {
        return items;
    }

    let mut result = Vec::with_capacity(items.len() * 2);
    let mut count = 0;

    for item in &items {
        result.push(item.clone());
        count += 1;

        // Insert checkpoint after every 5 items (but not at the very end)
        if count % 5 == 0 && count < items.len() {
            result.push(UatFormItem {
                kind: FEK::Checkpoint,
                id: Some(format!("cp-{}", count)),
                check: None,
                text: Some(format!("Checkpoint after {} items", count)),
                flow: None,
                target: None,
                checkpoint: Some(sddk_domain::UatCheckpoint {
                    id: format!("cp-{}", count),
                    label: Some(format!("Checkpoint after {} items", count)),
                    evidence_summary: sddk_domain::UatEvidenceSummary::default(),
                    items: vec![],
                }),
            });
        }
    }

    result
}
