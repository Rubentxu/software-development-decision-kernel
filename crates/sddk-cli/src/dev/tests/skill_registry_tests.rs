//! Tests for skill registry — extracted to dev/tests/ to keep registry.rs below LOC ceiling.

use crate::CliEnvironment;
use crate::dev::registry::write_skill_registry;

fn temp_project(tag: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sddk-reg-prj-{tag}-{n}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn temp_framework(tag: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sddk-reg-frm-{tag}-{n}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Environment with sddk_data_dir and home pointing at the temp location so the
/// registry is written and user-scope skills are scanned from a directory we control.
fn test_environment(temp_root: &std::path::Path) -> CliEnvironment {
    CliEnvironment {
        home: Some(temp_root.to_path_buf()),
        data_home: None,
        sddk_data_dir: Some(temp_root.to_path_buf()),
        state_home: None,
        cache_home: None,
        sddk_actor: None,
        user: None,
    }
}

/// Create a minimal SKILL.md with name and description frontmatter.
fn make_skill(dir: &std::path::Path, name: &str, description: &str) {
    let content = format!("---\nname: {name}\ndescription: \"{description}\"\n---\n",);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), content).unwrap();
}

/// Initialize a fake git repo with a fake remote so `crate::resolve_remote`
/// returns a deterministic URL and the registry identity resolver produces a
/// stable p-*. Uses `git init` so the git command succeeds.
fn init_fake_git_remote(dir: &std::path::Path) {
    // Run `git init` so git commands work in this temp dir.
    let init_output = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(dir)
        .output();
    if init_output.map(|o| !o.status.success()).unwrap_or(true) {
        // git not available or failed — skip.
        return;
    }
    // Use a fixed remote URL so the p-* ID is deterministic for the same dir path.
    let remote_url = "https://test.example.com/sddk-framework.git";
    let _ = std::process::Command::new("git")
        .args(["remote", "add", "origin", remote_url])
        .current_dir(dir)
        .output();
}

#[test]
fn write_skill_registry_is_idempotent_and_dedupes() {
    // project_root drives project-ID computation; framework_root is scanned for skills.
    // Both are temp dirs here so we control the entire outcome.
    let project = temp_project("idempotent");
    let framework = temp_framework("idempotent");
    let env = test_environment(&project);

    // Initialize a fake git remote so resolve_remote returns a deterministic URL.
    init_fake_git_remote(&framework);

    // Skills live in framework scope (mirrors real dogfooding: sddk-framework IS the adopted workspace).
    make_skill(
        &framework.join("skills/sddk-apply"),
        "sddk-apply",
        "Apply SDD tasks",
    );
    make_skill(
        &framework.join("skills/sddk-design"),
        "sddk-design",
        "Design SDD solutions",
    );
    // _shared should be skipped.
    make_skill(
        &framework.join("skills/_shared"),
        "_shared",
        "Shared internal",
    );
    // skill-registry should be skipped.
    make_skill(
        &framework.join("skills/skill-registry"),
        "skill-registry",
        "Registry indexer",
    );

    // Pass framework as project_root so project-ID derives from the dir that holds the skills.
    let (path1, count1) = write_skill_registry(&env, &framework, &framework).unwrap();
    assert_eq!(
        count1, 2,
        "only sddk-apply and sddk-design should be included"
    );

    // Second invocation must produce byte-identical output (idempotent).
    let (path2, count2) = write_skill_registry(&env, &framework, &framework).unwrap();
    assert_eq!(count2, 2);
    let content1 = std::fs::read_to_string(&path1).unwrap();
    let content2 = std::fs::read_to_string(&path2).unwrap();
    assert_eq!(
        content1, content2,
        "second invocation must be byte-identical (idempotent)"
    );

    // Verify schema: table has 5 columns.
    assert!(
        content1.contains("| Name | Trigger | Description | Scope | Path |"),
        "registry must have correct header"
    );

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&framework).ok();
}

