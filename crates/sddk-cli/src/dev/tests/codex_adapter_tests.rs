//! Tests for the Codex adapter (I8, I9, codex halves of I10/I11).

use super::CodexAdapter;
use crate::dev::editor_adapters::reconcile::ReconcileContext;
use crate::dev::editor_adapters::test_fixtures::{self, ctx};
use crate::dev::editor_adapters::{EditorAdapter, RegistrationContext};
use std::collections::BTreeMap;

fn register_into(
    fixture: &test_fixtures::Fixture,
    dir: &std::path::Path,
) -> crate::dev::editor_adapters::AdapterReport {
    let adapter = CodexAdapter {
        dir: dir.to_path_buf(),
    };
    let context = ctx(fixture, Some(&fixture.models));
    adapter.register(&context)
}

// I8 — toml written from md body; adversarial content survives escaping.
#[test]
fn codex_toml_from_md_body() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.registered, 3, "{:?}", report.errors);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let parsed: toml::Value =
        toml::from_str(&std::fs::read_to_string(dir.path().join("agents/sddk-foo.toml")).unwrap())
            .unwrap();
    assert_eq!(parsed["name"], toml::Value::String("sddk-foo".into()));
    assert_eq!(
        parsed["description"],
        toml::Value::String("Foo explorer".into())
    );
    assert_eq!(
        parsed["model"],
        toml::Value::String("openai/gpt-5.4-fast".into())
    );
    assert_eq!(
        parsed["developer_instructions"],
        toml::Value::String("\n# Foo body\n".into())
    );

    // Adversarial body: newlines, quotes, backslashes, hashes, emoji.
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("agents")).unwrap();
    let body = "# Section\n\"double\" 'single' \\ backslash\n# hash line\nemoji: \u{1f680}\nline\twith\ttabs\n";
    std::fs::write(
        root.path().join("agents/sddk-foo.md"),
        format!("---\nname: sddk-foo\ndescription: Foo\n---\n{body}"),
    )
    .unwrap();
    let sources = crate::dev::editor_adapters::load_agent_sources(root.path());
    let context = RegistrationContext {
        root: root.path(),
        agents: &sources,
        models: Some(&fixture.models),
    };
    let out_dir = tempfile::tempdir().unwrap();
    let adapter = CodexAdapter {
        dir: out_dir.path().to_path_buf(),
    };
    let report = adapter.register(&context);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let written = std::fs::read_to_string(out_dir.path().join("agents/sddk-foo.toml")).unwrap();
    let reparsed: toml::Value = toml::from_str(&written).unwrap();
    assert_eq!(
        reparsed["developer_instructions"],
        toml::Value::String(format!("\n{body}")),
        "body must round-trip exactly through TOML escaping"
    );
}

// I9 — unsupported codex fields are omitted.
#[test]
fn codex_omits_unsupported_fields() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    register_into(&fixture, dir.path());
    let raw = std::fs::read_to_string(dir.path().join("agents/orchestrator.toml")).unwrap();
    assert!(!raw.contains("model_reasoning_effort"), "{raw}");
    assert!(!raw.contains("model_reasoning_summary"), "{raw}");
    let parsed: toml::Value = toml::from_str(&raw).unwrap();
    let keys: Vec<&str> = parsed
        .as_table()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec!["description", "developer_instructions", "model", "name"]
    );
}

// I10 (codex half) — first-time only: pre-existing file untouched.
#[test]
fn codex_first_time_only() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("agents")).unwrap();
    let target = dir.path().join("agents/orchestrator.toml");
    std::fs::write(
        &target,
        "name = \"orchestrator\"\ndescription = \"user edited\"\n",
    )
    .unwrap();
    let before = std::fs::read(&target).unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.skipped_existing, 1);
    assert_eq!(report.registered, 2);
    assert_eq!(std::fs::read(&target).unwrap(), before);
}

// I11 (codex half) — prune framework-namespaced orphans, keep user files.
#[test]
fn codex_prune_namespace_files() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("agents")).unwrap();
    std::fs::write(dir.path().join("agents/sddk-zombie.toml"), "stale").unwrap();
    std::fs::write(dir.path().join("agents/my-agent.toml"), "user").unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.pruned, 1);
    assert!(!dir.path().join("agents/sddk-zombie.toml").exists());
    assert!(dir.path().join("agents/my-agent.toml").exists());
}

// ConfigAbsent (codex): omit the `model` key.
#[test]
fn codex_config_absent_omits_model_key() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let context = RegistrationContext {
        root: fixture.root.path(),
        agents: &fixture.agents,
        models: None,
    };
    let adapter = CodexAdapter {
        dir: dir.path().to_path_buf(),
    };
    let report = adapter.register(&context);
    assert_eq!(report.registered, 3);
    let parsed: toml::Value = toml::from_str(
        &std::fs::read_to_string(dir.path().join("agents/orchestrator.toml")).unwrap(),
    )
    .unwrap();
    assert!(parsed.get("model").is_none());
}

