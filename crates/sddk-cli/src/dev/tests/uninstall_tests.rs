//! Tests for `dev uninstall` claude/codex native-file pruning.

use crate::dev::uninstall::{run_dev_uninstall, uninstall_native_editor};
use crate::dev::{LinkEditor, OutputFormat, UninstallArgs};

fn fixture_root() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::write(
        root.join("agents/orchestrator.md"),
        "---\nname: orchestrator\ndescription: coord\n---\n# Body\n",
    )
    .unwrap();
    (tmp, root)
}

// Native prune: framework-namespaced claude/codex files removed, user kept.
#[test]
fn native_editor_prunes_framework_files_keeps_user() {
    let (tmp, root) = fixture_root();
    let claude = tmp.path().join("claude");
    std::fs::create_dir_all(claude.join("agents")).unwrap();
    std::fs::write(claude.join("agents/sddk-apply.md"), "framework").unwrap();
    std::fs::write(claude.join("agents/my-agent.md"), "user").unwrap();
    std::fs::create_dir_all(claude.join("skills")).unwrap();
    std::os::unix::fs::symlink(
        root.join("skills/nonexistent"),
        claude.join("skills/sddk-linked"),
    )
    .ok();

    let report = uninstall_native_editor(&root, &claude, "md").unwrap();
    assert_eq!(report.entries_removed, 1, "{:?}", report.errors);
    assert!(!claude.join("agents/sddk-apply.md").exists());
    assert!(claude.join("agents/my-agent.md").exists(), "user file kept");
}

// Full command: `--editor all` touches the four editors; user files survive.
#[test]
fn uninstall_all_reports_four_editors() {
    let (tmp, root) = fixture_root();
    let claude = tmp.path().join("claude");
    std::fs::create_dir_all(claude.join("agents")).unwrap();
    std::fs::write(claude.join("agents/gentle-bar.md"), "framework").unwrap();
    std::fs::write(claude.join("agents/my-agent.md"), "user").unwrap();
    let codex = tmp.path().join("codex");
    std::fs::create_dir_all(codex.join("agents")).unwrap();
    std::fs::write(codex.join("agents/sddk-explore.toml"), "framework").unwrap();
    std::fs::write(codex.join("agents/my-agent.toml"), "user").unwrap();

    let args = UninstallArgs {
        prefix: None,
        editor: Some(LinkEditor::All),
        root: root.clone(),
        opencode_dir: Some(tmp.path().join("opencode")),
        zcode_dir: Some(tmp.path().join("zcode")),
        claude_dir: Some(claude.clone()),
        codex_dir: Some(codex.clone()),
        format: OutputFormat::Text,
    };
    let output = run_dev_uninstall(args);
    assert_eq!(output.status, 0, "{}", output.stderr);
    assert!(
        output
            .stdout
            .contains("claude: 1 native agent files removed"),
        "{}",
        output.stdout
    );
    assert!(
        output
            .stdout
            .contains("codex: 1 native agent files removed"),
        "{}",
        output.stdout
    );
    assert!(output.stdout.contains("opencode:"), "{}", output.stdout);
    assert!(output.stdout.contains("zcode:"), "{}", output.stdout);
    assert!(claude.join("agents/my-agent.md").exists(), "user file kept");
    assert!(
        codex.join("agents/my-agent.toml").exists(),
        "user file kept"
    );
}

// Doctor smoke on a temp HOME: advisory claude/codex dir checks render.
#[test]
fn doctor_reports_claude_codex_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(home.join(".claude")).unwrap();
    let environment = crate::CliEnvironment {
        home: Some(home),
        ..crate::CliEnvironment::default()
    };
    let args = crate::dev::DoctorArgs {
        format: OutputFormat::Text,
        strict: false,
        prefix: None,
    };
    let output = crate::dev::doctor::run_dev_doctor(args, &environment);
    assert!(
        output.stdout.contains("editor.claude_dir"),
        "{}",
        output.stdout
    );
    assert!(
        output.stdout.contains("editor.codex_dir"),
        "{}",
        output.stdout
    );
}
