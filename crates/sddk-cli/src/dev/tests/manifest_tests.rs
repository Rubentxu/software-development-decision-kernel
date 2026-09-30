//! Tests for `dev manifest` — extracted to dev/tests/ to keep manifest.rs below LOC ceiling.

use crate::dev::install::run_dev_install;
use crate::dev::manifest::{MANIFEST_FILE, manifest_entries, verify_manifest, write_manifest};
use crate::dev::update::update_bundle;
use crate::dev::{InstallArgs, LinkEditor, OutputFormat, UpdateArgs};

/// Run an update against a local fixture bundle that carries NO signature.
///
/// The fixtures here are tarballs built in a tempdir; they have never been
/// signed, and signing them would mean reaching Sigstore from a unit test —
/// a network dependency and a device-flow prompt in CI. `sddk dev update`
/// deliberately refuses an unsigned bundle (see INC-AUDIT-S14), so these
/// tests must opt out explicitly and say so.
///
/// The opt-in is passed as a parameter rather than set in the environment
/// on purpose. The crate is `#![forbid(unsafe_code)]`, and an env var would
/// be process-global state leaking into unrelated tests. Passing it
/// explicitly also keeps the fixture honest: the flag is the same switch an
/// operator would flip from the command line.
fn update_bundle_unsigned_fixture(
    target: &std::path::Path,
    args: &UpdateArgs,
) -> anyhow::Result<String> {
    let mut args = args.clone();
    args.allow_unsigned = true;
    update_bundle(target, &args)
}
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
        actor: None,
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
        prune: false,
        keep: None,
        prune_only: false,
        allow_unsigned: false,
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

    let error = update_bundle_unsigned_fixture(&target, &args)
        .unwrap_err()
        .to_string();
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

    let error = update_bundle_unsigned_fixture(&target, &args)
        .unwrap_err()
        .to_string();
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

    update_bundle_unsigned_fixture(&target, &args).unwrap();

    assert_eq!(
        std::fs::read_to_string(target.join("agents/a.md")).unwrap(),
        "content"
    );
    assert!(target.join(MANIFEST_FILE).is_file());
    assert!(verify_manifest(&target).unwrap().is_empty());
}

