//! Tests for `dev models` — temp bundle fixtures only (M1–M5).

use super::{ModelsArgs, ModelsCommand, ModelsListArgs, ModelsSetArgs, ModelsValidateArgs};
use crate::{CliEnvironment, OutputFormat};

fn config_yaml() -> &'static str {
    "tiers:\n  premium:\n    opencode: deepseek/deepseek-chat\n    zcode: deepseek/deepseek-chat\n    claude: sonnet\n    codex: openai/gpt-5.4\n  fast:\n    opencode: zai-coding-plan/glm-5-turbo\n    zcode: zai-coding-plan/glm-5-turbo\n    claude: haiku\n    codex: openai/gpt-5.4-fast\nagents:\n  orchestrator:\n    tier: premium\n  sddk-explore:\n    tier: fast\n"
}

/// Synthetic framework bundle at `<tmp>/data/framework/v0.0.0` with
/// two agents and a config file; environment resolves the same bundle.
fn bundle_fixture(config: &str) -> (tempfile::TempDir, CliEnvironment) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("data/framework/v0.0.0");
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    for (name, desc) in [
        ("orchestrator", "coordinator"),
        ("sddk-explore", "explorer"),
    ] {
        std::fs::write(
            root.join(format!("agents/{name}.md")),
            format!("---\nname: {name}\ndescription: {desc}\n---\n# Body\n"),
        )
        .unwrap();
    }
    std::fs::write(root.join("assets/agent-models.yaml"), config).unwrap();
    let environment = CliEnvironment {
        sddk_data_dir: Some(tmp.path().join("data")),
        ..CliEnvironment::default()
    };
    (tmp, environment)
}

fn run(args: ModelsArgs, environment: &CliEnvironment) -> crate::CommandOutput {
    super::run_dev_models(args, environment)
}

// M1 — validate exit codes: 0 valid / 2 invalid / 3 unresolvable bundle
#[test]
fn validate_ok_exit_zero_and_invalid_exit_two() {
    let (_tmp, environment) = bundle_fixture(config_yaml());
    let ok = run(
        ModelsArgs {
            command: ModelsCommand::Validate(ModelsValidateArgs { file: None }),
        },
        &environment,
    );
    assert_eq!(ok.status, 0, "stderr: {}", ok.stderr);

    // Point at an invalid file directly.
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.yaml");
    std::fs::write(&bad, "agents:\n  a:\n    tier: turbo\n").unwrap();
    let invalid = run(
        ModelsArgs {
            command: ModelsCommand::Validate(ModelsValidateArgs {
                file: Some(bad.clone()),
            }),
        },
        &environment,
    );
    assert_eq!(invalid.status, 2);
    assert!(invalid.stderr.contains("turbo"), "{}", invalid.stderr);

    // No bundle at all → exit 3.
    let empty_env = CliEnvironment {
        sddk_data_dir: Some(dir.path().join("nothing")),
        ..CliEnvironment::default()
    };
    let unresolvable = run(
        ModelsArgs {
            command: ModelsCommand::Validate(ModelsValidateArgs { file: None }),
        },
        &empty_env,
    );
    assert_eq!(unresolvable.status, 3);
}

