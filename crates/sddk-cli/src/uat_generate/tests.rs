//! Tests for uat_generate module.
//!
//! Integration tests and unit tests for the generate pipeline.

/// RED test: build_plan requires non-empty inputs.
/// If no features can be extracted, returns NoFeaturesExtracted error.
/// This ensures atomicity: no partial output is ever written.
#[test]
fn build_plan_requires_non_empty() {
    let result = crate::uat_generate::planner::build_plan("v1.0.0", &None, &None, &None, &[]);
    assert!(matches!(
        result,
        Err(crate::uat_generate::planner::PlanError::NoFeaturesExtracted)
    ));
}

/// Test that build_plan produces features from requirements markdown.
#[test]
fn build_plan_from_requirements() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login Feature\n- User can login with email\n- User can reset password\n\n## API Feature\n- API returns JSON\n",
    )
    .unwrap();

    let result = crate::uat_generate::planner::build_plan(
        "v1.0.0",
        &Some(req_dir.to_path_buf()),
        &None,
        &None,
        &[],
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.plan.features.is_empty());
}

/// Test that build_plan produces features from changelog Added/Changed.
#[test]
fn build_plan_from_changelog() {
    let td = tempfile::TempDir::new().unwrap();
    let changelog = td.path().join("CHANGELOG.md");
    std::fs::write(
        &changelog,
        "## Added\n- New login feature\n- API endpoint\n\n## Changed\n- Performance improvements\n",
    )
    .unwrap();

    let result =
        crate::uat_generate::planner::build_plan("v1.0.0", &None, &Some(changelog), &None, &[]);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.plan.features.is_empty());
}

/// Test that build_plan produces scenarios from AAM discovery candidates.
#[test]
fn build_plan_from_aam_candidates() {
    use crate::uat_discover::AamScenarioCandidate;

    let candidate = AamScenarioCandidate {
        flow_ref: Some("flow-1".to_string()),
        title: "User Login Flow".to_string(),
        priority: Some("P1".to_string()),
        plain_steps: vec![
            "Navigate to /login".to_string(),
            "Enter credentials".to_string(),
        ],
        estimated_duration_minutes: Some(10),
        evidence: crate::uat_discover::aam::AamEvidence {
            kinds: vec!["screenshot".to_string()],
        },
        provenance: crate::uat_discover::aam::AamProvenance {
            generated_by: Some("fara".to_string()),
            author: None,
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            last_modified_at: None,
            origin: Some("discovered".to_string()),
            origin_ref: None,
            modified_by: None,
            linked_defect: None,
            repro_command: None,
            tags: vec![],
            confidence: Some(0.8),
            human_reviewed: false,
            fallback: None,
        },
    };

    let result =
        crate::uat_generate::planner::build_plan("v1.0.0", &None, &None, &None, &[candidate]);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.plan.features.is_empty());
    let scenarios: usize = output.plan.features.iter().map(|f| f.scenarios.len()).sum();
    assert_eq!(scenarios, 1);
}

/// Test that build_plan atomicity: error returns no partial output.
/// If build fails, NO file should be written (this tests the planner itself
/// returns error, not that files aren't written - that tested elsewhere).
#[test]
fn build_plan_atomic_no_partial() {
    // If build fails, no plan is returned (atomic rule: no partial output)
    let result = crate::uat_generate::planner::build_plan("v1.0.0", &None, &None, &None, &[]);
    assert!(result.is_err());

    // No output path is passed, so no file should exist
    // This test verifies the planner itself doesn't write files
}

