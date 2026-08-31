//! Tests for the JSON agent-map registration core (I1–I5, I12–I13).

use super::{IdeKey, upsert_json_agents};
use crate::dev::editor_adapters::test_fixtures::{self, ctx};

fn temp_config_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// I1 — idempotent double registration: second run sees everything existing.
#[test]
fn json_idempotent_double_register() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    let context = ctx(&fixture, Some(&fixture.models));
    let first = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(first.registered, 3);
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    let bytes = std::fs::read(&path).unwrap();
    let second = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(second.registered, 0);
    assert_eq!(second.skipped_existing, 3);
    assert_eq!(second.pruned, 0);
    assert!(second.errors.is_empty(), "{:?}", second.errors);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "file must be byte-identical"
    );
}

// I2 — user-set model/description survive byte-identical.
#[test]
fn json_user_model_survives() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    let seeded = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "agent": {
            "orchestrator": {
                "description": "my custom description",
                "mode": "primary",
                "model": "deepseek/deepseek-v4-pro",
                "prompt": "{file:/custom/path.md}"
            },
            "sddk-foo": {
                "description": "user edited foo",
                "mode": "subagent",
                "hidden": true,
                "model": "deepseek/deepseek-reasoner",
                "prompt": "{file:/custom/foo.md}"
            },
            "gentle-bar": {
                "description": "user edited bar",
                "mode": "subagent",
                "hidden": true,
                "model": "zai-coding-plan/glm-5-turbo",
                "prompt": "{file:/custom/bar.md}"
            }
        },
        "mcp": {}
    });
    let seeded_bytes = serde_json::to_string_pretty(&seeded).unwrap();
    std::fs::write(&path, &seeded_bytes).unwrap();
    let context = ctx(&fixture, Some(&fixture.models));
    let report = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(report.skipped_existing, 3);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        seeded_bytes.as_bytes(),
        "user entries must remain byte-identical (no write at all)"
    );
}

// I3 — first-time entry created with model from override resolution.
#[test]
fn json_first_time_creates() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    let context = ctx(&fixture, Some(&fixture.models));
    let report = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(report.registered, 3);
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let foo = &config["agent"]["sddk-foo"];
    assert_eq!(
        foo["model"], "deepseek/deepseek-reasoner",
        "override must win"
    );
    assert_eq!(foo["description"], "Foo explorer");
    assert_eq!(foo["mode"], "subagent");
    assert_eq!(foo["hidden"], true);
    assert_eq!(
        foo["prompt"],
        format!(
            "{{file:{}}}",
            fixture.root.path().join("agents/sddk-foo.md").display()
        )
    );
    let orchestrator = &config["agent"]["orchestrator"];
    assert_eq!(
        orchestrator["mode"], "primary",
        "PRIMARY_AGENTS must be primary"
    );
    assert_eq!(orchestrator["model"], "deepseek/deepseek-chat");
    assert!(orchestrator.get("hidden").is_none());
}

// I3.5 — when an existing entry's prompt path looks like a previous sddk
// install (stale dogfooding repo path), refresh it to the new bundle root
// while preserving user customizations (model, description, hidden).
#[test]
fn json_refreshes_stale_framework_prompt_path() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    let new_root = fixture.root.path();
    // Use a path that the heuristic recognises as a previous sddk install
    // (contains `/sddk-framework/agents/`).
    let stale_root = new_root.parent().unwrap().join("sddk-framework");
    let seeded = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "agent": {
            "orchestrator": {
                "description": "user tweaked the description",
                "mode": "primary",
                "model": "deepseek/deepseek-chat",
                "prompt": format!("{{file:{}}}", stale_root.join("agents/orchestrator.md").display())
            },
            "sddk-foo": {
                "description": "Foo explorer",
                "mode": "subagent",
                "hidden": true,
                "model": "deepseek/deepseek-reasoner",
                "prompt": format!("{{file:{}}}", stale_root.join("agents/sddk-foo.md").display())
            }
        },
        "mcp": {}
    });
    std::fs::write(&path, serde_json::to_string_pretty(&seeded).unwrap()).unwrap();
    let context = ctx(&fixture, Some(&fixture.models));
    let report = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(report.updated_stale, 2, "both stale paths should refresh");
    assert_eq!(
        report.skipped_existing, 0,
        "stale paths must not be skipped"
    );
    assert_eq!(
        report.registered, 1,
        "the third fixture agent (gentle-bar) was missing from the seed and must be registered"
    );
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let orchestrator = &config["agent"]["orchestrator"];
    assert_eq!(
        orchestrator["prompt"],
        format!(
            "{{file:{}}}",
            new_root.join("agents/orchestrator.md").display()
        ),
        "prompt must point at the new bundle root"
    );
    assert_eq!(
        orchestrator["description"], "user tweaked the description",
        "user-customized description must survive the refresh"
    );
    assert_eq!(orchestrator["model"], "deepseek/deepseek-chat");
    let foo = &config["agent"]["sddk-foo"];
    assert_eq!(
        foo["prompt"],
        format!("{{file:{}}}", new_root.join("agents/sddk-foo.md").display())
    );
    assert_eq!(foo["model"], "deepseek/deepseek-reasoner");
    assert_eq!(foo["hidden"], true);
}

