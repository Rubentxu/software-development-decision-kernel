//! End-to-end tests for `dev link` four-editor dispatch (L1–L8).
//! All fixtures are temp dirs — the real `$HOME` is never touched.

use super::run_dev_link;
use crate::CliEnvironment;
use crate::dev::editor_adapters::test_fixtures::FIXTURE_YAML;
use crate::dev::{LinkArgs, LinkEditor, OutputFormat};
use std::path::Path;

fn link_fixture(config: Option<&str>) -> (tempfile::TempDir, CliEnvironment) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir_all(root.join("agents")).unwrap();
    for (name, description) in [
        ("orchestrator", "Team coordinator"),
        ("sddk-foo", "Foo explorer"),
        ("gentle-bar", "Bar reviewer"),
    ] {
        std::fs::write(
            root.join(format!("agents/{name}.md")),
            format!("---\nname: {name}\ndescription: {description}\n---\n# Body\n"),
        )
        .unwrap();
    }
    if let Some(yaml) = config {
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("assets/agent-models.yaml"), yaml).unwrap();
    }
    let environment = CliEnvironment {
        home: Some(tmp.path().join("home")),
        sddk_data_dir: Some(tmp.path().join("data")),
        ..CliEnvironment::default()
    };
    (tmp, environment)
}

fn args_all(root: &Path, tmp: &Path, editor: LinkEditor) -> LinkArgs {
    LinkArgs {
        root: root.to_path_buf(),
        editor,
        opencode_dir: Some(tmp.join("opencode")),
        zcode_dir: Some(tmp.join("zcode")),
        claude_dir: Some(tmp.join("claude")),
        codex_dir: Some(tmp.join("codex")),
        write_registry: false,
        format: OutputFormat::Json,
    }
}

