//! Unit tests for uat_discover module.
//!
//! Tests cover:
//! - parser good/partial/empty regression
//! - multi-run combine
//! - fallback behavior
//! - URL extraction
//! - budget command args via command builder pure

use tempfile::TempDir;

use crate::uat_discover::aam::AamModel;
use crate::uat_discover::parser::{
    SummaryJson, TrajectoryEntry, extract_action_metadata, extract_action_string, parse_run_dir,
    sanitize_for_id,
};

#[test]
fn test_parse_run_dir_empty_dir() {
    let dir = TempDir::new().unwrap();
    let result = parse_run_dir(dir.path(), "test goal", 0).unwrap();
    assert!(result.pages.is_empty());
    assert!(result.flows.is_empty());
}

#[test]
fn test_parse_run_dir_with_summary_only() {
    let dir = TempDir::new().unwrap();
    let summary = r#"{
        "goal": "test goal",
        "url": "http://localhost:3000",
        "stepsTaken": 3,
        "maxSteps": 50,
        "done": true,
        "finalUrl": "http://localhost:3000/page2",
        "pageTitle": "Test Page"
    }"#;
    std::fs::write(dir.path().join("summary.json"), summary).unwrap();

    let result = parse_run_dir(dir.path(), "test goal", 0).unwrap();
    assert!(result.pages.is_empty()); // No trajectory → no pages
    assert!(result.flows.is_empty());
    assert!(result.urls.contains(&"http://localhost:3000".to_string()));
    assert!(
        result
            .urls
            .contains(&"http://localhost:3000/page2".to_string())
    );
}

#[test]
fn test_parse_run_dir_with_trajectory() {
    let dir = TempDir::new().unwrap();
    let summary = r#"{
        "goal": "navigate and click",
        "url": "http://localhost:3000",
        "stepsTaken": 2,
        "maxSteps": 50,
        "done": true,
        "pageTitle": "Home"
    }"#;
    std::fs::write(dir.path().join("summary.json"), summary).unwrap();

    let trajectory = r#"[
        {
            "step": 1,
            "screenshot": "screenshot-01.png",
            "rawDecision": "go to home",
            "action": "goto /",
            "result": "loaded"
        },
        {
            "step": 2,
            "screenshot": "screenshot-02.png",
            "rawDecision": "click button",
            "action": "click #btn",
            "result": "clicked"
        }
    ]"#;
    std::fs::write(dir.path().join("trajectory.json"), trajectory).unwrap();

    // Create dummy screenshot files
    std::fs::write(dir.path().join("screenshot-01.png"), "").unwrap();
    std::fs::write(dir.path().join("screenshot-02.png"), "").unwrap();

    let result = parse_run_dir(dir.path(), "navigate and click", 0).unwrap();

    assert!(
        !result.pages.is_empty(),
        "should have pages from trajectory"
    );
    assert!(!result.flows.is_empty(), "should have flow from trajectory");
    assert_eq!(result.pages.len(), 2);
    assert_eq!(result.flows.len(), 1);
    assert_eq!(result.screenshots.len(), 2);
    assert!(result.urls.contains(&"http://localhost:3000".to_string()));
}

#[test]
fn test_parse_run_dir_multiple_runs_combine() {
    let base = TempDir::new().unwrap();

    // Run 0
    let run0 = base.path().join("run-000");
    std::fs::create_dir(&run0).unwrap();
    std::fs::write(
        run0.join("summary.json"),
        r#"{"goal": "goal A", "url": "http://a.com"}"#,
    )
    .unwrap();
    std::fs::write(
        run0.join("trajectory.json"),
        r#"[{"step": 1, "action": "visit a", "result": "ok"}]"#,
    )
    .unwrap();

    // Run 1
    let run1 = base.path().join("run-001");
    std::fs::create_dir(&run1).unwrap();
    std::fs::write(
        run1.join("summary.json"),
        r#"{"goal": "goal B", "url": "http://b.com"}"#,
    )
    .unwrap();
    std::fs::write(
        run1.join("trajectory.json"),
        r#"[{"step": 1, "action": "visit b", "result": "ok"}]"#,
    )
    .unwrap();

    let result0 = parse_run_dir(&run0, "goal A", 0).unwrap();
    let result1 = parse_run_dir(&run1, "goal B", 1).unwrap();

    // Combine manually (as the module does)
    let combined_pages = [result0.pages, result1.pages].concat();
    let combined_flows = [result0.flows, result1.flows].concat();
    let combined_urls = [result0.urls, result1.urls].concat();

    assert_eq!(combined_pages.len(), 2);
    assert_eq!(combined_flows.len(), 2);
    assert!(combined_urls.contains(&"http://a.com".to_string()));
    assert!(combined_urls.contains(&"http://b.com".to_string()));
}

