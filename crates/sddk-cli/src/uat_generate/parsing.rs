//! E14.5 — Text parsing utilities for the generate pipeline.
//!
//! Pure functions for extracting criteria from markdown, parsing changelogs,
//! and extracting REQ IDs from text.

/// Extract criterion lines from a markdown file (headings + bullets).
pub fn extract_criteria_from_md(content: &str) -> Vec<String> {
    let mut criteria = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // Extract ## headings as high-level criteria
        if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            criteria.push(trimmed.trim_start_matches('#').trim().to_string());
        }
        // Extract bullet points
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            criteria.push(
                trimmed
                    .trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim()
                    .to_string(),
            );
        }
        // Extract numbered list items
        if trimmed.len() > 2
            && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
            && trimmed.contains('.')
        {
            criteria.push(
                trimmed
                    .split_once('.')
                    .map(|x| x.1.trim())
                    .unwrap_or(trimmed)
                    .to_string(),
            );
        }
    }
    criteria
}

/// Parse Added/Changed sections from a changelog.
pub fn parse_changelog_sections(content: &str) -> (Vec<String>, Vec<String>) {
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## Added") || trimmed.eq_ignore_ascii_case("### Added") {
            current_section = "added".to_string();
        } else if trimmed.eq_ignore_ascii_case("## Changed")
            || trimmed.eq_ignore_ascii_case("### Changed")
        {
            current_section = "changed".to_string();
        } else if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
            current_section = String::new();
        } else if !current_section.is_empty()
            && (trimmed.starts_with("- ") || trimmed.starts_with("* "))
        {
            let text = trimmed
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim()
                .to_string();
            if current_section == "added" {
                added.push(text);
            } else if current_section == "changed" {
                changed.push(text);
            }
        }
    }
    (added, changed)
}

/// Extract REQ ids from text (e.g., "REQ-001", "RF-016").
pub fn extract_req_ids(content: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for word in content.split_whitespace() {
        let word = word.trim_end_matches(['.', ',', ':', ')']);
        if (word.starts_with("REQ") || word.starts_with("RF"))
            && word.len() > 3
            && word.chars().skip(3).all(|c| c.is_ascii_digit() || c == '-')
        {
            ids.push(word.to_string());
        }
    }
    ids
}

/// Build scenario title from criterion text.
pub fn scenario_title_from_criterion(criterion: &str) -> String {
    let mut title = criterion.to_string();
    if !title.is_empty() {
        let mut chars = title.chars();
        if let Some(first) = chars.next() {
            title = first.to_uppercase().collect::<String>() + chars.as_str();
        }
    }
    if title.len() > 80 {
        title.truncate(77);
        title.push_str("...");
    }
    title
}

/// Build a UatStep from plain text.
pub fn step_from_text(text: &str) -> sddk_domain::UatStep {
    sddk_domain::UatStep {
        action: text.to_string(),
        copy_hint: false,
        expected: String::new(),
        step: None,
        kind: None,
        vs_expected_check: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_criteria_from_md_headings() {
        let content = "# Requirements\n\n## Login Feature\n- User can login\n- User can logout\n\n### API Section\n* API returns JSON\n";
        let criteria = extract_criteria_from_md(content);
        assert!(criteria.iter().any(|c| c == "Login Feature"));
        assert!(criteria.iter().any(|c| c == "API Section"));
        assert!(criteria.iter().any(|c| c == "User can login"));
        assert!(criteria.iter().any(|c| c == "User can logout"));
        assert!(criteria.iter().any(|c| c == "API returns JSON"));
    }

    #[test]
    fn test_extract_criteria_from_md_numbered() {
        let content = "## Login\n1. Enter credentials\n2. Submit form\n";
        let criteria = extract_criteria_from_md(content);
        assert!(criteria.iter().any(|c| c == "Enter credentials"));
        assert!(criteria.iter().any(|c| c == "Submit form"));
    }

    #[test]
    fn test_parse_changelog_sections_added_changed() {
        let content = "## Added\n- Feature A\n- Feature B\n\n## Changed\n- Bug fix C\n";
        let (added, changed) = parse_changelog_sections(content);
        assert_eq!(added.len(), 2);
        assert!(added.contains(&"Feature A".to_string()));
        assert!(added.contains(&"Feature B".to_string()));
        assert_eq!(changed.len(), 1);
        assert!(changed.contains(&"Bug fix C".to_string()));
    }

    #[test]
    fn test_extract_req_ids() {
        let content = "REQ-001, REQ-002, RF-016, REQ-003\n";
        let ids = extract_req_ids(content);
        assert!(ids.contains(&"REQ-001".to_string()));
        assert!(ids.contains(&"REQ-002".to_string()));
        assert!(ids.contains(&"RF-016".to_string()));
        assert!(ids.contains(&"REQ-003".to_string()));
    }

    #[test]
    fn test_scenario_title_from_criterion() {
        assert_eq!(
            scenario_title_from_criterion("user can login"),
            "User can login"
        );
        assert_eq!(
            scenario_title_from_criterion("api returns json"),
            "Api returns json"
        );
    }

    #[test]
    fn test_scenario_title_truncation() {
        let long_criterion = "a".repeat(100);
        let title = scenario_title_from_criterion(&long_criterion);
        assert!(title.len() <= 80);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_step_from_text() {
        let step = step_from_text("Enter credentials");
        assert_eq!(step.action, "Enter credentials");
        assert!(!step.copy_hint);
        assert!(step.expected.is_empty());
    }
}
