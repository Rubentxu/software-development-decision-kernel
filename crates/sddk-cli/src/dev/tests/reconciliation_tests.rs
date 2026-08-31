//! Tests for `dev link` — extracted to dev/tests/ to keep link.rs below LOC ceiling.

use crate::dev::link::{link_file, prune_editor};

fn temp_tree(tag: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sddk-link-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn prune_removes_framework_deprecated_but_keeps_foreign() {
    let root = temp_tree("root");
    let editor = temp_tree("editor");
    // Framework source: one agent + one skill.
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::write(
        root.join("agents/orchestrator.md"),
        "---\nname: orchestrator\n---\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("skills")).unwrap();
    std::fs::create_dir_all(root.join("skills/sddk-apply")).unwrap();
    std::fs::write(root.join("skills/sddk-apply/SKILL.md"), "# apply\n").unwrap();

    // Editor state: broken framework link, orphan namespaced skill,
    // stale backup, and a foreign (arch-stack) skill that must survive.
    std::fs::create_dir_all(editor.join("agents")).unwrap();
    std::fs::create_dir_all(editor.join("skills")).unwrap();
    std::fs::create_dir_all(editor.join("workflows")).unwrap();
    std::os::unix::fs::symlink(
        "/nonexistent/sddk-deprecated.md",
        editor.join("agents/sddk-deprecated.md"),
    )
    .unwrap();
    std::fs::create_dir_all(editor.join("skills/sddk-continue-options")).unwrap();
    std::fs::write(
        editor.join("skills/sddk-continue-options/SKILL.md"),
        "# orphan\n",
    )
    .unwrap();
    std::fs::write(editor.join("workflows/sddk-a-full.sddk-stale"), "stale\n").unwrap();
    std::fs::create_dir_all(editor.join("skills/architecture-discovery")).unwrap();
    std::fs::write(
        editor.join("skills/architecture-discovery/SKILL.md"),
        "# foreign\n",
    )
    .unwrap();

    let pruned = prune_editor(&root, &editor);
    // 1 broken agent + 1 orphan skill + 1 stale workflow = 3.
    assert_eq!(pruned, 3);
    assert!(!editor.join("agents/sddk-deprecated.md").exists());
    assert!(!editor.join("skills/sddk-continue-options").exists());
    assert!(!editor.join("workflows/sddk-a-full.sddk-stale").exists());
    // Foreign surface untouched.
    assert!(
        editor
            .join("skills/architecture-discovery/SKILL.md")
            .exists()
    );

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&editor).ok();
}

#[test]
fn link_file_is_idempotent_when_target_matches() {
    let dir = temp_tree("link");
    let source = dir.join("source.md");
    let target = dir.join("target.md");
    std::fs::write(&source, "content").unwrap();
    let mut stale = 0usize;
    link_file(&source, &target, &mut stale).unwrap();
    let mtime1 = std::fs::metadata(&target).unwrap().modified().unwrap();
    link_file(&source, &target, &mut stale).unwrap();
    let mtime2 = std::fs::metadata(&target).unwrap().modified().unwrap();
    assert_eq!(mtime1, mtime2, "correct symlink must not be recreated");
    assert_eq!(stale, 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn link_file_replaces_stale_copy_with_backup() {
    let dir = temp_tree("stale");
    let source = dir.join("source.md");
    let target = dir.join("target.md");
    std::fs::write(&source, "new").unwrap();
    std::fs::write(&target, "old copy").unwrap();
    let mut stale = 0usize;
    link_file(&source, &target, &mut stale).unwrap();
    assert_eq!(stale, 1);
    assert!(dir.join("target.sddk-stale").exists());
    assert!(
        std::fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    std::fs::remove_dir_all(&dir).ok();
}