/// A bundle carrying a BUNDLE.toml with `bundle.version` must install into
/// `framework/<version>/`, NOT into the framework root. Mixing the two
/// layouts is the root cause of the session-33b installer defect: a later
/// install.sh expecting `framework/<version>/` found it empty and every
/// editor symlink broke (69 broken links, all_present false).
#[test]
fn update_installs_versioned_bundle_into_version_dir() {
    let source = temp_root("update-versioned-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content").unwrap();
    write_manifest(&source).unwrap();
    std::fs::write(
        source.join(crate::dev::bundle_manifest::BUNDLE_MANIFEST_FILE),
        "[bundle]\nid = \"sddk-framework\"\nversion = \"9.9.9\"\nschema_version = 2\nbinary_min_version = \"2.0.0\"\nbinary_max_version = \"99.0.0\"\ncompatibility = \">=1.91\"\n",
    )
    .unwrap();
    // The manifest must cover BUNDLE.toml itself (it is a bundle file).
    let (_releases, args) = release_bundle(&source, "v-test-versioned");
    let target = temp_root("update-versioned-target");

    let out = update_bundle_unsigned_fixture(&target, &args).unwrap();

    // The files land in the VERSION dir, not the root.
    assert!(
        target.join("9.9.9/agents/a.md").is_file(),
        "bundle must install into version dir, output was: {out}"
    );
    assert!(!target.join("agents").exists(), "root must stay clean");
    assert!(target.join("9.9.9").join(MANIFEST_FILE).is_file());
    assert!(out.contains("9.9.9"));
    std::fs::remove_dir_all(&target).ok();
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

    // Cycle-46: a coherent install (v2 schema) requires a BUNDLE.toml at the
    // source root declaring the bundle version and binary compatibility.
    // Without it `dev install --source` refuses to write the v2 receipt.
    // Use the current CARGO_PKG_VERSION (rather than a hardcoded string) so
    // the bundle declares it is compatible with the binary that is being
    // built today, regardless of which version that is.
    use crate::dev::bundle_manifest::{ContentsSection, write_bundle_manifest};
    write_bundle_manifest(
        &source,
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION"),
        ContentsSection::default(),
    )
    .unwrap();

    let prefix = temp_root("agent-models-prefix");
    let args = InstallArgs {
        actor: None,
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

/// Cycle-47 D3: a v1 receipt (no schema_version field) under the prefix must
/// be rewritten as v2 when the install source carries a BUNDLE.toml that
/// declares a bundle version compatible with the running binary. The
/// migration is implicit: `run_dev_install` always writes a v2 receipt when
/// --source is provided.
#[test]
fn install_migrates_legacy_v1_receipt_to_v2_when_source_has_bundle_toml() {
    let source = temp_root("v1-migrate-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content-a").unwrap();
    write_manifest(&source).unwrap();
    // Add a BUNDLE.toml declaring compat with the running binary so the
    // install preflight passes.
    use crate::dev::bundle_manifest::{ContentsSection, write_bundle_manifest};
    write_bundle_manifest(
        &source,
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION"),
        ContentsSection::default(),
    )
    .unwrap();

    let prefix = temp_root("v1-migrate-prefix");
    // Pre-plant a v1 receipt (no schema_version, no bundle_version, etc.).
    let v1 = serde_json::json!({
        "version": "0.1.0-pre",
        "commit": "0000000000000000000000000000000000000000",
        "binary_sha256": "sha256:deadbeef",
        "channel": "dev",
        "installed_at": "2026-08-15T00:00:00Z",
        "binary_path": "sddk",
        "bundle": true
    });
    std::fs::write(
        prefix.join("sddk-install.json"),
        serde_json::to_string_pretty(&v1).unwrap(),
    )
    .unwrap();

    let args = InstallArgs {
        actor: None,
        prefix: prefix.clone(),
        channel: "dev".to_owned(),
        timestamp: Some("2026-08-31T00:00:00Z".to_owned()),
        commit: None,
        source: Some(source.clone()),
        release_receipt: None,
        format: OutputFormat::Json,
    };
    let result = run_dev_install(args);
    assert_eq!(
        result.status, 0,
        "install should succeed: {}",
        result.stderr
    );

    // Receipt must now be v2.
    let raw = std::fs::read_to_string(prefix.join("sddk-install.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["schema_version"], 2);
    // INC-DEBT-038 (session-47): --source produce layout FLAT (sin
    // framework/<v>/ ni current); el recibo honesto declara layout="flat" y
    // deja bundle_version a null en vez de enlazar una version que el layout
    // no produce (eso era el bug: el doctor tachaba la coherencia).
    assert_eq!(parsed["layout"], "flat");
    assert!(
        parsed["bundle_version"].is_null(),
        "flat install must not bind bundle_version (INC-DEBT-038)"
    );
    assert!(
        parsed["bundle_sha256"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(parsed["coherence_checked"], true);
    // Old version field is REPLACED with the current install (not preserved);
    // migration is "overwrite with v2 truth", not "merge with v1 legacy".
    assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&prefix).ok();
}

#[test]
fn install_rejects_agent_actor_on_framework_bundle_surface() {
    // ARCH-HEX-001 slice: FrameworkBundle is System-only (ADR-069 §3).
    // An agent: prefixed actor must be rejected fail-closed BEFORE any
    // prefix mutation.
    let prefix = temp_root("auth-agent-prefix");
    let args = InstallArgs {
        actor: Some("agent:orchestrator".into()),
        prefix: prefix.clone(),
        channel: "dev".to_owned(),
        timestamp: None,
        commit: None,
        source: None,
        release_receipt: None,
        format: OutputFormat::Json,
    };
    let result = run_dev_install(args);
    assert_ne!(result.status, 0, "agent actor must be rejected");
    assert!(result.stderr.contains("authority"));
    // No binary was written to the prefix.
    assert!(!prefix.join("bin").join("sddk").exists());
    assert!(!prefix.join("sddk").exists());
}

#[test]
fn install_admits_plain_actor_as_system_on_bundle_surface() {
    // Plain ids infer System per the v1.81.x prefix contract; the gate
    // must admit them and proceed to the install receipt.
    let prefix = temp_root("auth-system-prefix");
    let args = InstallArgs {
        actor: Some("rubentxu".into()),
        prefix: prefix.clone(),
        channel: "dev".to_owned(),
        timestamp: None,
        commit: None,
        source: None,
        release_receipt: None,
        format: OutputFormat::Json,
    };
    let result = run_dev_install(args);
    assert_eq!(result.status, 0, "plain actor is System: {}", result.stderr);
}

/// A bundle WITHOUT BUNDLE.toml (legacy root layout) must keep installing
/// into the root: the fix must not break the legacy layout contract.
#[test]
fn update_legacy_root_layout_bundle_still_installs_at_root() {
    let source = temp_root("update-legacy-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content").unwrap();
    write_manifest(&source).unwrap();
    let (_releases, args) = release_bundle(&source, "v-test-legacy");
    let target = temp_root("update-legacy-target");

    let out = update_bundle_unsigned_fixture(&target, &args).unwrap();

    assert!(target.join("agents/a.md").is_file());
    assert!(!out.contains(" into "));
    std::fs::remove_dir_all(&target).ok();
}

/// REGRESSION (session-34, observed on the real machine against v2.2.23):
/// a legacy root-layout update must NOT destroy sibling content that
/// already lives in the framework root. `copy_tree(CopyMode::Always)` with
/// target == framework ROOT renames the whole root (version dirs from a
/// previous install.sh, the `current` symlink) to `.old-<pid>` and DELETES
/// it in the atomic swap. Observed: `dev update --version v2.2.23` removed
/// `2.2.21/`, `2.2.22/` and `current` in one shot. A root-layout update
/// must merge into the existing root instead.
#[test]
fn update_legacy_root_layout_preserves_existing_root_content() {
    let source = temp_root("update-legacy-preserve-source");
    std::fs::create_dir_all(source.join("agents")).unwrap();
    std::fs::write(source.join("agents/a.md"), "content").unwrap();
    write_manifest(&source).unwrap();
    let (_releases, args) = release_bundle(&source, "v-test-legacy2");
    let target = temp_root("update-legacy-preserve-target");

    // Simulate a machine where install.sh already created a versioned
    // layout plus the `current` symlink, and the incoming bundle is a
    // legacy root-layout tarball (no BUNDLE.toml).
    std::fs::create_dir_all(target.join("2.2.22/agents")).unwrap();
    std::fs::write(target.join("2.2.22/agents/old.md"), "old").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(target.join("2.2.22"), target.join("current")).unwrap();

    update_bundle_unsigned_fixture(&target, &args).unwrap();

    assert!(
        target.join("agents/a.md").is_file(),
        "new bundle content must land in the root"
    );
    assert!(
        target.join("2.2.22/agents/old.md").is_file(),
        "pre-existing version dir must survive the root-layout update"
    );
    #[cfg(unix)]
    assert!(
        target.join("current").exists(),
        "pre-existing current symlink must survive"
    );
    std::fs::remove_dir_all(&target).ok();
}
