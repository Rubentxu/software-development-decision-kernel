//! Tests for the ZCode adapter (ADR-0081): native md sub-agents,
//! primary-agent command files, agent-map era migration, bounded pruning.

use super::super::EditorAdapter;
use super::super::reconcile::{ReconcileAdapter, ReconcileContext};
use super::super::test_fixtures::{self, ctx};
use super::ZCodeAdapter;
use crate::dev::agent_models::AgentModelsConfig;
use std::collections::BTreeMap;

fn register_into(
    fixture: &test_fixtures::Fixture,
    dir: &std::path::Path,
) -> super::super::AdapterReport {
    let adapter = ZCodeAdapter {
        dir: dir.to_path_buf(),
    };
    let context = ctx(fixture, Some(&fixture.models));
    adapter.register(&context)
}

fn reconcile_ctx<'a>(
    fixture: &'a test_fixtures::Fixture,
    models: Option<&'a AgentModelsConfig>,
    renames: &'a BTreeMap<String, String>,
) -> ReconcileContext<'a> {
    ReconcileContext {
        root: fixture.root.path(),
        agents: &fixture.agents,
        models,
        renames,
    }
}

// ADR-0081 — sub-agents are native `agents/<name>.md` files with required
// `name` frontmatter; primary agents become command files instead.
#[test]
fn register_writes_native_subagents_and_primary_command() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.registered, 3, "{:?}", report.errors);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    // Sub-agent: native md with required name + tier-mapped model + inline body.
    let foo = std::fs::read_to_string(dir.path().join("agents/sddk-foo.md")).unwrap();
    assert!(
        foo.starts_with("---\nname: sddk-foo\ndescription: Foo explorer\n"),
        "{foo}"
    );
    assert!(
        foo.contains("model: zai-coding-plan/glm-5-turbo\n"),
        "{foo}"
    );
    assert!(foo.contains("tools: read, bash\n"), "{foo}");
    assert!(foo.ends_with("# Foo body\n"));

    // Primary agent: command file under commands/, NOT an agent file.
    assert!(!dir.path().join("agents/orchestrator.md").exists());
    let orchestrator =
        std::fs::read_to_string(dir.path().join("commands/orchestrator.md")).unwrap();
    assert!(
        orchestrator.contains("description: Team coordinator\n"),
        "{orchestrator}"
    );
    assert!(orchestrator.contains("source: sddk\n"), "{orchestrator}");
    assert!(
        orchestrator.contains("model: deepseek/deepseek-chat\n"),
        "{orchestrator}"
    );
    assert!(orchestrator.contains("# Orchestrator body\n"));

    // No agent map is written: ZCode does not read `zcode.json`.
    assert!(!dir.path().join("zcode.json").exists());

    // Agent files are real files, not symlinks into the framework.
    let meta = std::fs::symlink_metadata(dir.path().join("agents/sddk-foo.md")).unwrap();
    assert!(!meta.file_type().is_symlink());
}

// First-time-only invariant: a second register run touches nothing.
#[test]
fn register_is_idempotent() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let first = register_into(&fixture, dir.path());
    assert_eq!(first.registered, 3);
    let second = register_into(&fixture, dir.path());
    assert_eq!(second.registered, 0, "{:?}", second);
    assert_eq!(second.skipped_existing, 3, "{:?}", second);
    assert_eq!(second.updated_stale, 0, "{:?}", second);
}