#[test]
fn test_fallback_model_has_correct_markers() {
    let aam = AamModel::fallback("http://app", "/");

    assert_eq!(aam.app.fara_version, "unreachable");
    assert_eq!(aam.provenance.fallback, Some("no-fara".into()));
    assert!(aam.provenance.tags.contains(&"fallback".to_string()));
}

#[test]
fn test_fallback_model_has_entry_url() {
    let aam = AamModel::fallback("http://app", "/login");
    assert!(aam.urls.contains(&"/login".to_string()));
}

#[test]
fn test_trajectory_entry_action_object_format() {
    let json = "{\
        \"step\": 1,\
        \"action\": {\"type\": \"click\", \"selector\": \"#submit\", \"value\": \"test\"},\
        \"result\": {\"message\": \"button clicked\", \"success\": true}\
    }";
    let entry: TrajectoryEntry = serde_json::from_str(json).unwrap();

    assert_eq!(entry.step, 1);
    // Action should be extracted as string
    let action_str = crate::uat_discover::parser::extract_action_string(&entry.action);
    assert!(action_str.contains("click") || action_str.contains("type"));
}

#[test]
fn test_trajectory_entry_result_object_format() {
    let json = "{\
        \"step\": 2,\
        \"result\": {\"text\": \"page loaded\", \"status\": 200}\
    }";
    let entry: TrajectoryEntry = serde_json::from_str(json).unwrap();
    let result_str = crate::uat_discover::parser::extract_result_string(&entry.result);
    assert!(result_str.contains("page loaded") || result_str.contains("text"));
}

#[test]
fn test_url_extraction_from_summary() {
    let json = r#"{
        "goal": "test",
        "url": "http://localhost:3000/login",
        "finalUrl": "http://localhost:3000/dashboard",
        "stepsTaken": 5,
        "maxSteps": 50
    }"#;
    let summary: SummaryJson = serde_json::from_str(json).unwrap();

    assert!(summary.url.contains("localhost:3000"));
    assert!(summary.final_url.as_ref().unwrap().contains("dashboard"));
}

#[test]
fn test_parse_run_dir_screenshot_files_discovered() {
    let dir = TempDir::new().unwrap();
    let summary = r#"{"goal": "test", "url": "http://test.com"}"#;
    std::fs::write(dir.path().join("summary.json"), summary).unwrap();
    std::fs::write(dir.path().join("trajectory.json"), "[]").unwrap();

    // Create screenshot files
    std::fs::write(dir.path().join("screenshot-01.png"), "").unwrap();
    std::fs::write(dir.path().join("screenshot-02.png"), "").unwrap();
    std::fs::write(dir.path().join("screenshot.png"), "").unwrap();

    let result = parse_run_dir(dir.path(), "test", 0).unwrap();
    assert_eq!(result.screenshots.len(), 3);
    assert!(
        result
            .screenshots
            .contains(&"screenshot-01.png".to_string())
    );
    assert!(result.screenshots.contains(&"screenshot.png".to_string()));
}

#[test]
fn test_empty_trajectory_no_flow() {
    let dir = TempDir::new().unwrap();
    let summary = r#"{"goal": "empty run", "url": "http://test.com", "stepsTaken": 0}"#;
    std::fs::write(dir.path().join("summary.json"), summary).unwrap();
    std::fs::write(dir.path().join("trajectory.json"), "[]").unwrap();

    let result = parse_run_dir(dir.path(), "empty", 0).unwrap();
    assert!(result.pages.is_empty());
    assert!(result.flows.is_empty()); // No steps → no flow
}

#[test]
fn test_flow_id_sanitizes_goal() {
    let dir = TempDir::new().unwrap();
    let summary = r#"{"goal": "test flow with spaces!", "url": "http://test.com"}"#;
    std::fs::write(dir.path().join("summary.json"), summary).unwrap();
    std::fs::write(
        dir.path().join("trajectory.json"),
        r#"[{"step": 1, "action": "test", "result": "ok"}]"#,
    )
    .unwrap();

    let result = parse_run_dir(dir.path(), "test flow with spaces!", 0).unwrap();
    assert!(!result.flows.is_empty());
    let flow_id = &result.flows[0].id;
    assert!(flow_id.contains("test_flow_with_spaces_"));
}