// ── INC-DEBT-010: apply rename handlers ───────────────────────────────────────

/// Test that the Codex apply block renames the file when FieldDiff { field_name: "name" }
/// is present in the result.
/// Before fix: old file remains as orphan, new file gets created.
/// After fix: old file is removed, new file has canonical content.
#[test]
fn apply_renames_codex_file_on_name_diff() {
    use crate::dev::editor_adapters::reconcile::ReconcileAdapter;

    let tmp = tempfile::tempdir().unwrap();
    let agents_dir = tmp.path().join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();

    // Setup: codex/orchestrator.toml exists with orchestrator content
    let old_path = agents_dir.join("orchestrator.toml");
    let content = "name = \"orchestrator\"\ndescription = \"old\"\n";
    std::fs::write(&old_path, content).unwrap();

    // Bundle agent is "team-lead" (different from existing file "orchestrator")
    let bundle_root = tmp.path().join("bundle");
    let bundle_agents_dir = bundle_root.join("agents");
    std::fs::create_dir_all(&bundle_agents_dir).unwrap();
    let bundle_content = "---\nname: team-lead\ndescription: old\n---\n# body\n";
    std::fs::write(bundle_agents_dir.join("team-lead.md"), bundle_content).unwrap();

    let agents = crate::dev::editor_adapters::load_agent_sources(&bundle_root);
    let bundle_agent = agents.into_iter().find(|a| a.name == "team-lead").unwrap();

    let ctx = ReconcileContext {
        root: &bundle_root,
        agents: &[bundle_agent],
        models: None,
        renames: &BTreeMap::new(),
    };

    let adapter = CodexAdapter {
        dir: tmp.path().to_path_buf(),
    };

    // Call reconcile with apply=true
    let _report = adapter.reconcile(&ctx, true);

    let new_path = agents_dir.join("team-lead.toml");

    // After apply:
    // - team-lead.toml should exist with canonical content
    // - orchestrator.toml should NOT exist (renamed away)
    assert!(
        new_path.exists(),
        "team-lead.toml should exist after rename"
    );
    assert!(
        !old_path.exists(),
        "orchestrator.toml should NOT exist after rename: {:?}",
        agents_dir.read_dir()
    );
}

/// RED test: apply_rename_codex_file should rename the .toml file when
/// FieldDiff { field_name: "name" } is provided.
/// This test FAILS TO COMPILE until the helper is extracted in commit 4.
#[test]
fn apply_renames_codex_file_on_name_diff_helper() {
    use crate::dev::editor_adapters::AgentSource;
    use crate::dev::editor_adapters::codex::apply_rename_codex_file;
    use crate::dev::editor_adapters::reconcile::{ExistingEntry, FieldDiff, ReconcileTarget};
    use std::collections::BTreeMap;

    let tmp = tempfile::tempdir().unwrap();
    let agents_dir = tmp.path().join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();

    std::fs::write(
        agents_dir.join("orchestrator.toml"),
        "name = \"orchestrator\"\ndescription = \"old\"\n",
    )
    .unwrap();

    let agent = AgentSource {
        name: "team-lead".to_string(),
        description: "new".to_string(),
        body: "# new body\n".to_string(),
        tools: None,
        aliases: None,
    };

    let target = ReconcileTarget {
        name: "team-lead".to_string(),
        description: "new".to_string(),
        model: None,
        mode: None,
        hidden: None,
        prompt: None,
        tools: None,
    };

    let existing = ExistingEntry {
        name: "orchestrator".to_string(),
        description: Some("old".to_string()),
        model: None,
        mode: None,
        hidden: None,
        prompt: None,
        tools: None,
        extras: BTreeMap::new(),
    };

    let diff = FieldDiff {
        field_name: "name",
        old_value: Some(serde_json::Value::String("orchestrator".to_string())),
        new_value: Some(serde_json::Value::String("team-lead".to_string())),
    };

    let mut errors = Vec::new();
    apply_rename_codex_file(
        &agents_dir,
        &agent,
        &target,
        Some(&existing),
        &diff,
        &mut errors,
    );

    assert!(
        agents_dir.join("team-lead.toml").exists(),
        "team-lead.toml should exist"
    );
    assert!(
        !agents_dir.join("orchestrator.toml").exists(),
        "orchestrator.toml should NOT exist"
    );
    assert!(errors.is_empty(), "no errors expected: {errors:?}");
}
