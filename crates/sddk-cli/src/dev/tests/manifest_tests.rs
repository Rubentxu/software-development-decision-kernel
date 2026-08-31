//! Tests for `dev manifest` — extracted to dev/tests/ to keep manifest.rs below LOC ceiling.

use crate::dev::install::run_dev_install;
use crate::dev::manifest::{MANIFEST_FILE, manifest_entries, verify_manifest, write_manifest};
use crate::dev::update::update_bundle;
use crate::dev::{InstallArgs, LinkEditor, OutputFormat, UpdateArgs};
use sddk_testkit::TestRepository;
use sha2::{Digest, Sha256};

fn temp_root(tag: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sddk-manifest-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn manifest_generates_and_verifies() {
    let root = temp_root("gen");
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::write(root.join("agents/a.md"), "content-a").unwrap();
    std::fs::create_dir_all(root.join("skills/sddk-x")).unwrap();
    std::fs::write(root.join("skills/sddk-x/SKILL.md"), "content-x").unwrap();

    let count = write_manifest(&root).unwrap();
    assert_eq!(count, 2);
    assert!(root.join(MANIFEST_FILE).is_file());
    let mismatches = verify_manifest(&root).unwrap();
    assert!(
        mismatches.is_empty(),
        "intact tree must verify: {mismatches:?}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn manifest_detects_tampering() {
    let root = temp_root("tamper");
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::write(root.join("agents/a.md"), "content-a").unwrap();
    write_manifest(&root).unwrap();
    // Tamper after manifest generation.
    std::fs::write(root.join("agents/a.md"), "content-TAMPERED").unwrap();
    let mismatches = verify_manifest(&root).unwrap();
    assert_eq!(mismatches.len(), 1);
    assert!(mismatches[0].contains("agents/a.md"));
    assert!(mismatches[0].contains("hash mismatch"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn manifest_detects_missing_file() {
    let root = temp_root("missing");
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::write(root.join("agents/a.md"), "content-a").unwrap();
    write_manifest(&root).unwrap();
    std::fs::remove_file(root.join("agents/a.md")).unwrap();
    let mismatches = verify_manifest(&root).unwrap();
    assert_eq!(mismatches.len(), 1);
    assert!(mismatches[0].contains("missing"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn install_fails_on_manifest_mismatch_and_leaves_prefix_clean() {
    // Source: a bundle with MANIFEST_SURFACES.
    let source = temp_root("mismatch-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content-a").unwrap();
    std::fs::create_dir_all(source.join("skills/sddk-x")).unwrap();
    std::fs::write(source.join("skills/sddk-x/SKILL.md"), "skill content").unwrap();
    write_manifest(&source).unwrap();
    // Tamper: change a file after manifest was generated.
    std::fs::write(source.join("agents/a.md"), "TAMPERED").unwrap();

    // Prefix: empty temp dir.
    let prefix = temp_root("mismatch-prefix");

    let args = InstallArgs {
        prefix: prefix.clone(),
        channel: "dev".to_owned(),
        timestamp: None,
        commit: None,
        source: Some(source.clone()),
        release_receipt: None,
        format: OutputFormat::Json,
    };
    let result = run_dev_install(args);
    assert!(
        result.status != 0,
        "install should fail on manifest mismatch, got status={} stderr={}",
        result.status,
        result.stderr
    );
    // FAIL-CLOSED: nothing must be written to prefix when manifest verification
    // fails — not binary, not surfaces. A tampered source corrupts nothing.
    let has_bin = prefix.join("bin/sddk").exists();
    let has_agents = prefix.join("agents").exists();
    let has_skills = prefix.join("skills").exists();
    assert!(
        !has_bin && !has_agents && !has_skills,
        "fail-closed: nothing must be written on manifest mismatch; \
         bin={has_bin}, agents={has_agents}, skills={has_skills}"
    );
    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&prefix).ok();
}

fn release_bundle(source: &std::path::Path, version: &str) -> (std::path::PathBuf, UpdateArgs) {
    let releases = temp_root("update-releases");
    let release_dir = releases.join("download").join(version);
    std::fs::create_dir_all(&release_dir).unwrap();
    let bundle = release_dir.join("software-development-decision-kernel.tar.gz");
    let status = std::process::Command::new("tar")
        .args(["czf"])
        .arg(&bundle)
        .args(["-C"])
        .arg(source)
        .arg(".")
        .status()
        .unwrap();
    assert!(status.success());
    let checksum = crate::dev::common::sha256_hex(&bundle).unwrap();
    std::fs::write(
        release_dir.join("software-development-decision-kernel.tar.gz.sha256"),
        format!("{checksum}  software-development-decision-kernel.tar.gz\n"),
    )
    .unwrap();
    let args = UpdateArgs {
        root: std::path::PathBuf::new(),
        version: Some(version.to_owned()),
        repo: "unused/for-file-url".to_owned(),
        base_url: Some(format!("file://{}", releases.display())),
        editor: LinkEditor::All,
        format: OutputFormat::Text,
    };
    (releases, args)
}

#[cfg(unix)]
#[test]
fn update_rejects_mismatch_before_touching_target() {
    let source = temp_root("update-mismatch-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "original").unwrap();
    write_manifest(&source).unwrap();
    std::fs::write(source.join("agents/a.md"), "tampered").unwrap();
    let (_releases, args) = release_bundle(&source, "v-test-mismatch");
    let target = temp_root("update-mismatch-target");
    std::fs::write(target.join("sentinel"), "keep").unwrap();

    let error = update_bundle(&target, &args).unwrap_err().to_string();
    assert!(error.contains("content verification FAILED"), "{error}");
    assert_eq!(
        std::fs::read_to_string(target.join("sentinel")).unwrap(),
        "keep"
    );
    assert!(!target.join("agents").exists());
}

#[cfg(unix)]
#[test]
fn update_requires_manifest_before_touching_target() {
    let source = temp_root("update-no-manifest-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content").unwrap();
    let (_releases, args) = release_bundle(&source, "v-test-no-manifest");
    let target = temp_root("update-no-manifest-target");
    std::fs::write(target.join("sentinel"), "keep").unwrap();

    let error = update_bundle(&target, &args).unwrap_err().to_string();
    assert!(error.contains("MANIFEST.sha256"), "{error}");
    assert_eq!(
        std::fs::read_to_string(target.join("sentinel")).unwrap(),
        "keep"
    );
    assert!(!target.join("agents").exists());
}

#[cfg(unix)]
#[test]
fn update_installs_verified_staged_bundle() {
    let source = temp_root("update-valid-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content").unwrap();
    write_manifest(&source).unwrap();
    let (_releases, args) = release_bundle(&source, "v-test-valid");
    let target = temp_root("update-valid-target");

    update_bundle(&target, &args).unwrap();

    assert_eq!(
        std::fs::read_to_string(target.join("agents/a.md")).unwrap(),
        "content"
    );
    assert!(target.join(MANIFEST_FILE).is_file());
    assert!(verify_manifest(&target).unwrap().is_empty());
}

#[test]
fn manifest_inside_worktree_excludes_untracked() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    repo.write("agents/tracked.md", "# Tracked\n").unwrap();
    repo.write("agents/untracked.md", "# Untracked\n").unwrap();
    repo.git(&["add", "agents/tracked.md"]).unwrap();
    repo.git(&["commit", "-q", "-m", "tracked"]).unwrap();
    let entries = manifest_entries(repo.path()).unwrap();
    assert!(entries.iter().any(|(p, _)| p.contains("tracked.md")));
    assert!(!entries.iter().any(|(p, _)| p.contains("untracked.md")));
}

#[test]
fn manifest_inside_worktree_fails_on_missing_tracked() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    repo.write("agents/to-delete.md", "# Delete me\n").unwrap();
    repo.git(&["add", "agents/to-delete.md"]).unwrap();
    repo.git(&["commit", "-q", "-m", "add"]).unwrap();
    std::fs::remove_file(repo.path().join("agents/to-delete.md")).unwrap();
    let result = manifest_entries(repo.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("to-delete"));
}

#[test]
fn manifest_inside_worktree_hashes_current_bytes() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    repo.write("agents/modified.md", "original").unwrap();
    repo.git(&["add", "agents/modified.md"]).unwrap();
    repo.git(&["commit", "-q", "-m", "initial"]).unwrap();
    repo.write("agents/modified.md", "modified").unwrap();
    let entries = manifest_entries(repo.path()).unwrap();
    let entry = entries
        .iter()
        .find(|(p, _)| p.contains("modified.md"))
        .unwrap();
    let expected = format!("{:x}", Sha256::digest(b"modified"));
    assert_eq!(entry.1, expected);
}

#[cfg(unix)]
#[test]
fn manifest_inside_worktree_excludes_symlinks() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let agents = repo.path().join("agents");
    std::fs::create_dir_all(&agents).unwrap();
    std::fs::write(agents.join("real.md"), "# Real\n").unwrap();
    std::os::unix::fs::symlink("real.md", agents.join("link.md")).ok();
    repo.git(&["add", "agents/real.md", "agents/link.md"])
        .unwrap();
    repo.git(&["commit", "-q", "-m", "add"]).unwrap();
    let entries = manifest_entries(repo.path()).unwrap();
    assert!(entries.iter().any(|(p, _)| p.contains("real.md")));
    assert!(!entries.iter().any(|(p, _)| p.contains("link.md")));
}

#[test]
fn manifest_fails_closed_for_corrupt_git_marker() {
    let root = temp_root("corrupt-git");
    std::fs::create_dir_all(root.join("agents")).unwrap();
    std::fs::write(root.join("agents/a.md"), "content").unwrap();
    std::fs::write(root.join(".git"), "gitdir: /missing").unwrap();

    assert!(write_manifest(&root).is_err());
    assert!(!root.join(MANIFEST_FILE).exists());
    std::fs::remove_dir_all(root).ok();
}

#[cfg(unix)]
#[test]
fn manifest_fails_closed_for_non_utf8_tracked_path() {
    use std::os::unix::ffi::OsStringExt;

    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    std::fs::create_dir_all(repo.path().join("agents")).unwrap();
    let name = std::ffi::OsString::from_vec(vec![0xff, b'.', b'm', b'd']);
    std::fs::write(repo.path().join("agents").join(name), "content").unwrap();
    repo.git(&["add", "."]).unwrap();
    repo.git(&["commit", "-q", "-m", "non-utf8"]).unwrap();

    let error = write_manifest(repo.path()).unwrap_err().to_string();
    assert!(error.contains("UTF-8"), "{error}");
    assert!(!repo.path().join(MANIFEST_FILE).exists());
}

#[cfg(unix)]
#[test]
fn manifest_ignores_non_utf8_tracked_path_outside_surfaces() {
    use std::os::unix::ffi::OsStringExt;

    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    std::fs::create_dir_all(repo.path().join("agents")).unwrap();
    repo.write("agents/a.md", "content").unwrap();
    std::fs::create_dir_all(repo.path().join("docs")).unwrap();
    let name = std::ffi::OsString::from_vec(vec![0xff, b'.', b'm', b'd']);
    std::fs::write(repo.path().join("docs").join(name), "ignored").unwrap();
    repo.git(&["add", "."]).unwrap();
    repo.git(&["commit", "-q", "-m", "outside surface"])
        .unwrap();

    assert_eq!(write_manifest(repo.path()).unwrap(), 1);
    assert!(verify_manifest(repo.path()).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn manifest_round_trips_special_utf8_paths() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    std::fs::create_dir_all(repo.path().join("agents")).unwrap();
    std::fs::write(repo.path().join("agents/line\nbreak.md"), "content").unwrap();
    std::fs::write(repo.path().join("agents/back\\slash.md"), "content").unwrap();
    std::fs::write(repo.path().join("agents/trailing-space "), "content").unwrap();
    repo.git(&["add", "."]).unwrap();
    repo.git(&["commit", "-q", "-m", "special paths"]).unwrap();

    assert_eq!(write_manifest(repo.path()).unwrap(), 3);
    let manifest = std::fs::read_to_string(repo.path().join(MANIFEST_FILE)).unwrap();
    assert!(manifest.contains("agents/line\\nbreak.md"), "{manifest:?}");
    assert!(manifest.contains("agents/back\\\\slash.md"), "{manifest:?}");
    assert!(verify_manifest(repo.path()).unwrap().is_empty());
}

// B1 — BundleCoverage: agent-models.yaml rides the assets surface — manifest
// hash-checks it and `dev install` ships it (manifest integrity covers it).
#[test]
fn manifest_covers_agent_models_yaml_and_install_ships_it() {
    let source = temp_root("agent-models-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content-a").unwrap();
    std::fs::create_dir_all(source.join("assets")).unwrap();
    std::fs::write(
        source.join("assets/agent-models.yaml"),
        "tiers: {}\nagents: {}\n",
    )
    .unwrap();
    write_manifest(&source).unwrap();
    let manifest = std::fs::read_to_string(source.join(MANIFEST_FILE)).unwrap();
    assert!(
        manifest.contains("assets/agent-models.yaml"),
        "manifest must hash the canonical config: {manifest}"
    );

    let prefix = temp_root("agent-models-prefix");
    let args = InstallArgs {
        prefix: prefix.clone(),
        channel: "dev".to_owned(),
        timestamp: None,
        commit: None,
        source: Some(source.clone()),
        release_receipt: None,
        format: OutputFormat::Json,
    };
    let result = run_dev_install(args);
    assert_eq!(result.status, 0, "{}", result.stderr);
    assert!(
        prefix.join("assets/agent-models.yaml").is_file(),
        "install must ship the canonical config under assets/"
    );
    assert!(
        verify_manifest(&prefix).unwrap().is_empty(),
        "installed tree must verify against its manifest"
    );
    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&prefix).ok();
}
