//! Integration test: workflow manifest parsing and validation.
//!
//! This test reads the canonical workflow.yaml and verifies it deserializes
//! correctly, and that key references are valid.

#[test]
fn test_workflow_yaml_deserialization() {
    let yaml_content = include_str!("../../../workflow/workflow.yaml");

    // Parse the YAML
    let manifest: sddk_domain::workflow::WorkflowManifest = serde_saphyr::from_str(yaml_content)
        .expect("workflow.yaml must deserialize without errors");

    // Verify schema version
    assert_eq!(manifest.schema_version, 1);

    // Verify workflow metadata
    assert_eq!(manifest.workflow.id, "sddk-standard");
    assert_eq!(manifest.workflow.version, "0.1.0");

    // Verify statuses
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::Open)
    );
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::Blocked)
    );
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::Remediating)
    );
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::ReleasePending)
    );
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::Released)
    );
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::Closed)
    );
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::Abandoned)
    );
    assert!(
        manifest
            .statuses
            .contains(&sddk_domain::cycle::CycleStatus::Recovering)
    );

    // Verify phases
    assert!(
        manifest
            .phases
            .contains(&sddk_domain::cycle::Phase::Explore)
    );
    assert!(
        manifest
            .phases
            .contains(&sddk_domain::cycle::Phase::Specify)
    );
    assert!(manifest.phases.contains(&sddk_domain::cycle::Phase::Design));
    assert!(manifest.phases.contains(&sddk_domain::cycle::Phase::Plan));
    assert!(manifest.phases.contains(&sddk_domain::cycle::Phase::Build));
    assert!(manifest.phases.contains(&sddk_domain::cycle::Phase::Verify));
    assert!(
        manifest
            .phases
            .contains(&sddk_domain::cycle::Phase::Release)
    );
    assert!(
        manifest
            .phases
            .contains(&sddk_domain::cycle::Phase::Archive)
    );

    // Verify paths
    assert!(manifest.paths.contains_key("A-min"));
    assert!(manifest.paths.contains_key("A-lite"));
    assert!(manifest.paths.contains_key("A-full"));
    assert!(manifest.paths.contains_key("B-direct"));

    // Verify debt_verification in paths
    assert_eq!(
        manifest.paths.get("A-min").unwrap().debt_verification,
        "mandatory"
    );
    assert_eq!(
        manifest.paths.get("A-lite").unwrap().debt_verification,
        "mandatory"
    );
    assert_eq!(
        manifest.paths.get("A-full").unwrap().debt_verification,
        "mandatory"
    );
    assert_eq!(
        manifest.paths.get("B-direct").unwrap().debt_verification,
        "disabled"
    );
    assert!(
        manifest
            .paths
            .get("B-direct")
            .unwrap()
            .phases
            .contains(&"verify".to_owned())
    );

    // Verify key transitions exist
    let transition_ids: Vec<&str> = manifest.transitions.iter().map(|t| t.id.as_str()).collect();
    assert!(transition_ids.contains(&"cycle.start"));
    assert!(transition_ids.contains(&"phase.explore.complete"));
    assert!(transition_ids.contains(&"phase.specify.complete"));
    assert!(transition_ids.contains(&"phase.design.complete"));
    assert!(transition_ids.contains(&"phase.build.complete"));
    assert!(transition_ids.contains(&"phase.verify.complete"));
    assert!(transition_ids.contains(&"release.complete"));
    assert!(transition_ids.contains(&"archive.complete"));
    assert!(transition_ids.contains(&"cycle.start.b-direct"));
    assert!(transition_ids.contains(&"phase.specify.complete.a-min"));
    assert!(transition_ids.contains(&"phase.design.complete.a-lite"));
    assert!(transition_ids.contains(&"phase.build.complete.b-direct"));
    assert!(transition_ids.contains(&"phase.verify.complete.a-min"));
    assert!(transition_ids.contains(&"phase.verify.complete.a-lite"));
    assert!(transition_ids.contains(&"phase.verify.complete.b-direct"));

    // Verify phase.build.remediate transition exists (ADR-0077 / REQ-Build-Remediate-Transition)
    assert!(
        transition_ids.contains(&"phase.build.remediate"),
        "phase.build.remediate transition must exist per REQ-Build-Remediate-Transition"
    );

    let build_remediate = manifest
        .transitions
        .iter()
        .find(|transition| transition.id == "phase.build.remediate")
        .unwrap();
    // S4: Path coverage across all four paths
    assert_eq!(
        build_remediate.paths,
        ["A-min", "A-lite", "A-full", "B-direct"],
        "phase.build.remediate must cover all four delivery paths"
    );
    // Verify from.status == REMEDIATING
    assert_eq!(
        build_remediate.from.as_ref().unwrap().status,
        sddk_domain::cycle::CycleStatus::Remediating,
        "phase.build.remediate.from.status must be REMEDIATING"
    );
    // Verify from.phase == Build
    assert_eq!(
        build_remediate.from.as_ref().unwrap().phase,
        Some(sddk_domain::cycle::Phase::Build),
        "phase.build.remediate.from.phase must be Build"
    );
    // Verify to.status == OPEN
    assert_eq!(
        build_remediate.to.status,
        sddk_domain::cycle::CycleStatus::Open,
        "phase.build.remediate.to.status must be OPEN"
    );
    // Verify to.phase == Build (stays in build phase)
    assert_eq!(
        build_remediate.to.phase,
        Some(sddk_domain::cycle::Phase::Build),
        "phase.build.remediate.to.phase must be Build"
    );
    // Verify produces is empty
    assert!(
        build_remediate.produces.is_empty(),
        "phase.build.remediate.produces must be empty"
    );

    let b_direct_start = manifest
        .transitions
        .iter()
        .find(|transition| transition.id == "cycle.start.b-direct")
        .unwrap();
    assert_eq!(b_direct_start.paths, ["B-direct"]);
    assert_eq!(
        b_direct_start.to.phase,
        Some(sddk_domain::cycle::Phase::Build)
    );
    let b_direct_build = manifest
        .transitions
        .iter()
        .find(|transition| transition.id == "phase.build.complete.b-direct")
        .unwrap();
    assert_eq!(b_direct_build.to.status, sddk_domain::CycleStatus::Open);
    assert_eq!(
        b_direct_build.to.phase,
        Some(sddk_domain::cycle::Phase::Verify)
    );

    // Verify artifacts are defined
    assert!(manifest.artifacts.contains_key("exploration-report"));
    assert!(manifest.artifacts.contains_key("specification"));
    assert!(manifest.artifacts.contains_key("design"));
    assert!(manifest.artifacts.contains_key("verification-report"));
    for artifact in ["merge-receipt", "release-receipt", "archive-manifest"] {
        let definition = manifest.artifacts.get(artifact).unwrap();
        assert!(definition.terminal);
        assert!(definition.consumers.is_empty());
    }
    assert_eq!(
        manifest.artifacts.get("merge-receipt").unwrap().producer,
        "local-git"
    );
    assert_eq!(
        manifest.artifacts.get("release-receipt").unwrap().producer,
        "local-git"
    );
    assert!(
        manifest
            .gates
            .get("no-pending-effects")
            .unwrap()
            .description
            .as_deref()
            .unwrap()
            .contains("excluded")
    );

    // Verify gates are defined
    assert!(manifest.gates.contains_key("exploration-sufficient"));
    assert!(manifest.gates.contains_key("tests-pass"));
    assert!(manifest.gates.contains_key("policy-compliant"));

    // Verify forge configuration
    assert!(manifest.forge.is_some());
    let forge = manifest.forge.as_ref().unwrap();
    assert_eq!(forge.provider, "auto");
    assert!(forge.capabilities.is_some());
    let caps = forge.capabilities.as_ref().unwrap();
    assert!(caps.contains_key("git.create_branch"));
    assert!(caps.contains_key("pr.merge"));

    // Verify storage configuration
    assert!(manifest.storage.is_some());
    let storage = manifest.storage.as_ref().unwrap();
    assert!(storage.xdg);

    // Verify project identity
    assert!(manifest.project_identity.is_some());
    let identity = manifest.project_identity.as_ref().unwrap();
    assert_eq!(identity.scheme, "remote-url-hash");
    assert_eq!(identity.scope, "required");
    assert_eq!(identity.fallback.as_deref(), Some("receipt-uuid"));
}