/// RED test: build_plan with only last_plan preserves scenarios deeply.
/// Creates a valid previous plan with 2 features/scenarios, then runs
/// build_plan with ONLY last_plan (no requirements/changelog/AAM).
/// The output must preserve both scenarios with content/IDs/provenance
/// and set release.last_uat_release correctly.
#[test]
fn build_plan_from_last_plan_preserves_scenarios() {
    let td = tempfile::TempDir::new().unwrap();
    let last_plan_path = td.path().join("uat-plan-v1.0.0.yaml");

    // Create a valid previous plan with 2 features (1 scenario each)
    let prev_plan = sddk_domain::UatPlan {
        schema_version: sddk_domain::LATEST_PLAN_SCHEMA_VERSION,
        release: sddk_domain::UatPlanRelease {
            candidate: "v1.0.0".to_string(),
            project: None,
            last_uat_release: Some("v0.9.0".to_string()),
        },
        generated_by: "uat-planner".to_string(),
        generated_at: "2024-01-01T00:00:00Z".to_string(),
        features: vec![
            sddk_domain::UatFeature {
                id: "F-01".to_string(),
                name: "Login Feature".to_string(),
                requirement_ref: Some("REQ-001".to_string()),
                design_ref: None,
                priority: sddk_domain::UatPriority::P1,
                scenarios: vec![sddk_domain::UatScenario {
                    id: "S-001".to_string(),
                    title: "User can login".to_string(),
                    priority: sddk_domain::UatPriority::P1,
                    assignee: sddk_domain::UatAssignee::Developer,
                    preconditions: vec!["User registered".to_string()],
                    plain_steps: vec![sddk_domain::UatStep {
                        action: "Navigate to /login".to_string(),
                        copy_hint: false,
                        expected: "Login form visible".to_string(),
                        step: None,
                        kind: None,
                        vs_expected_check: None,
                    }],
                    technical_steps: vec![],
                    rationale: Some("Core login flow".to_string()),
                    evidence_prompt: None,
                    flags: vec![],
                    est_minutes: 5,
                    context: None,
                    evidence: None,
                    risk: None,
                    automation: None,
                    provenance: Some(sddk_domain::UatProvenance {
                        author: "test".to_string(),
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        last_modified_at: "2024-01-01T00:00:00Z".to_string(),
                        origin: sddk_domain::UatOrigin::Spec,
                        origin_ref: Some("REQ-001".to_string()),
                    }),
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
                }],
            },
            sddk_domain::UatFeature {
                id: "F-02".to_string(),
                name: "API Feature".to_string(),
                requirement_ref: Some("REQ-002".to_string()),
                design_ref: None,
                priority: sddk_domain::UatPriority::P2,
                scenarios: vec![sddk_domain::UatScenario {
                    id: "S-002".to_string(),
                    title: "API returns JSON".to_string(),
                    priority: sddk_domain::UatPriority::P2,
                    assignee: sddk_domain::UatAssignee::Developer,
                    preconditions: vec![],
                    plain_steps: vec![sddk_domain::UatStep {
                        action: "GET /api/status".to_string(),
                        copy_hint: false,
                        expected: "JSON response".to_string(),
                        step: None,
                        kind: None,
                        vs_expected_check: None,
                    }],
                    technical_steps: vec![],
                    rationale: Some("API verification".to_string()),
                    evidence_prompt: None,
                    flags: vec![],
                    est_minutes: 3,
                    context: None,
                    evidence: None,
                    risk: None,
                    automation: None,
                    provenance: Some(sddk_domain::UatProvenance {
                        author: "test".to_string(),
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        last_modified_at: "2024-01-01T00:00:00Z".to_string(),
                        origin: sddk_domain::UatOrigin::Regression,
                        origin_ref: Some("REQ-002".to_string()),
                    }),
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
                }],
            },
        ],
        runner_mode: None,
        approval: Some(sddk_domain::UatPlanApproval {
            id: "T-previous".to_string(),
            display: "Previous Approver".to_string(),
            approved_at: "2024-01-02T00:00:00Z".to_string(),
        }),
    };

    // Write the previous plan
    let yaml = serde_saphyr::to_string(&prev_plan).unwrap();
    std::fs::write(&last_plan_path, &yaml).unwrap();

    // Run build_plan with ONLY last_plan (no requirements, changelog, or AAM)
    let result = crate::uat_generate::planner::build_plan(
        "v2.0.0",              // new release
        &None,                 // no requirements
        &None,                 // no changelog
        &Some(last_plan_path), // ONLY last_plan
        &[],                   // no AAM candidates
    );

    assert!(
        result.is_ok(),
        "build_plan should succeed with last_plan only: {:?}",
        result
    );
    let output = result.unwrap();

    // Verify 2 features and 2 scenarios preserved
    assert_eq!(output.plan.features.len(), 2, "Should have 2 features");
    let total_scenarios: usize = output.plan.features.iter().map(|f| f.scenarios.len()).sum();
    assert_eq!(total_scenarios, 2, "Should have 2 scenarios");

    // Verify scenario IDs are preserved
    let s1 = output
        .plan
        .features
        .iter()
        .flat_map(|f| f.scenarios.iter())
        .find(|s| s.id == "S-001");
    assert!(s1.is_some(), "Scenario S-001 should be preserved");
    let s1 = s1.unwrap();
    assert_eq!(s1.title, "User can login");
    assert!(s1.provenance.is_some());
    assert_eq!(
        s1.provenance.as_ref().unwrap().origin,
        sddk_domain::UatOrigin::Spec
    );

    let s2 = output
        .plan
        .features
        .iter()
        .flat_map(|f| f.scenarios.iter())
        .find(|s| s.id == "S-002");
    assert!(s2.is_some(), "Scenario S-002 should be preserved");
    let s2 = s2.unwrap();
    assert_eq!(s2.title, "API returns JSON");
    assert!(s2.provenance.is_some());
    assert_eq!(
        s2.provenance.as_ref().unwrap().origin,
        sddk_domain::UatOrigin::Regression
    );

    // Verify release.last_uat_release is set to previous release
    assert_eq!(
        output.plan.release.last_uat_release,
        Some("v1.0.0".to_string()),
        "last_uat_release should be set to v1.0.0"
    );
    assert_eq!(output.plan.release.candidate, "v2.0.0");

    // Verify previous approval is NOT copied (new plan starts fresh)
    assert!(
        output.plan.approval.is_none(),
        "Previous approval should NOT be copied to new plan"
    );
}

/// Test that invalid last-plan content (parse error) returns Err PlanError,
/// not a warning "starting fresh" — explicit invalid input must be Err.
#[test]
fn build_plan_invalid_last_plan_returns_error() {
    let td = tempfile::TempDir::new().unwrap();
    let invalid_plan_path = td.path().join("invalid-plan.yaml");

    // Write invalid YAML
    std::fs::write(&invalid_plan_path, "not: [valid: yaml: at: all").unwrap();

    // Also provide requirements so we don't get RequirementsRequired
    let req_dir = td.path().join("req");
    std::fs::create_dir(&req_dir).unwrap();
    std::fs::write(req_dir.join("req.md"), "# Req\n- Feature").unwrap();

    let result = crate::uat_generate::planner::build_plan(
        "v2.0.0",
        &Some(req_dir),
        &None,
        &Some(invalid_plan_path),
        &[],
    );

    // Should error on parse, not silently "starting fresh"
    assert!(
        result.is_err(),
        "Invalid last_plan should return Err, not Ok with warning"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            crate::uat_generate::planner::PlanError::LastPlanParseFailed(_)
        ),
        "Should be LastPlanParseFailed, got: {:?}",
        err
    );
}
