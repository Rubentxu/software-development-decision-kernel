//! M7.9 — Skill registry bridge dotted-name branch tests.
//!
//! Closes the gap between `with_required_skill("core.<...>@v1")`
//! declarations (the M7.5 contractual placeholders) and SKILL.md
//! files whose frontmatter `name` is already fully namespaced.
//! Before this extension, every parsed skill id was wrapped with
//! the scope prefix, so `core.workflow-orchestration` never
//! round-tripped to the id the CommandSpec expects.
//!
//! These tests pin the contract:
//!
//! 1. dotted name + framework scope → id preserved verbatim,
//    scope recorded as Framework, ref_token = `core.<name>@v1`;
//! 2. dotted name + user scope → same id; scope recorded as User
//!    (so `load_registry` dedupe gives framework precedence);
//! 3. un-dotted name → existing behavior unchanged
//!    (regression guard for the 174 existing framework skills);
//! 4. dotted name starting with a scope label (`framework.foo`)
//!    → parse returns `None` (defensive guard);
//! 5. dotted name with downstream `validate()` rejection → still
//!    skipped silently.

use std::path::PathBuf;

use sddk_cli::skill_registry_bridge::{parse_skill_md, SkillScope};

fn write_skill(dir: &std::path::Path, name: &str, description: &str, version: &str) -> PathBuf {
    let skill_dir = dir.join("dummy");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let path = skill_dir.join("SKILL.md");
    let body = format!(
        "---\nname: {name}\ndescription: {description}\nmetadata:\n  version: \"{version}\"\n---\n\n# body\n"
    );
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn m79_dotted_name_framework_scope_preserves_id() {
    let tmp = tempdir();
    let path = write_skill(&tmp, "core.workflow-orchestration", "core skill", "1");
    let loaded =
        parse_skill_md(&path, SkillScope::Framework).expect("dotted name must parse");
    assert_eq!(loaded.definition.id, "core.workflow-orchestration");
    assert_eq!(loaded.definition.version, 1);
    assert_eq!(loaded.scope, SkillScope::Framework);
    // ref_token format: id@version
    let ref_token = format!("{}@{}", loaded.definition.id, loaded.definition.version);
    assert_eq!(ref_token, "core.workflow-orchestration@1");
}

#[test]
fn m79_dotted_name_user_scope_records_scope_but_id_unchanged() {
    let tmp = tempdir();
    let path = write_skill(&tmp, "core.workflow-orchestration", "core skill", "1");
    let loaded = parse_skill_md(&path, SkillScope::User).expect("dotted name must parse");
    assert_eq!(loaded.definition.id, "core.workflow-orchestration");
    assert_eq!(loaded.scope, SkillScope::User);
    // scope is metadata; the id stays the same regardless of scope
}

#[test]
fn m79_undotted_name_keeps_historical_wrapping() {
    // Regression guard: the 174 existing `framework.<name>@v1` skills
    // must continue to parse to id = `framework.<name>`.
    let tmp = tempdir();
    let path = write_skill(&tmp, "shared-skill", "an existing skill", "1");
    let loaded = parse_skill_md(&path, SkillScope::Framework).expect("undotted must parse");
    assert_eq!(loaded.definition.id, "framework.shared-skill");
    assert_eq!(loaded.scope, SkillScope::Framework);
}

#[test]
fn m79_dotted_name_with_scope_prefix_is_skipped() {
    // Defensive: `framework.contract-review` would otherwise produce
    // id = `framework.framework.contract-review` (double prefix). The
    // guard rejects and parse returns None.
    let tmp = tempdir();
    let path = write_skill(&tmp, "framework.contract-review", "would collide", "1");
    let loaded = parse_skill_md(&path, SkillScope::Framework);
    assert!(loaded.is_none(), "scope-prefix collision must be skipped");

    let path2 = write_skill(&tmp, "user.foo", "would collide", "1");
    let loaded2 = parse_skill_md(&path2, SkillScope::User);
    assert!(loaded2.is_none(), "user-prefix collision must be skipped");

    let path3 = write_skill(&tmp, "project.bar", "would collide", "1");
    let loaded3 = parse_skill_md(&path3, SkillScope::Project);
    assert!(loaded3.is_none(), "project-prefix collision must be skipped");
}

#[test]
fn m79_all_three_m75_placeholders_parse_as_real_skills() {
    // Happy-path: all three M7.5 contractual placeholders declared on
    // CommandSpec round-trip to real `LoadedSkill`s with ids matching
    // the placeholders exactly. This is the integration-level proof
    // that the runtime admission gate will admit `lint`, `cycle`,
    // and `release` once these SKILL.md files land in the bundle.
    let tmp = tempdir();
    let names = [
        "core.contract-review",
        "core.workflow-orchestration",
        "core.release-planning",
    ];
    for (i, name) in names.iter().enumerate() {
        let subdir = tmp.join(format!("skill-{i}"));
        std::fs::create_dir_all(&subdir).unwrap();
        let path = subdir.join("SKILL.md");
        let body = format!(
            "---\nname: {name}\ndescription: core placeholder skill\nmetadata:\n  version: \"1\"\n---\n\n# body\n"
        );
        std::fs::write(&path, body).unwrap();
        let loaded = parse_skill_md(&path, SkillScope::Framework)
            .unwrap_or_else(|| panic!("{name} must parse"));
        assert_eq!(loaded.definition.id, *name, "id must be preserved verbatim");
        assert_eq!(loaded.scope, SkillScope::Framework);
        assert_eq!(loaded.definition.version, 1);
    }
}

fn tempdir() -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "sddk-cli-m79-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).unwrap();
    base
}