#[test]
fn test_workflow_transition_lookup() {
    let yaml_content = include_str!("../../../workflow/workflow.yaml");

    let manifest: sddk_domain::workflow::WorkflowManifest =
        serde_saphyr::from_str(yaml_content).expect("workflow.yaml must deserialize");

    // Test finding transition by ID
    let explore_complete = manifest
        .transitions
        .iter()
        .find(|t| t.id == "phase.explore.complete");

    assert!(explore_complete.is_some());
    let transition = explore_complete.unwrap();

    // Verify source state
    assert!(transition.from.is_some());
    let from = transition.from.as_ref().unwrap();
    assert_eq!(from.status, sddk_domain::cycle::CycleStatus::Open);
    assert_eq!(from.phase, Some(sddk_domain::cycle::Phase::Explore));

    // Verify target state
    assert_eq!(transition.to.status, sddk_domain::cycle::CycleStatus::Open);
    assert_eq!(
        transition.to.phase,
        Some(sddk_domain::cycle::Phase::Specify)
    );

    // Verify requirements
    assert!(!transition.requires.is_empty());
}

#[test]
fn test_cycle_block_transition_no_phase() {
    let yaml_content = include_str!("../../../workflow/workflow.yaml");

    let manifest: sddk_domain::workflow::WorkflowManifest =
        serde_saphyr::from_str(yaml_content).expect("workflow.yaml must deserialize");

    // Find the cycle.block transition
    let block_transition = manifest.transitions.iter().find(|t| t.id == "cycle.block");

    assert!(block_transition.is_some());
    let transition = block_transition.unwrap();

    // Verify source state has no phase (block/unblock don't change phase)
    assert!(transition.from.as_ref().unwrap().phase.is_none());
    assert_eq!(
        transition.from.as_ref().unwrap().status,
        sddk_domain::cycle::CycleStatus::Open
    );

    // Verify target state has no phase
    assert!(transition.to.phase.is_none());
    assert_eq!(
        transition.to.status,
        sddk_domain::cycle::CycleStatus::Blocked
    );
}