// Agent-map era artifacts (symlinks, `name`-less stale writes) are replaced
// in place; the primary agent's legacy md is removed in favor of the command.
#[test]
fn register_migrates_agent_map_era_artifacts() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents");
    std::fs::create_dir_all(&agents).unwrap();
    std::os::unix::fs::symlink(
        "/nonexistent/framework/agents/sddk-foo.md",
        agents.join("sddk-foo.md"),
    )
    .unwrap();
    std::fs::write(
        agents.join("gentle-bar.md"),
        "---\ndescription: Bar reviewer\n---\nold body\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "/nonexistent/framework/agents/orchestrator.md",
        agents.join("orchestrator.md"),
    )
    .unwrap();

    let report = register_into(&fixture, dir.path());
    assert_eq!(report.updated_stale, 2, "{:?}", report);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let foo = std::fs::read_to_string(agents.join("sddk-foo.md")).unwrap();
    assert!(foo.contains("name: sddk-foo\n"), "{foo}");
    let bar = std::fs::read_to_string(agents.join("gentle-bar.md")).unwrap();
    assert!(bar.contains("name: gentle-bar\n"), "{bar}");
    assert!(!bar.contains("old body"), "{bar}");
    assert!(
        !std::fs::symlink_metadata(agents.join("sddk-foo.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );

    // Primary legacy artifact removed; command registered instead.
    assert!(!agents.join("orchestrator.md").exists());
    assert!(dir.path().join("commands/orchestrator.md").is_file());
}

// Model vocabulary: ids without a `provider/` slash are rejected with an
// error and the agent is skipped, never written with a broken model.
#[test]
fn register_rejects_invalid_model_vocabulary() {
    let fixture = test_fixtures::build();
    let models = AgentModelsConfig::from_yaml(
        "tiers:\n  premium:\n    opencode: deepseek/deepseek-chat\n    zcode: deepseek/deepseek-chat\n    claude: sonnet\n    codex: openai/gpt-5.4\n  fast:\n    opencode: zai-coding-plan/glm-5-turbo\n    zcode: zai-coding-plan/glm-5-turbo\n    claude: haiku\n    codex: openai/gpt-5.4-fast\nagents:\n  orchestrator:\n    tier: premium\n  sddk-foo:\n    tier: fast\n    overrides:\n      zcode: minimax-no-slash\n  gentle-bar:\n    tier: fast\n",
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let adapter = ZCodeAdapter {
        dir: dir.path().to_path_buf(),
    };
    let report = adapter.register(&test_fixtures::ctx(&fixture, Some(&models)));
    assert_eq!(report.registered, 2, "{:?}", report);
    assert_eq!(report.skipped_unresolved, 1, "{:?}", report);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("not in zcode vocabulary")),
        "{:?}",
        report.errors
    );
    assert!(!dir.path().join("agents/sddk-foo.md").exists());
}

// Pruning is bounded: sddk-owned orphans and marker commands are removed;
// user agent files and non-marker commands are never touched.
#[test]
fn prune_is_bounded_to_sddk_owned_artifacts() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let agents = dir.path().join("agents");
    let commands = dir.path().join("commands");
    std::fs::create_dir_all(&agents).unwrap();
    std::fs::create_dir_all(&commands).unwrap();
    std::fs::write(
        agents.join("sddk-zombie.md"),
        "---\ndescription: stale\n---\n",
    )
    .unwrap();
    std::fs::write(
        agents.join("my-agent.md"),
        "---\nname: my-agent\n---\nuser\n",
    )
    .unwrap();
    std::fs::write(
        commands.join("sddk-zombie.md"),
        "---\ndescription: stale\nsource: sddk\n---\n",
    )
    .unwrap();
    std::fs::write(
        commands.join("my-command.md"),
        "---\ndescription: user command\n---\n",
    )
    .unwrap();

    let report = register_into(&fixture, dir.path());
    assert_eq!(report.pruned, 2, "{:?}", report);
    assert!(!agents.join("sddk-zombie.md").exists());
    assert!(agents.join("my-agent.md").exists(), "user agent kept");
    assert!(!commands.join("sddk-zombie.md").exists());
    assert!(commands.join("my-command.md").exists(), "user command kept");
}

