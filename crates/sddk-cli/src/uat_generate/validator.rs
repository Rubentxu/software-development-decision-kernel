//! E14.5 — Input validation for the generate pipeline.
//!
//! Validates all inputs before any file system operations (atomic write rule).
//! Requirements: dir required OR if optional must provide ≥1 criterion.
//! Changelog/last-plan: must exist if provided.
//! Discovery: --discover requires --app-url.

use std::path::{Path, PathBuf};

/// Validation errors for generate pipeline inputs.
#[derive(Debug)]
pub enum ValidateError {
    /// Requirements directory is required but missing.
    RequirementsRequired,
    /// Requirements directory doesn't exist or isn't a directory.
    RequirementsNotDir(PathBuf),
    /// No criteria found: requirements dir exists but no markdown files with content.
    NoCriteriaFound(PathBuf),
    /// Changelog file was provided but doesn't exist.
    ChangelogNotFound(PathBuf),
    /// Last-plan file was provided but doesn't exist.
    LastPlanNotFound(PathBuf),
    /// Discovery requested but --app-url is missing.
    DiscoverRequiresAppUrl,
}

impl std::fmt::Display for ValidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidateError::RequirementsRequired => {
                write!(f, "requirements directory is required but missing")
            }
            ValidateError::RequirementsNotDir(path) => {
                write!(
                    f,
                    "requirements path is not a directory: {}",
                    path.display()
                )
            }
            ValidateError::NoCriteriaFound(path) => {
                write!(
                    f,
                    "no criteria found in requirements directory: {}",
                    path.display()
                )
            }
            ValidateError::ChangelogNotFound(path) => {
                write!(f, "changelog file not found: {}", path.display())
            }
            ValidateError::LastPlanNotFound(path) => {
                write!(f, "last-plan file not found: {}", path.display())
            }
            ValidateError::DiscoverRequiresAppUrl => {
                write!(f, "--discover requires --app-url")
            }
        }
    }
}

/// Validate all generate pipeline inputs before any file operations.
/// Returns Ok if inputs are valid, Err with specific error otherwise.
/// Does NOT write any files (atomic write rule: validate before write).
///
/// Source presence rule (RED phase):
/// - If requirements is explicitly provided (even if empty dir), validate it
/// - Otherwise (None), at least one of changelog/last_plan/discover must be present
pub fn validate_inputs(
    requirements: &Option<PathBuf>,
    changelog: &Option<PathBuf>,
    last_plan: &Option<PathBuf>,
    discover: bool,
    app_url: &Option<String>,
) -> Result<(), ValidateError> {
    // (A) Source presence: if requirements is None, at least one other source must be present
    let has_changelog = changelog.as_ref().map(|cl| cl.exists()).unwrap_or(false);
    let has_last_plan = last_plan.as_ref().map(|lp| lp.exists()).unwrap_or(false);

    if requirements.is_none() && !has_changelog && !has_last_plan && !discover {
        return Err(ValidateError::RequirementsRequired);
    }

    // (A) Requirements validation (when provided, must be valid dir with content)
    if let Some(req_dir) = requirements {
        if !req_dir.is_dir() {
            return Err(ValidateError::RequirementsNotDir(req_dir.clone()));
        }
        if count_criteria_in_dir(req_dir) == 0 {
            return Err(ValidateError::NoCriteriaFound(req_dir.clone()));
        }
    }

    // (A) Changelog: must exist if provided
    if let Some(cl) = changelog
        && !cl.exists()
    {
        return Err(ValidateError::ChangelogNotFound(cl.clone()));
    }

    // (A) Last-plan: must exist if provided
    if let Some(lp) = last_plan
        && !lp.exists()
    {
        return Err(ValidateError::LastPlanNotFound(lp.clone()));
    }

    // (A) Discovery requires app-url
    if discover && app_url.is_none() {
        return Err(ValidateError::DiscoverRequiresAppUrl);
    }

    Ok(())
}

/// Count markdown files with actual content (headings/bullets) in a directory.
/// Returns 0 if no criteria found.
fn count_criteria_in_dir(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md")
                && let Ok(content) = std::fs::read_to_string(&path)
                && has_criteria_content(&content)
            {
                count += 1;
            }
        }
    }
    count
}

/// Check if markdown content has headings or bullets (criterion markers).
fn has_criteria_content(content: &str) -> bool {
    // Look for heading lines (# or ##) or bullet points (- or *)
    content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.starts_with('*')
            || trimmed.starts_with("1.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_requires_requirements_or_alternative() {
        let _td = TempDir::new().unwrap();

        // Task 1 RED phase: validate_inputs MUST Err when all inputs are None
        // (no requirements, no changelog, no last_plan, no discover).
        // The check that at least one criterion source is provided happens HERE,
        // not deferred to build_plan.
        let result = validate_inputs(&None, &None, &None, false, &None);
        assert!(matches!(result, Err(ValidateError::RequirementsRequired)));
    }

    #[test]
    fn validate_requirements_not_dir() {
        let td = TempDir::new().unwrap();
        let not_a_dir = td.path().join("notadir");

        let result = validate_inputs(&Some(not_a_dir), &None, &None, false, &None);
        assert!(matches!(result, Err(ValidateError::RequirementsNotDir(_))));
    }

    #[test]
    fn validate_empty_requirements_dir() {
        let td = TempDir::new().unwrap();

        let result = validate_inputs(&Some(td.path().to_path_buf()), &None, &None, false, &None);
        assert!(matches!(result, Err(ValidateError::NoCriteriaFound(_))));
    }

    #[test]
    fn validate_requirements_with_md_file() {
        let td = TempDir::new().unwrap();
        let req_dir = td.path();

        // Create a markdown file with heading
        std::fs::write(
            req_dir.join("req.md"),
            "# Requisitos\n\n- Feature 1\n- Feature 2\n",
        )
        .unwrap();

        let result = validate_inputs(&Some(req_dir.to_path_buf()), &None, &None, false, &None);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_changelog_not_found() {
        let td = TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(req_dir.join("req.md"), "# Requisitos\n- Feature 1\n").unwrap();

        let missing_changelog = td.path().join("CHANGELOG.md");
        let result = validate_inputs(
            &Some(req_dir.to_path_buf()),
            &Some(missing_changelog),
            &None,
            false,
            &None,
        );
        assert!(matches!(result, Err(ValidateError::ChangelogNotFound(_))));
    }

    #[test]
    fn validate_changelog_exists() {
        let td = TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(req_dir.join("req.md"), "# Requisitos\n- Feature 1\n").unwrap();

        let changelog = td.path().join("CHANGELOG.md");
        std::fs::write(&changelog, "## Added\n- Feature 1\n").unwrap();

        let result = validate_inputs(
            &Some(req_dir.to_path_buf()),
            &Some(changelog),
            &None,
            false,
            &None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_discover_without_app_url() {
        let td = TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(req_dir.join("req.md"), "# Requisitos\n- Feature 1\n").unwrap();

        let result = validate_inputs(&Some(req_dir.to_path_buf()), &None, &None, true, &None);
        assert!(matches!(result, Err(ValidateError::DiscoverRequiresAppUrl)));
    }

    #[test]
    fn validate_discover_with_app_url() {
        let td = TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(req_dir.join("req.md"), "# Requisitos\n- Feature 1\n").unwrap();

        let result = validate_inputs(
            &Some(req_dir.to_path_buf()),
            &None,
            &None,
            true,
            &Some("http://localhost:3000".to_string()),
        );
        assert!(result.is_ok());
    }
}