#[test]
fn write_skill_registry_skips_non_skill_dirs() {
    let project = temp_project("skip");
    let framework = temp_framework("skip");
    let env = test_environment(&project);
    init_fake_git_remote(&framework);

    // Create a valid skill in framework scope.
    make_skill(
        &framework.join("skills/sddk-verify"),
        "sddk-verify",
        "Verify SDD implementation",
    );
    // A directory without SKILL.md should be skipped.
    std::fs::create_dir_all(framework.join("skills/sddk-incomplete")).unwrap();
    // A regular file in skills/ should be skipped.
    std::fs::write(framework.join("skills/README.md"), "# readme\n").unwrap();

    let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
    assert_eq!(
        count, 1,
        "only sddk-verify with SKILL.md should be included"
    );

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&framework).ok();
}

#[test]
fn write_skill_registry_project_skips_framework_when_empty() {
    // Skills from both scopes — no dedup needed, both appear.
    let project = temp_project("proj-only");
    let framework = temp_framework("proj-only");
    let env = test_environment(&project);
    init_fake_git_remote(&framework);

    // Project skill must be in a project-scope dir under project_root (framework here).
    make_skill(
        &framework.join(".opencode/skills/sddk-apply"),
        "sddk-apply",
        "Apply",
    );
    // Framework skill in the framework scope.
    make_skill(
        &framework.join("skills/sddk-design"),
        "sddk-design",
        "Design",
    );

    let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
    assert_eq!(count, 2, "skills from both scopes should appear");

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&framework).ok();
}

#[test]
fn write_skill_registry_project_wins_over_framework() {
    // Same skill name in project and framework scopes; project must win.
    let project = temp_project("precedence");
    let framework = temp_framework("precedence");
    let env = test_environment(&project);
    init_fake_git_remote(&framework);

    // Skill in framework scope at `framework/skills/sddk-apply`.
    make_skill(
        &framework.join("skills/sddk-apply"),
        "sddk-apply",
        "Framework apply skill",
    );
    // Same name in project scope — must be under project_root (framework here)
    // at a recognized project-scope path so project scope finds it first.
    make_skill(
        &framework.join(".opencode/skills/sddk-apply"),
        "sddk-apply",
        "Project-level apply skill",
    );

    let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
    assert_eq!(count, 1, "only one sddk-apply should appear (project wins)");

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&framework).ok();
}

#[test]
fn write_skill_registry_user_wins_over_framework() {
    // Same skill name in user and framework scopes; user must win.
    let project = temp_project("user-precedence");
    let framework = temp_framework("user-precedence");
    init_fake_git_remote(&framework);

    // Skill in framework scope.
    make_skill(
        &framework.join("skills/sddk-design"),
        "sddk-design",
        "Framework design skill",
    );
    // Same name in user scope should override.
    let fake_home = temp_project("user-home");
    make_skill(
        &fake_home.join(".config/opencode/skills/sddk-design"),
        "sddk-design",
        "User design skill",
    );

    let mut env_with_home = test_environment(&project);
    env_with_home.home = Some(fake_home.clone());
    let (_, count) = write_skill_registry(&env_with_home, &framework, &framework).unwrap();
    assert_eq!(count, 1, "only one sddk-design should appear (user wins)");

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&framework).ok();
    std::fs::remove_dir_all(&fake_home).ok();
}

#[test]
fn write_skill_registry_empty_when_no_skills() {
    let project = temp_project("empty");
    let framework = temp_framework("empty");
    let env = test_environment(&project);
    init_fake_git_remote(&framework);

    let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
    assert_eq!(count, 0, "no skills means empty registry");

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&framework).ok();
}

#[test]
fn write_skill_registry_deterministic_p_id() {
    // Same project must produce the same p-* ID across calls.
    let project = temp_project("det-id");
    let framework = temp_framework("det-id");
    let env = test_environment(&project);
    init_fake_git_remote(&framework);

    make_skill(
        &framework.join("skills/sddk-apply"),
        "sddk-apply",
        "Test skill",
    );

    let (path1, _) = write_skill_registry(&env, &framework, &framework).unwrap();
    let (path2, _) = write_skill_registry(&env, &framework, &framework).unwrap();

    // The registry path contains the project ID; both calls must write to same location.
    assert_eq!(
        path1, path2,
        "same project_root must produce same registry path (same p-* ID)"
    );

    std::fs::remove_dir_all(&project).ok();
    std::fs::remove_dir_all(&framework).ok();
}