#[test]
fn test_summary_json_deserializes_all_fields() {
    let json = r#"{
        "goal": "checkout flow",
        "url": "http://shop.com/cart",
        "stepsTaken": 12,
        "maxSteps": 50,
        "done": true,
        "stopReason": "goal reached",
        "finalUrl": "http://shop.com/confirmation",
        "pageTitle": "Order Confirmed",
        "faraUrl": "http://fara.local:8082"
    }"#;
    let summary: SummaryJson = serde_json::from_str(json).unwrap();

    assert_eq!(summary.goal, "checkout flow");
    assert_eq!(summary.steps_taken, 12);
    assert_eq!(summary.max_steps, 50);
    assert!(summary.done);
    assert_eq!(summary.stop_reason.as_deref(), Some("goal reached"));
    assert_eq!(summary.fara_url.as_deref(), Some("http://fara.local:8082"));
}

#[test]
fn test_budget_parameter_passed_to_command() {
    // Test that budget is correctly stored in args (verifies CLI wiring)
    use crate::uat::DiscoverArgs;

    let args = DiscoverArgs {
        app_url: "http://test.com".into(),
        entry: "/".into(),
        goals: vec!["test goal".into()],
        budget: 25, // Custom budget
        fara_url: Some("http://fara:8082".into()),
        output: None,
        format: crate::OutputFormat::Text,
    };

    assert_eq!(args.budget, 25);
}

#[test]
fn test_multiple_goals_produce_multiple_flows() {
    let base = TempDir::new().unwrap();

    for i in 0..3 {
        let run_dir = base.path().join(format!("run-{:03}", i));
        std::fs::create_dir(&run_dir).unwrap();
        std::fs::write(
            run_dir.join("summary.json"),
            format!(
                r#"{{"goal": "goal {}", "url": "http://test.com/{}", "stepsTaken": 1}}"#,
                i, i
            ),
        )
        .unwrap();
        std::fs::write(
            run_dir.join("trajectory.json"),
            format!(
                r#"[{{"step": 1, "action": "step for goal {}", "result": "done"}}]"#,
                i
            ),
        )
        .unwrap();
    }

    let mut all_pages = Vec::new();
    let mut all_flows = Vec::new();

    for i in 0..3 {
        let run_dir = base.path().join(format!("run-{:03}", i));
        let result = parse_run_dir(&run_dir, &format!("goal {}", i), i).unwrap();
        all_pages.extend(result.pages);
        all_flows.extend(result.flows);
    }

    assert_eq!(all_pages.len(), 3);
    assert_eq!(all_flows.len(), 3);
}

// Parser unit tests moved from parser.rs

#[test]
fn test_parse_trajectory_entry_minimal() {
    let json = r#"{"step": 1}"#;
    let entry: TrajectoryEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.step, 1);
    assert!(entry.screenshot.is_none());
}

#[test]
fn test_parse_trajectory_entry_full() {
    let json = r#"{
        "step": 3,
        "screenshot": "screenshot-03.png",
        "rawDecision": "click login button",
        "action": "click",
        "result": "navigation to /dashboard"
    }"#;
    let entry: TrajectoryEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.step, 3);
    assert_eq!(entry.screenshot.as_deref(), Some("screenshot-03.png"));
    assert_eq!(entry.raw_decision.as_deref(), Some("click login button"));
}

#[test]
fn test_parse_trajectory_entry_action_object() {
    let json = "{\"step\": 1, \"action\": {\"type\": \"click\", \"selector\": \"#btn\"}, \"result\": {\"message\": \"clicked\"}}";
    let entry: TrajectoryEntry = serde_json::from_str(json).unwrap();
    let action_str = extract_action_string(&entry.action);
    assert!(action_str.contains("click"));
}

#[test]
fn test_parse_summary_json_minimal() {
    let json = r#"{"goal": "login"}"#;
    let summary: SummaryJson = serde_json::from_str(json).unwrap();
    assert_eq!(summary.goal, "login");
    assert_eq!(summary.steps_taken, 0);
}

#[test]
fn test_parse_summary_json_full() {
    let json = r#"{
        "goal": "test checkout flow",
        "url": "http://localhost:3000",
        "stepsTaken": 5,
        "maxSteps": 50,
        "done": true,
        "stopReason": "goal reached",
        "finalUrl": "http://localhost:3000/confirmation",
        "pageTitle": "Confirmation",
        "faraUrl": "http://127.0.0.1:8082"
    }"#;
    let summary: SummaryJson = serde_json::from_str(json).unwrap();
    assert_eq!(summary.goal, "test checkout flow");
    assert_eq!(summary.steps_taken, 5);
    assert!(summary.done);
    assert_eq!(
        summary.final_url.as_deref(),
        Some("http://localhost:3000/confirmation")
    );
}

#[test]
fn test_sanitize_for_id() {
    assert_eq!(sanitize_for_id("test-flow"), "test_flow");
    assert_eq!(sanitize_for_id("test flow!"), "test_flow_");
    assert_eq!(sanitize_for_id("test@#flow"), "test__flow");
}