// Reconcile detects model drift and rewrites the native file on apply;
// dry-run reports the change without touching the file.
#[test]
fn reconcile_updates_drifted_model() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    register_into(&fixture, dir.path());

    let drifted = AgentModelsConfig::from_yaml(
        "tiers:\n  premium:\n    opencode: deepseek/deepseek-chat\n    zcode: deepseek/deepseek-chat\n    claude: sonnet\n    codex: openai/gpt-5.4\n  fast:\n    opencode: zai-coding-plan/glm-5-turbo\n    zcode: zai-coding-plan/glm-5-turbo\n    claude: haiku\n    codex: openai/gpt-5.4-fast\nagents:\n  orchestrator:\n    tier: premium\n  sddk-foo:\n    tier: fast\n    overrides:\n      zcode: deepseek/deepseek-chat\n  gentle-bar:\n    tier: fast\n",
    )
    .unwrap();
    let renames = super::super::renames_builder(&fixture.agents);
    let adapter = ZCodeAdapter {
        dir: dir.path().to_path_buf(),
    };

    let target_path = dir.path().join("agents/sddk-foo.md");
    let before = std::fs::read_to_string(&target_path).unwrap();

    let dry_run = adapter.reconcile(&reconcile_ctx(&fixture, Some(&drifted), &renames), false);
    assert_eq!(dry_run.agents_changed, 1, "{:?}", dry_run);
    assert_eq!(
        std::fs::read_to_string(&target_path).unwrap(),
        before,
        "dry-run must not touch the file"
    );

    let applied = adapter.reconcile(&reconcile_ctx(&fixture, Some(&drifted), &renames), true);
    assert_eq!(applied.agents_changed, 1, "{:?}", applied);
    let after = std::fs::read_to_string(&target_path).unwrap();
    assert!(after.contains("model: deepseek/deepseek-chat\n"), "{after}");

    // A second reconcile against the same models is clean.
    let clean = adapter.reconcile(&reconcile_ctx(&fixture, Some(&drifted), &renames), false);
    assert_eq!(clean.agents_changed, 0, "{:?}", clean);
}

// Reconcile backfills a primary command file that predates the model field.
#[test]
fn reconcile_backfills_command_model() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    register_into(&fixture, dir.path());
    let command_path = dir.path().join("commands/orchestrator.md");
    std::fs::write(
        &command_path,
        "---\ndescription: Team coordinator\nargument-hint: <goal>\nsource: sddk\n---\nold body\n",
    )
    .unwrap();

    let renames = super::super::renames_builder(&fixture.agents);
    let adapter = ZCodeAdapter {
        dir: dir.path().to_path_buf(),
    };
    let report = adapter.reconcile(
        &reconcile_ctx(&fixture, Some(&fixture.models), &renames),
        true,
    );
    assert_eq!(report.agents_changed, 1, "{:?}", report);
    let content = std::fs::read_to_string(&command_path).unwrap();
    assert!(
        content.contains("model: deepseek/deepseek-chat\n"),
        "{content}"
    );
    assert!(content.contains("# Orchestrator body\n"), "{content}");
}

// zcode model vocabulary: only full provider/model ids.
#[test]
fn zcode_model_vocabulary() {
    assert!(super::zcode_model_valid("deepseek/deepseek-chat"));
    assert!(super::zcode_model_valid("zai-coding-plan/glm-5-turbo"));
    assert!(!super::zcode_model_valid("sonnet"));
    assert!(!super::zcode_model_valid("minimax"));
}

// Generated command files carry the sddk marker; hand-written files do not
// match even when they share a framework-namespaced name.
#[test]
fn sddk_command_marker_detection() {
    let dir = tempfile::tempdir().unwrap();
    let marked = dir.path().join("orchestrator.md");
    std::fs::write(&marked, "---\ndescription: x\nsource: sddk\n---\nbody\n").unwrap();
    assert!(super::is_sddk_command(&marked));

    let unmarked = dir.path().join("sddk-zombie.md");
    std::fs::write(&unmarked, "---\ndescription: x\n---\nbody\n").unwrap();
    assert!(!super::is_sddk_command(&unmarked));
}
