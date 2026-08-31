//! Thin runner for `sddk uat discover` — wires CLI args to the discover module.
//!
//! This module is the thin connection layer between the clap args in `uat.rs`
//! and the core discovery logic in `parser.rs` / `aam.rs`.

use std::path::PathBuf;

use crate::CommandOutput;
use crate::failure_envelope;
use crate::uat::DiscoverArgs;

/// Run the discover command: explore app with Fara and produce AAM.
/// Warnings (e.g., Fara unreachable) are included in stdout per spec.
pub fn run(args: DiscoverArgs) -> CommandOutput {
    match super::discover(&args) {
        Ok(outcome) => {
            let output_path = args
                .output
                .clone()
                .unwrap_or_else(|| PathBuf::from("discovered-flows.yaml"));

            match super::render_discovery_output(&outcome, &args.goals, &output_path) {
                Ok(stdout) => CommandOutput {
                    status: 0,
                    stdout,
                    stderr: String::new(),
                },
                Err(e) => failure_envelope(&e),
            }
        }
        Err(e) => failure_envelope(&e),
    }
}

#[cfg(test)]
mod tests {
    use crate::uat_discover::aam::{AamModel, AamPage};
    use crate::uat_discover::parser::ParseResult;
    use crate::uat_discover::{DiscoveryOutcome, computer_use_command_args, merge_successful_runs};

    #[test]
    fn test_computer_use_command_args_structure() {
        let args = computer_use_command_args(
            "http://app.example.com",
            "login as admin",
            "/tmp/output",
            "http://127.0.0.1:8082",
            25,
        );

        // Verify sequence: [--url, app_url, --goal, goal, --output, output, --fara-url, fara_url, --max-steps, budget]
        assert_eq!(args.len(), 10);
        assert_eq!(args[0], "--url");
        assert_eq!(args[1], "http://app.example.com");
        assert_eq!(args[2], "--goal");
        assert_eq!(args[3], "login as admin");
        assert_eq!(args[4], "--output");
        assert_eq!(args[5], "/tmp/output");
        assert_eq!(args[6], "--fara-url");
        assert_eq!(args[7], "http://127.0.0.1:8082");
        assert_eq!(args[8], "--max-steps");
        assert_eq!(args[9], "25");
    }

    #[test]
    fn test_merge_successful_runs_empty() {
        let results: Vec<ParseResult> = vec![];
        let merged = merge_successful_runs(results);
        assert!(merged.pages.is_empty());
        assert!(merged.flows.is_empty());
        assert!(merged.screenshots.is_empty());
        assert!(merged.urls.is_empty());
    }

    #[test]
    fn test_merge_successful_runs_multiple() {
        use crate::uat_discover::aam::{AamFlow, AamPage};

        let result1 = ParseResult {
            pages: vec![AamPage {
                id: "page-a".into(),
                path: "/login".into(),
                title: "Login".into(),
                semantic: "login page".into(),
                url_snapshot: "http://app/login".into(),
                elements: vec![],
            }],
            flows: vec![AamFlow {
                id: "flow-a".into(),
                semantic: "login flow".into(),
                pages: vec!["/login".into()],
                steps: vec![],
                trajectory_hash: None,
            }],
            screenshots: vec!["screenshot-01.png".into()],
            urls: vec!["http://app/login".into()],
        };

        let result2 = ParseResult {
            pages: vec![AamPage {
                id: "page-b".into(),
                path: "/dashboard".into(),
                title: "Dashboard".into(),
                semantic: "dashboard page".into(),
                url_snapshot: "http://app/dashboard".into(),
                elements: vec![],
            }],
            flows: vec![],
            screenshots: vec!["screenshot-02.png".into()],
            urls: vec!["http://app/dashboard".into()],
        };

        let merged = merge_successful_runs(vec![result1, result2]);

        assert_eq!(merged.pages.len(), 2);
        assert_eq!(merged.flows.len(), 1);
        assert_eq!(merged.screenshots.len(), 2);
        assert_eq!(merged.urls.len(), 2);
    }

    #[test]
    fn test_validate_successful_artifacts_rejects_empty() {
        use crate::uat_discover::validate_successful_artifacts;

        let empty = ParseResult::default();
        let result = validate_successful_artifacts(&empty);
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("empty AAM after successful run"),
            "expected 'empty AAM after successful run' in error, got: {err_msg}"
        );
    }

    #[test]
    fn test_validate_successful_artifacts_accepts_non_empty() {
        use crate::uat_discover::validate_successful_artifacts;

        let with_pages = ParseResult {
            pages: vec![AamPage {
                id: "p1".into(),
                path: "/".into(),
                title: "Home".into(),
                semantic: String::new(),
                url_snapshot: String::new(),
                elements: vec![],
            }],
            flows: vec![],
            screenshots: vec![],
            urls: vec![],
        };
        assert!(validate_successful_artifacts(&with_pages).is_ok());
    }

    #[test]
    fn test_discovery_outcome_with_warning() {
        let aam = AamModel::fallback("http://app", "/login");
        let outcome = DiscoveryOutcome {
            aam,
            warning: Some("Fara not reachable at http://127.0.0.1:8082".into()),
        };

        assert!(outcome.warning.is_some());
        assert!(outcome.aam.urls.contains(&"/login".to_string()));
    }

    #[test]
    fn test_discovery_outcome_without_warning() {
        let aam = AamModel::fallback("http://app", "/");
        let outcome = DiscoveryOutcome { aam, warning: None };

        assert!(outcome.warning.is_none());
    }

    #[test]
    fn test_parse_result_default_is_empty() {
        let result = ParseResult::default();
        assert!(result.pages.is_empty());
        assert!(result.flows.is_empty());
        assert!(result.screenshots.is_empty());
        assert!(result.urls.is_empty());
    }

    #[test]
    fn test_aam_model_fallback_has_unreachable_fara() {
        let aam = AamModel::fallback("http://app", "/");
        assert_eq!(aam.app.fara_version, "unreachable");
        assert_eq!(aam.provenance.fallback, Some("no-fara".into()));
        assert!(aam.pages.is_empty());
        assert!(aam.flows.is_empty());
    }

    #[test]
    fn test_aam_model_fallback_has_urls() {
        let aam = AamModel::fallback("http://app", "/login");
        assert!(aam.urls.contains(&"/login".to_string()));
    }

    #[test]
    fn test_render_discovery_output_fara_unreachable_warning() {
        use crate::uat_discover::render_discovery_output;
        use tempfile::NamedTempFile;

        let aam = AamModel::fallback("http://app.example.com", "/");
        let outcome = DiscoveryOutcome {
            aam,
            warning: Some("Fara not reachable at http://127.0.0.1:8082".into()),
        };

        let temp_file = NamedTempFile::with_suffix(".yaml").unwrap();
        let path = temp_file.path().to_path_buf();

        let stdout = render_discovery_output(&outcome, &["login goal".into()], &path).unwrap();

        // Exactly the WARN line first
        assert!(
            stdout.starts_with("WARN: Fara not reachable at http://127.0.0.1:8082\n"),
            "stdout should start with exact WARN line, got: {stdout}"
        );

        // File was written
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("fara_version"));
        assert!(content.contains("unreachable"));
    }
}