// M2 — set tier + override, then re-list and assert both persisted
#[test]
fn set_tier_and_override_roundtrip() {
    let (_tmp, environment) = bundle_fixture(config_yaml());
    let set_tier = run(
        ModelsArgs {
            command: ModelsCommand::Set(ModelsSetArgs {
                file: None,
                agent: "sddk-explore".to_owned(),
                tier: Some(super::ModelTier::Premium),
                r#override: Vec::new(),
                clear_override: Vec::new(),
            }),
        },
        &environment,
    );
    assert_eq!(set_tier.status, 0, "stderr: {}", set_tier.stderr);
    let set_override = run(
        ModelsArgs {
            command: ModelsCommand::Set(ModelsSetArgs {
                file: None,
                agent: "sddk-explore".to_owned(),
                tier: None,
                r#override: vec!["opencode=deepseek/deepseek-reasoner".to_owned()],
                clear_override: Vec::new(),
            }),
        },
        &environment,
    );
    assert_eq!(set_override.status, 0, "stderr: {}", set_override.stderr);

    let list = run(
        ModelsArgs {
            command: ModelsCommand::List(ModelsListArgs {
                file: None,
                format: OutputFormat::Json,
            }),
        },
        &environment,
    );
    assert_eq!(list.status, 0);
    let doc: serde_json::Value = serde_json::from_str(&list.stdout).unwrap();
    let agent = doc["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "sddk-explore")
        .unwrap();
    assert_eq!(agent["tier"], "premium");
    assert_eq!(agent["overrides"]["opencode"], "deepseek/deepseek-reasoner");
}

// M3 — an invalid `set` writes nothing to disk
#[test]
fn set_invalid_writes_nothing() {
    let (_tmp, environment) = bundle_fixture(config_yaml());
    let target = environment.sddk_data_dir.clone().unwrap();
    let file = target.join("framework/v0.0.0/assets/agent-models.yaml");
    let before = std::fs::read(&file).unwrap();

    let result = run(
        ModelsArgs {
            command: ModelsCommand::Set(ModelsSetArgs {
                file: None,
                agent: "orchestrator".to_owned(),
                tier: None,
                r#override: vec!["opencode=".to_owned()],
                clear_override: Vec::new(),
            }),
        },
        &environment,
    );
    assert_eq!(result.status, 2);
    assert_eq!(
        std::fs::read(&file).unwrap(),
        before,
        "file must be untouched"
    );
}

// M4 — atomic write creates missing parents; failure preserves the original
#[test]
fn set_atomic_on_missing_parent_and_failure_preserves_original() {
    let (_tmp, environment) = bundle_fixture(config_yaml());
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("a/b/c/agent-models.yaml");
    let ok = run(
        ModelsArgs {
            command: ModelsCommand::Set(ModelsSetArgs {
                file: Some(deep.clone()),
                agent: "orchestrator".to_owned(),
                tier: Some(super::ModelTier::Fast),
                r#override: Vec::new(),
                clear_override: Vec::new(),
            }),
        },
        &environment,
    );
    assert_eq!(ok.status, 0, "stderr: {}", ok.stderr);
    let written = std::fs::read_to_string(&deep).unwrap();
    assert!(written.contains("orchestrator"));

    // Invalid edit against the same file must leave its bytes intact.
    let before = std::fs::read(&deep).unwrap();
    let invalid = run(
        ModelsArgs {
            command: ModelsCommand::Set(ModelsSetArgs {
                file: Some(deep.clone()),
                agent: "orchestrator".to_owned(),
                tier: None,
                r#override: vec!["zcode=   ".to_owned()],
                clear_override: Vec::new(),
            }),
        },
        &environment,
    );
    assert_eq!(invalid.status, 2);
    assert_eq!(std::fs::read(&deep).unwrap(), before);
}

// M5 — JSON list shape is the stable contract the TUI consumes
#[test]
fn list_json_stable_for_tui() {
    let (_tmp, environment) = bundle_fixture(config_yaml());
    let list = run(
        ModelsArgs {
            command: ModelsCommand::List(ModelsListArgs {
                file: None,
                format: OutputFormat::Json,
            }),
        },
        &environment,
    );
    assert_eq!(list.status, 0);
    let doc: serde_json::Value = serde_json::from_str(&list.stdout).unwrap();
    assert!(doc["target"].is_string());
    assert!(doc["tiers"].is_object());
    assert_eq!(doc["tiers"]["premium"]["claude"], "sonnet");
    let agents = doc["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 2, "bundle agents only: {doc}");
    for agent in agents {
        assert!(agent["name"].is_string());
        assert!(agent["tier"].is_string());
        assert!(agent["overrides"].is_object());
    }
}