// L1 — `--editor all` registers all four editors in native formats.
#[test]
fn link_all_registers_four_editors() {
    let (tmp, environment) = link_fixture(Some(FIXTURE_YAML));
    let root = tmp.path().join("root");
    let output = run_dev_link(args_all(&root, tmp.path(), LinkEditor::All), &environment);
    assert_eq!(output.status, 0, "{}", output.stderr);
    let reports: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
    let reports = reports.as_array().unwrap();
    assert_eq!(reports.len(), 4, "one report per editor");
    for report in reports {
        assert_eq!(report["agents_registered"], 3, "{report}");
        assert!(report["errors"].as_array().unwrap().is_empty(), "{report}");
    }
    for editor in ["opencode", "zcode"] {
        let config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join(editor).join(format!("{editor}.json")))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(config["agent"].as_object().unwrap().len(), 3);
        assert_eq!(
            config["agent"]["orchestrator"]["model"],
            "deepseek/deepseek-chat"
        );
    }
    assert!(tmp.path().join("claude/agents/orchestrator.md").is_file());
    assert!(tmp.path().join("claude/agents/sddk-foo.md").is_file());
    assert!(tmp.path().join("claude/agents/gentle-bar.md").is_file());
    assert!(tmp.path().join("codex/agents/orchestrator.toml").is_file());
    assert!(tmp.path().join("codex/agents/sddk-foo.toml").is_file());
    assert!(tmp.path().join("codex/agents/gentle-bar.toml").is_file());
    // claude/codex agents are native files, not symlinks.
    assert!(
        !std::fs::symlink_metadata(tmp.path().join("claude/agents/orchestrator.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

// L2 — single-editor selection touches only that editor.
#[test]
fn link_single_editor_touches_only_it() {
    let (tmp, environment) = link_fixture(Some(FIXTURE_YAML));
    let root = tmp.path().join("root");
    let output = run_dev_link(
        args_all(&root, tmp.path(), LinkEditor::OpenCode),
        &environment,
    );
    assert_eq!(output.status, 0, "{}", output.stderr);
    assert!(tmp.path().join("opencode/opencode.json").is_file());
    assert!(!tmp.path().join("zcode").exists());
    assert!(!tmp.path().join("claude").exists());
    assert!(!tmp.path().join("codex").exists());
}

// L3 — REGRESSION: user-set DeepSeek model survives; no MiniMax restored.
#[test]
fn link_preserves_user_deepseek() {
    let (tmp, environment) = link_fixture(Some(FIXTURE_YAML));
    let root = tmp.path().join("root");
    let opencode_dir = tmp.path().join("opencode");
    std::fs::create_dir_all(&opencode_dir).unwrap();
    let seeded = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "agent": {
            "orchestrator": {
                "description": "my migrated description",
                "mode": "primary",
                "model": "deepseek/deepseek-v4-pro",
                "prompt": "{file:/custom/orchestrator.md}"
            }
        },
        "mcp": {}
    });
    std::fs::write(
        opencode_dir.join("opencode.json"),
        serde_json::to_string_pretty(&seeded).unwrap(),
    )
    .unwrap();
    let output = run_dev_link(
        args_all(&root, tmp.path(), LinkEditor::OpenCode),
        &environment,
    );
    assert_eq!(output.status, 0, "{}", output.stderr);
    let content = std::fs::read_to_string(opencode_dir.join("opencode.json")).unwrap();
    assert!(
        content.contains("deepseek/deepseek-v4-pro"),
        "user model kept"
    );
    assert!(
        content.contains("my migrated description"),
        "user description kept"
    );
    assert!(
        !content.to_lowercase().contains("minimax"),
        "no MiniMax may be restored: {content}"
    );
    let reports: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(reports[0]["agents_skipped_existing"], 1);
}

// L4 — pruning is bounded to framework-namespaced entries across editors.
#[test]
fn link_prune_bounded() {
    let (tmp, environment) = link_fixture(Some(FIXTURE_YAML));
    let root = tmp.path().join("root");
    // opencode/zcode: stale + user JSON entries.
    for editor in ["opencode", "zcode"] {
        let dir = tmp.path().join(editor);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{editor}.json")),
            serde_json::to_string_pretty(&serde_json::json!({
                "$schema": "https://opencode.ai/config.json",
                "agent": {
                    "sddk-zombie": {"description": "stale", "mode": "subagent", "model": "x"},
                    "my-agent": {"description": "user", "mode": "primary", "model": "y"}
                },
                "mcp": {}
            }))
            .unwrap(),
        )
        .unwrap();
    }
    // claude/codex: stale + user native files.
    std::fs::create_dir_all(tmp.path().join("claude/agents")).unwrap();
    std::fs::write(tmp.path().join("claude/agents/sddk-zombie.md"), "stale").unwrap();
    std::fs::write(tmp.path().join("claude/agents/my-agent.md"), "user").unwrap();
    std::fs::create_dir_all(tmp.path().join("codex/agents")).unwrap();
    std::fs::write(tmp.path().join("codex/agents/sddk-zombie.toml"), "stale").unwrap();
    std::fs::write(tmp.path().join("codex/agents/my-agent.toml"), "user").unwrap();

    let output = run_dev_link(args_all(&root, tmp.path(), LinkEditor::All), &environment);
    assert_eq!(output.status, 0, "{}", output.stderr);
    for editor in ["opencode", "zcode"] {
        let config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join(editor).join(format!("{editor}.json")))
                .unwrap(),
        )
        .unwrap();
        assert!(
            config["agent"].get("sddk-zombie").is_none(),
            "{editor} zombie pruned"
        );
        assert_eq!(
            config["agent"]["my-agent"]["model"], "y",
            "{editor} user kept"
        );
    }
    assert!(!tmp.path().join("claude/agents/sddk-zombie.md").exists());
    assert!(tmp.path().join("claude/agents/my-agent.md").exists());
    assert!(!tmp.path().join("codex/agents/sddk-zombie.toml").exists());
    assert!(tmp.path().join("codex/agents/my-agent.toml").exists());
}