// I3.6 — when an existing entry's prompt path looks USER-customized
// (not a previous sddk install), do NOT touch it: preserve byte-untouched.
#[test]
fn json_preserves_user_customized_prompt_path() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    let seeded = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "agent": {
            "orchestrator": {
                "description": "my custom orchestrator",
                "mode": "primary",
                "model": "deepseek/deepseek-chat",
                "prompt": "{file:/my/personal/prompts/orchestrator.md}"
            }
        },
        "mcp": {}
    });
    let seeded_bytes = serde_json::to_string_pretty(&seeded).unwrap();
    std::fs::write(&path, &seeded_bytes).unwrap();
    let context = ctx(&fixture, Some(&fixture.models));
    let report = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(report.skipped_existing, 1, "user path must be preserved");
    assert_eq!(report.updated_stale, 0, "user path must not be touched");
    assert_eq!(
        report.registered, 2,
        "fixture agents missing from the seed (sddk-foo, gentle-bar) must be registered"
    );
    // The user-customized orchestrator entry is preserved byte-untouched:
    // its description, mode, model and prompt all match the seeded values.
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        config["agent"]["orchestrator"], seeded["agent"]["orchestrator"],
        "user-customized orchestrator entry must remain byte-untouched"
    );
}

// I4 — prune removes only framework-namespaced orphans; user entries kept.
#[test]
fn json_prunes_framework_orphan_only() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {
                "sddk-zombie": { "description": "stale", "mode": "subagent", "model": "x" },
                "my-agent": { "description": "user", "mode": "primary", "model": "y" }
            },
            "mcp": {}
        }))
        .unwrap(),
    )
    .unwrap();
    let context = ctx(&fixture, Some(&fixture.models));
    let report = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(report.pruned, 1);
    assert_eq!(report.registered, 3);
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(config["agent"].get("sddk-zombie").is_none());
    assert_eq!(config["agent"]["my-agent"]["model"], "y");
    assert_eq!(config["agent"]["my-agent"]["description"], "user");
}

// I5 — zcode mirrors opencode: same agent set, same schema.
#[test]
fn zcode_mirrors_opencode_schema() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let opencode_path = dir.path().join("opencode.json");
    let zcode_path = dir.path().join("zcode.json");
    let context = ctx(&fixture, Some(&fixture.models));
    let opencode_report = upsert_json_agents(
        &opencode_path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    let zcode_report = upsert_json_agents(
        &zcode_path,
        IdeKey::Zcode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(opencode_report.registered, 3);
    assert_eq!(zcode_report.registered, 3);
    let opencode: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&opencode_path).unwrap()).unwrap();
    let zcode: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&zcode_path).unwrap()).unwrap();
    let opencode_agents = opencode["agent"].as_object().unwrap();
    let zcode_agents = zcode["agent"].as_object().unwrap();
    assert_eq!(opencode_agents.len(), zcode_agents.len());
    for (name, entry) in opencode_agents {
        let mirror = &zcode_agents[name];
        for key in ["description", "mode", "hidden", "prompt"] {
            assert_eq!(
                entry[key], mirror[key],
                "agent {name} key {key} must mirror"
            );
        }
        // Model may legitimately differ (per-IDE overrides) but the key
        // must exist in both — same schema.
        assert!(entry.get("model").is_some() && mirror.get("model").is_some());
    }
}