#[test]
fn test_workflow_policies() {
    let yaml_content = include_str!("../../../workflow/workflow.yaml");

    let manifest: sddk_domain::workflow::WorkflowManifest =
        serde_saphyr::from_str(yaml_content).expect("workflow.yaml must deserialize");

    // Verify policies
    assert!(manifest.policies.active_cycles_per_project.is_some());
    assert_eq!(manifest.policies.active_cycles_per_project.unwrap(), 1);

    assert!(manifest.policies.require_clean_worktree_on_start.is_some());
    assert!(manifest.policies.require_clean_worktree_on_start.unwrap());

    assert!(manifest.policies.debt_verification.is_some());
    let debt_ver = manifest.policies.debt_verification.as_ref().unwrap();
    assert_eq!(debt_ver.get("A-min").unwrap(), "mandatory");
    assert_eq!(debt_ver.get("B-direct").unwrap(), "disabled");
}

#[test]
fn test_valid_transition_with_workflow_yaml() {
    let yaml_content = include_str!("../../../workflow/workflow.yaml");

    let manifest: sddk_domain::workflow::WorkflowManifest =
        serde_saphyr::from_str(yaml_content).expect("workflow.yaml must deserialize");

    // Test valid transition with all requirements met
    let result = sddk_domain::workflow::valid_transition(
        &manifest,
        "phase.explore.complete",
        &["exploration-report".to_string()],
        &["exploration-sufficient".to_string()],
    );
    assert!(result.is_ok());

    // Test transition missing gate
    let result = sddk_domain::workflow::valid_transition(
        &manifest,
        "phase.explore.complete",
        &["exploration-report".to_string()],
        &[], // Missing gate
    );
    assert!(result.is_err());

    // Test transition not found
    let result =
        sddk_domain::workflow::valid_transition(&manifest, "nonexistent.transition", &[], &[]);
    assert!(result.is_err());
}
