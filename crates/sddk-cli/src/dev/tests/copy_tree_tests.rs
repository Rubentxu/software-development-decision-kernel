//! Tests for `dev common::copy_tree` — extracted to dev/tests/ to keep common.rs below LOC ceiling.

use crate::dev::common::{CopyMode, copy_tree};

fn temp_root(tag: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sddk-copytree-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `dead_code` allow: retained as API surface for future test helpers;
/// tracked for cleanup in phase2-hygiene-baseline.
#[allow(dead_code)]
fn sibling_paths(target: &std::path::Path) -> Vec<std::path::PathBuf> {
    let parent = target.parent().unwrap_or(target);
    let stem = target.file_name().unwrap_or_default().to_string_lossy();
    std::fs::read_dir(parent)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with(&*stem) && !p.exists()
                || (name.starts_with(&format!("{}.tmp-", stem)) && p.exists())
                || (name.starts_with(&format!("{}.old-", stem)) && p.exists())
        })
        .collect()
}

/// Check there are no leftover tmp- or old- sibling files.
fn assert_no_residue(target: &std::path::Path) {
    let parent = target.parent().unwrap_or(target);
    let stem = target.file_name().unwrap_or_default().to_string_lossy();
    let leftovers: Vec<_> = std::fs::read_dir(parent)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            (name.starts_with(&format!("{}.tmp-", stem))
                || name.starts_with(&format!("{}.old-", stem)))
                && p.exists()
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "found residue sibling files: {leftovers:?}"
    );
}

#[test]
fn copy_tree_always_copies_full_tree_without_residue() {
    // Source tree: {a.md, skills/x/SKILL.md}
    let source = temp_root("always-src");
    std::fs::create_dir_all(source.join("skills/x")).unwrap();
    std::fs::write(source.join("a.md"), "content-a").unwrap();
    std::fs::write(source.join("skills/x/SKILL.md"), "skill content").unwrap();

    let target = temp_root("always-target");

    copy_tree(&source, &target, CopyMode::Always).unwrap();

    // Full tree copied
    assert_eq!(
        std::fs::read_to_string(target.join("a.md")).unwrap(),
        "content-a"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("skills/x/SKILL.md")).unwrap(),
        "skill content"
    );
    // No residue siblings
    assert_no_residue(&target);

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&target).ok();
}

#[test]
fn copy_tree_always_swaps_existing_target() {
    // Pre-existing target with old.md
    let target = temp_root("always-swap-target");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("old.md"), "old content").unwrap();

    // Source with only new.md
    let source = temp_root("always-swap-source");
    std::fs::write(source.join("new.md"), "new content").unwrap();

    copy_tree(&source, &target, CopyMode::Always).unwrap();

    // new.md is present, old.md is gone
    assert_eq!(
        std::fs::read_to_string(target.join("new.md")).unwrap(),
        "new content"
    );
    assert!(
        !target.join("old.md").exists(),
        "old.md should be removed after swap"
    );
    // No residue
    assert_no_residue(&target);

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&target).ok();
}

#[test]
fn copy_tree_if_changed_preserves_identical_files() {
    // Target pre-populated with a.md (identical) and b.md (different)
    let target = temp_root("ifchanged-target");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("a.md"), "same-content").unwrap();
    let target_b = target.join("b.md");
    std::fs::write(&target_b, "old-b").unwrap();
    let target_a = target.join("a.md");
    let mtime_before_a = std::fs::metadata(&target_a).unwrap().modified().unwrap();

    // Source: a.md identical, b.md different
    let source = temp_root("ifchanged-source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("a.md"), "same-content").unwrap();
    std::fs::write(source.join("b.md"), "new-b").unwrap();

    // Small delay to ensure mtime would differ if file were rewritten
    std::thread::sleep(std::time::Duration::from_millis(20));

    copy_tree(&source, &target, CopyMode::IfChanged).unwrap();

    // a.md must NOT be rewritten — mtime preserved
    let mtime_after_a = std::fs::metadata(&target_a).unwrap().modified().unwrap();
    assert_eq!(
        mtime_before_a, mtime_after_a,
        "mtime of unchanged file should be preserved"
    );

    // b.md must be updated
    assert_eq!(std::fs::read_to_string(&target_b).unwrap(), "new-b");

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&target).ok();
}

#[cfg(unix)]
#[test]
fn copy_tree_failure_leaves_target_intact() {
    use std::os::unix::fs::PermissionsExt;

    // Target pre-populated with keep.md
    let target = temp_root("failure-target");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("keep.md"), "keep content").unwrap();

    // Source with x.md that we will try to copy
    let source = temp_root("failure-source");
    std::fs::write(source.join("x.md"), "x content").unwrap();

    // Make target's parent read-only so staging creation inside it fails
    let parent = target.parent().unwrap_or(&target);
    let original_mode = std::fs::metadata(parent).unwrap().permissions().mode();

    // Attempt to make parent read-only; if the filesystem doesn't support
    // chmod (e.g. some networkFS, WSL shared folders) or if we are root
    // (where chmod has no effect), skip gracefully.
    if std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o555)).is_err() {
        eprintln!(
            "SKIP copy_tree_failure_leaves_target_intact: \
             cannot set read-only permissions on this filesystem or as root"
        );
        std::fs::remove_dir_all(&source).ok();
        std::fs::remove_dir_all(&target).ok();
        return;
    }

    // Verify that chmod actually took effect by trying to write in parent.
    // If this succeeds despite chmod, we're root and must skip.
    let test_file = parent.join(".sddk-write-guard");
    if std::fs::write(&test_file, "test").is_ok() {
        let _ = std::fs::remove_file(&test_file);
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(original_mode));
        eprintln!(
            "SKIP copy_tree_failure_leaves_target_intact: \
             running as root, chmod 555 does not block writes"
        );
        std::fs::remove_dir_all(&source).ok();
        std::fs::remove_dir_all(&target).ok();
        return;
    }
    let _ = std::fs::remove_file(&test_file);

    let result = copy_tree(&source, &target, CopyMode::Always);

    // Restore permissions before assertions so we can clean up
    let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(original_mode));

    // Operation must fail
    assert!(
        result.is_err(),
        "copy_tree Always should fail when parent is read-only"
    );

    // target must be intact
    assert_eq!(
        std::fs::read_to_string(target.join("keep.md")).unwrap(),
        "keep content",
        "target keep.md must be untouched"
    );

    // No residue siblings
    assert_no_residue(&target);

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&target).ok();
}