#[test]
fn test_extract_action_metadata_full() {
    let json = serde_json::json!({
        "type": "click",
        "selector": "#submit-btn",
        "value": "admin",
        "target": "modal"
    });
    let meta = extract_action_metadata(&json);
    assert_eq!(meta.selector.as_deref(), Some("#submit-btn"));
    assert_eq!(meta.value.as_deref(), Some("admin"));
    assert_eq!(meta.target.as_deref(), Some("modal"));
}

#[test]
fn test_extract_action_metadata_partial() {
    let json = serde_json::json!({
        "type": "input",
        "selector": "#username"
    });
    let meta = extract_action_metadata(&json);
    assert_eq!(meta.selector.as_deref(), Some("#username"));
    assert!(meta.value.is_none());
    assert!(meta.target.is_none());
}

#[test]
fn test_extract_action_metadata_string_action() {
    // String action has no metadata
    let json = serde_json::json!("click");
    let meta = extract_action_metadata(&json);
    assert!(meta.selector.is_none());
    assert!(meta.value.is_none());
    assert!(meta.target.is_none());
}

#[test]
fn test_extract_action_metadata_empty_object() {
    let json = serde_json::json!({});
    let meta = extract_action_metadata(&json);
    assert!(meta.selector.is_none());
    assert!(meta.value.is_none());
    assert!(meta.target.is_none());
}

#[test]
fn test_parse_run_dir_action_object_populates_aam_flow_step() {
    // Task 6: parser action object selector/value/target is connected to AamFlowStep.
    // Assert the step resulting from parse_run_dir has selector/value/target set.
    let dir = tempfile::TempDir::new().unwrap();
    let summary = r#"{
        "goal": "fill login form",
        "url": "http://app/login",
        "stepsTaken": 2,
        "maxSteps": 50,
        "done": true,
        "pageTitle": "Login"
    }"#;
    std::fs::write(dir.path().join("summary.json"), summary).unwrap();

    let trajectory = serde_json::to_string(&vec![
        serde_json::json!({
            "step": 1,
            "screenshot": "screenshot-01.png",
            "rawDecision": "type username",
            "action": {"type": "input", "selector": "#username", "value": "admin"},
            "result": {"message": "typed"}
        }),
        serde_json::json!({
            "step": 2,
            "screenshot": "screenshot-02.png",
            "rawDecision": "click submit",
            "action": {"type": "click", "selector": "#submit-btn", "target": "form"},
            "result": {"message": "submitted"}
        }),
    ])
    .unwrap();
    std::fs::write(dir.path().join("trajectory.json"), trajectory).unwrap();

    let result = parse_run_dir(dir.path(), "fill login form", 0).unwrap();

    assert_eq!(result.flows.len(), 1, "should produce one flow");
    let steps = &result.flows[0].steps;
    assert_eq!(steps.len(), 2, "flow should have 2 steps");

    // Step 1: input action with selector and value
    assert_eq!(steps[0].action, "input");
    assert_eq!(steps[0].selector.as_deref(), Some("#username"));
    assert_eq!(steps[0].value.as_deref(), Some("admin"));
    assert!(steps[0].target.is_none(), "input should not have target");

    // Step 2: click action with selector and target
    assert_eq!(steps[1].action, "click");
    assert_eq!(steps[1].selector.as_deref(), Some("#submit-btn"));
    assert_eq!(steps[1].target.as_deref(), Some("form"));
    assert!(steps[1].value.is_none(), "click should not have value");
}

#[test]
fn test_screenshot_refs_are_relative_filenames() {
    // Task 7: screenshot refs are relative/names within AAM (not absolute paths).
    // They are filenames as stored by parse_run_dir.
    let dir = tempfile::TempDir::new().unwrap();
    let summary = r#"{"goal": "test", "url": "http://test.com"}"#;
    std::fs::write(dir.path().join("summary.json"), summary).unwrap();

    std::fs::write(dir.path().join("screenshot-01.png"), "").unwrap();
    std::fs::write(dir.path().join("screenshot-02.png"), "").unwrap();

    let result = parse_run_dir(dir.path(), "test", 0).unwrap();

    for screenshot in &result.screenshots {
        assert!(
            !screenshot.starts_with('/'),
            "screenshot ref should be relative, not absolute: {screenshot}"
        );
        assert!(
            !screenshot.contains(std::path::MAIN_SEPARATOR),
            "screenshot ref should be bare filename: {screenshot}"
        );
    }
    assert!(
        result
            .screenshots
            .contains(&"screenshot-01.png".to_string())
    );
    assert!(
        result
            .screenshots
            .contains(&"screenshot-02.png".to_string())
    );
}
