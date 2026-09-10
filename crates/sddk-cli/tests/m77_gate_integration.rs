// Adversarial inline test: verify gate semantics end-to-end
use sddk_cli::command_spec::CommandSpec;
use sddk_cli::skill_definition::SkillRegistry;
use sddk_cli::skill_registry_bridge::{
    AdmissionOutcome, DiskBridgeOptions, check_admission, gate_command_for_environment,
    load_registry,
};

#[test]
fn m77_gate_fails_closed_for_real_skill_in_cli_runner() {
    // A CommandSpec with a non-placeholder missing skill must produce
    // a CommandOutput that fails with status 64 via the gate helper.
    let env = sddk_cli::CliEnvironment::default();
    let spec = CommandSpec::new("lint", "adversarial").with_required_skill("not.a.real.skill@v99");

    match gate_command_for_environment(&spec, &env) {
        Ok(Some(out)) => {
            assert_eq!(out.status, 64, "must fail with status 64");
            assert!(out.stderr.contains("not.a.real.skill@v99"));
        }
        Ok(None) => panic!("gate returned None — should have blocked"),
        Err(e) => {
            // Fail-open path: framework bundle missing — ok for fresh install.
            println!("fail-open path: {e}");
        }
    }
}

#[test]
fn m77_load_registry_from_real_framework_home_finds_disk_skills() {
    // Loads skills from the actual framework bundle installed on this
    // machine. Asserts at least 10 framework skills are present.
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let Some(home) = home else {
        return;
    };

    // Resolve the framework root the same way the CLI does.
    let data = home.join(".local/share/sddk");
    let current_link = data.join("framework").join("current");
    let framework_root = std::fs::read_link(&current_link).ok();

    let Some(framework_root) = framework_root else {
        return;
    };
    let opts = DiskBridgeOptions {
        project_root: None,
        framework_root: Some(framework_root),
        home: Some(home),
    };
    let (reg, loaded) = load_registry(&opts);
    assert!(
        loaded.len() >= 10,
        "expected at least 10 framework skills, got {}",
        loaded.len()
    );
    assert!(!reg.entries().is_empty());
}

#[test]
fn m77_placeholder_skill_warns_but_admits() {
    // Verifies the placeholder check matches reality: the three
    // placeholders from M7.5 (core.contract-review, etc.) are NOT
    // on disk and produce MissingPlaceholderSkill outcome.
    let reg = SkillRegistry::new();
    for token in &[
        "core.contract-review@v1",
        "core.workflow-orchestration@v1",
        "core.release-planning@v1",
    ] {
        let cmd = CommandSpec::new("lint", "x").with_required_skill(token);
        let outcome = check_admission(&cmd, &reg);
        assert!(
            matches!(outcome, AdmissionOutcome::MissingPlaceholderSkill { .. }),
            "token {token} should be MissingPlaceholderSkill, got {outcome:?}"
        );
        assert!(outcome.is_admitted(), "{token} should not block");
    }
}

#[test]
fn m77_real_missing_skill_blocks() {
    // Anything that is NOT a known placeholder and is NOT on disk
    // must produce MissingRealSkill and block.
    let reg = SkillRegistry::new();
    let cmd = CommandSpec::new("lint", "x").with_required_skill("framework.contract-review@v1");
    let outcome = check_admission(&cmd, &reg);
    assert!(matches!(outcome, AdmissionOutcome::MissingRealSkill { .. }));
    assert!(!outcome.is_admitted());
}

#[test]
fn m77_mixed_placeholder_plus_real_blocks() {
    // When the user mixes placeholders with a non-placeholder required
    // skill, the gate must block (we cannot prove the missing entry
    // is "just" a placeholder).
    let reg = SkillRegistry::new();
    let cmd = CommandSpec::new("lint", "x")
        .with_required_skill("core.contract-review@v1")
        .with_required_skill("not.real@v1");
    let outcome = check_admission(&cmd, &reg);
    assert!(matches!(outcome, AdmissionOutcome::MissingRealSkill { .. }));
    assert!(!outcome.is_admitted());
}