// L5 — ZeroConfigSurvival: no agent-models.yaml → registers without model,
// warns, exits 0.
#[test]
fn link_without_config_degrades() {
    let (tmp, environment) = link_fixture(None);
    let root = tmp.path().join("root");
    let output = run_dev_link(args_all(&root, tmp.path(), LinkEditor::All), &environment);
    assert_eq!(output.status, 0, "{}", output.stderr);
    assert!(
        output.stderr.contains("agent-models.yaml not found"),
        "warning expected: {}",
        output.stderr
    );
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tmp.path().join("opencode/opencode.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(config["agent"].as_object().unwrap().len(), 3);
    for (name, entry) in config["agent"].as_object().unwrap() {
        assert!(
            entry.get("model").is_none(),
            "agent {name} must have no model key"
        );
    }
    let claude = std::fs::read_to_string(tmp.path().join("claude/agents/orchestrator.md")).unwrap();
    assert!(!claude.contains("model:"), "{claude}");
    let codex: toml::Value = toml::from_str(
        &std::fs::read_to_string(tmp.path().join("codex/agents/orchestrator.toml")).unwrap(),
    )
    .unwrap();
    assert!(codex.get("model").is_none());
}

// L6 — one editor fails, others succeed, exit non-zero (PerEditorReporting).
#[cfg(unix)]
#[test]
fn link_partial_failure_nonzero() {
    let (tmp, environment) = link_fixture(Some(FIXTURE_YAML));
    let root = tmp.path().join("root");
    let claude_dir = tmp.path().join("claude");
    std::fs::create_dir_all(&claude_dir).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&claude_dir, std::fs::Permissions::from_mode(0o000)).unwrap();

    let output = run_dev_link(args_all(&root, tmp.path(), LinkEditor::All), &environment);
    // Restore permissions so tempdir cleanup works.
    std::fs::set_permissions(&claude_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(output.status, 1, "any editor failure must fail the command");
    let reports: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
    let reports = reports.as_array().unwrap();
    let claude_report = reports
        .iter()
        .find(|r| r["editor"].as_str().unwrap().contains("claude"))
        .unwrap();
    assert!(!claude_report["errors"].as_array().unwrap().is_empty());
    // Others still registered.
    assert_eq!(
        reports
            .iter()
            .find(|r| r["editor"].as_str().unwrap().contains("opencode"))
            .unwrap()["agents_registered"],
        3
    );
    assert_eq!(
        reports
            .iter()
            .find(|r| r["editor"].as_str().unwrap().contains("codex"))
            .unwrap()["agents_registered"],
        3
    );
}

// L7 — report includes registration counts.
#[test]
fn link_report_contains_registration_counts() {
    let (tmp, environment) = link_fixture(Some(FIXTURE_YAML));
    let root = tmp.path().join("root");
    let mut args = args_all(&root, tmp.path(), LinkEditor::OpenCode);
    args.format = OutputFormat::Text;
    let output = run_dev_link(args, &environment);
    assert_eq!(output.status, 0, "{}", output.stderr);
    assert!(output.stdout.contains("registered: 3"), "{}", output.stdout);
    assert!(
        output.stdout.contains("skipped_existing: 0"),
        "{}",
        output.stdout
    );
    assert!(
        output.stdout.contains("skipped_unresolved: 0"),
        "{}",
        output.stdout
    );
}

// L8 — no hardcoded fallback: unconfigured agent skipped, never minimax.
#[test]
fn link_no_hardcoded_fallback() {
    let config_without_gentle_bar = FIXTURE_YAML.replace("  gentle-bar:\n    tier: fast\n", "");
    let (tmp, environment) = link_fixture(Some(&config_without_gentle_bar));
    let root = tmp.path().join("root");
    let output = run_dev_link(args_all(&root, tmp.path(), LinkEditor::All), &environment);
    assert_eq!(output.status, 0, "{}", output.stderr);
    let reports: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
    for report in reports.as_array().unwrap() {
        assert_eq!(report["agents_registered"], 2, "{report}");
        assert_eq!(report["agents_skipped_unresolved"], 1, "{report}");
    }
    // No editor file may contain a minimax model id.
    let mut haystacks = vec![
        std::fs::read_to_string(tmp.path().join("opencode/opencode.json")).unwrap(),
        std::fs::read_to_string(tmp.path().join("zcode/zcode.json")).unwrap(),
    ];
    for file in [
        "claude/agents/orchestrator.md",
        "claude/agents/sddk-foo.md",
        "codex/agents/orchestrator.toml",
        "codex/agents/sddk-foo.toml",
    ] {
        haystacks.push(std::fs::read_to_string(tmp.path().join(file)).unwrap());
    }
    for content in &haystacks {
        assert!(
            !content.to_lowercase().contains("minimax"),
            "hardcoded fallback detected: {content}"
        );
    }
}