// I12 — ConfigAbsent: entries written without a `model` key.
#[test]
fn config_absent_omits_model_key() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    let context = ctx(&fixture, None);
    let report = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(report.registered, 3, "ConfigAbsent still registers agents");
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for (name, entry) in config["agent"].as_object().unwrap() {
        assert!(
            entry.get("model").is_none(),
            "agent {name} must have no model key when config is absent"
        );
    }
}

// I13 — NoModelConfigured skips the agent; others still register.
#[test]
fn no_model_configured_skips_agent() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("opencode.json");
    // Fast tier table lacks opencode → sddk-foo and gentle-bar unresolvable.
    let yaml = test_fixtures::FIXTURE_YAML.replace(
        "fast:\n    opencode: zai-coding-plan/glm-5-turbo\n    zcode: zai-coding-plan/glm-5-turbo",
        "fast:\n    zcode: zai-coding-plan/glm-5-turbo",
    );
    let mut models = crate::dev::agent_models::AgentModelsConfig::from_yaml(&yaml).unwrap();
    models.clear_override("sddk-foo", IdeKey::Opencode);
    let context = ctx(&fixture, Some(&models));
    let report = upsert_json_agents(
        &path,
        IdeKey::Opencode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(
        report.registered, 1,
        "orchestrator resolves via premium table"
    );
    assert_eq!(report.skipped_unresolved, 2);
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(config["agent"].get("sddk-foo").is_none());
    assert!(config["agent"].get("orchestrator").is_some());
}

// ── INC-DEBT-010: apply_rename_in_agents_map helper ───────────────────────────

/// RED test: apply_rename_in_agents_map should rename the JSON map key when
/// FieldDiff { field_name: "name" } is provided.
/// This test FAILS TO COMPILE until the helper is extracted in commit 4.
#[test]
fn apply_renames_json_key_on_name_diff() {
    use crate::dev::editor_adapters::json::apply_rename_in_agents_map;
    use crate::dev::editor_adapters::reconcile::FieldDiff;

    let mut agents = serde_json::Map::new();
    agents.insert(
        "orchestrator".to_string(),
        serde_json::json!({"description": "old", "model": "old-model", "mode": "primary"}),
    );

    let diff = FieldDiff {
        field_name: "name",
        old_value: Some(serde_json::Value::String("orchestrator".to_string())),
        new_value: Some(serde_json::Value::String("team-lead".to_string())),
    };

    let mut errors = Vec::new();
    apply_rename_in_agents_map(&mut agents, &diff, &mut errors);

    assert!(
        agents.contains_key("team-lead"),
        "team-lead should exist after rename"
    );
    assert!(
        !agents.contains_key("orchestrator"),
        "orchestrator should NOT exist after rename"
    );
    assert_eq!(
        agents
            .get("team-lead")
            .unwrap()
            .get("description")
            .unwrap()
            .as_str()
            .unwrap(),
        "old"
    );
    assert!(errors.is_empty(), "no errors expected: {errors:?}");
}

// zcode prune — same bounded namespace rule as opencode (mirror parity).
#[test]
fn zcode_prunes_framework_orphan_only() {
    let fixture = test_fixtures::build();
    let dir = temp_config_dir();
    let path = dir.path().join("zcode.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {
                "sddk-zombie": { "description": "stale", "mode": "subagent", "model": "x" },
                "my-agent": { "description": "user", "mode": "primary", "model": "y" }
            },
            "mcp": {}
        }))
        .unwrap(),
    )
    .unwrap();
    let context = ctx(&fixture, Some(&fixture.models));
    let report = upsert_json_agents(
        &path,
        IdeKey::Zcode,
        &super::super::PRIMARY_AGENTS,
        &context,
    );
    assert_eq!(report.pruned, 1);
    assert_eq!(report.registered, 3);
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(config["agent"].get("sddk-zombie").is_none());
    assert_eq!(config["agent"]["my-agent"]["model"], "y");
}
