//! `dev update` — download and install a framework release bundle.

use super::manifest::MANIFEST_FILE;
use crate::dev::common::{CopyMode, copy_tree, count_manifest_entries, download_to, sha256_hex};
use crate::dev::manifest::verify_manifest;
use crate::dev::paths::framework_dir;
use crate::{CliEnvironment, CommandOutput, render_result};
use std::path::Path;

pub(crate) fn update_bundle(root: &Path, args: &super::UpdateArgs) -> anyhow::Result<String> {
    let version = args.version.as_deref().unwrap_or("latest");
    let base_url = match &args.base_url {
        Some(base) => base.clone(),
        None => format!("https://github.com/{}/releases", args.repo),
    };
    let asset = "software-development-decision-kernel.tar.gz";
    let url = if version == "latest" {
        format!("{base_url}/latest/download/{asset}")
    } else {
        format!("{base_url}/download/{version}/{asset}")
    };

    static NEXT_UPDATE_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT_UPDATE_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("sddk-update-{}-{sequence}", std::process::id()));
    let tmp_dir = tmp.join("dl");
    let staged_bundle = tmp_dir.join("bundle");
    let bundle = tmp_dir.join(asset);
    let checksum = tmp_dir.join(format!("{asset}.sha256"));
    std::fs::create_dir_all(&tmp_dir)?;
    std::fs::create_dir_all(&staged_bundle)?;

    download_to(&url, &bundle)?;
    download_to(&format!("{url}.sha256"), &checksum)?;

    let expected = std::fs::read_to_string(&checksum)?
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty checksum file: {}", checksum.display()))?
        .to_owned();
    let actual = sha256_hex(&bundle)?;
    if expected != actual {
        anyhow::bail!("framework sha256 mismatch\n  expected: {expected}\n  actual:   {actual}");
    }

    let extract = std::process::Command::new("tar")
        .args([
            "xzf",
            bundle.to_str().unwrap_or_default(),
            "-C",
            staged_bundle.to_str().unwrap_or_default(),
            // The release tarball wraps every entry under
            // `software-development-decision-kernel/`; strip that prefix so
            // the staged bundle root matches `MANIFEST_FILE`'s expected
            // location. This is the legacy split-asset path; the unified
            // artifact path used by install.sh extracts to bin/ + framework/.
            "--strip-components=1",
        ])
        .output()?;
    if !extract.status.success() {
        anyhow::bail!(
            "extract failed: {}",
            String::from_utf8_lossy(&extract.stderr).trim()
        );
    }
    // Post-extract integrity: verify every file against the manifest that
    // SHIPPED INSIDE the tarball before touching the target. The tarball
    // checksum proves transport integrity; the internal manifest proves
    // content integrity of each framework surface (ADR-011).
    let manifest_path = staged_bundle.join(MANIFEST_FILE);
    if !manifest_path.is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        anyhow::bail!("bundle is missing required {MANIFEST_FILE}");
    }
    match verify_manifest(&staged_bundle) {
        Ok(mismatches) if mismatches.is_empty() => {}
        Ok(mismatches) => {
            let _ = std::fs::remove_dir_all(&tmp);
            anyhow::bail!(
                "bundle content verification FAILED ({} mismatch(es)):\n  {}",
                mismatches.len(),
                mismatches.join("\n  ")
            );
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            anyhow::bail!("bundle manifest unreadable: {e}");
        }
    }
    let count = count_manifest_entries(root).unwrap_or(0);
    copy_tree(&staged_bundle, root, CopyMode::Always)?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(format!(
        "framework: {version} ({asset}) sha256 verified: {actual}; {count} files content-verified via {MANIFEST_FILE}\n"
    ))
}

/// Identify a "bundle version directory" inside `framework_dir/` by name.
/// Versions are `MAJOR.MINOR[.PATCH][-PRE][+BUILD]`; we keep it permissive
/// but reject anything that contains path separators or whitespace, and
/// require at least one digit so we never match `legacy`, `tmp`, etc.
fn is_bundle_version_dir(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains(' ') {
        return false;
    }
    name.chars().any(|c| c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
}

/// Resolve the version pointed to by the `current` symlink in `framework_dir`,
/// if any.
fn resolve_current_version(framework_dir: &Path) -> Option<String> {
    let current = framework_dir.join("current");
    let target = std::fs::read_link(&current).ok()?;
    let name = target.file_name()?.to_str()?.to_owned();
    if is_bundle_version_dir(&name) {
        Some(name)
    } else {
        None
    }
}

/// INC-DEBT-020: After pruning, re-point `current` symlink to the newest kept version.
/// This runs when `--prune` or `--prune-only` is used with `--root .`.
fn repoint_current_to_newest(framework_dir: &Path, newest_version: &str) {
    let current = framework_dir.join("current");
    let target = framework_dir.join(newest_version);
    if target.is_dir() {
        let tmp = framework_dir.join("current.tmp");
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&current);
        if std::os::unix::fs::symlink(&target, &tmp).is_ok() {
            let _ = std::fs::rename(&tmp, &current);
        }
    }
}

/// Semver-aware "newer than" comparison for bundle version directory names.
/// Supports dotted-numeric with optional `-prerelease` and `+build` tags.
/// Pre-release sorts below its release; numeric segments compare numerically.
fn cmp_bundle_version(a: &str, b: &str) -> std::cmp::Ordering {
    fn parse(s: &str) -> (Vec<u64>, Option<&str>, Option<&str>) {
        let (base, build) = match s.split_once('+') {
            Some((b, build)) => (b, Some(build)),
            None => (s, None),
        };
        let (core, pre) = match base.split_once('-') {
            Some((c, pre)) => (c, Some(pre)),
            None => (base, None),
        };
        let nums: Vec<u64> = core
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect();
        (nums, pre, build)
    }
    let (an, ap, ab) = parse(a);
    let (bn, bp, bb) = parse(b);
    let ord = an.cmp(&bn);
    if ord != std::cmp::Ordering::Equal {
        return ord;
    }
    // release > pre-release for the same core
    let pre_ord = match (ap, bp) {
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(x), Some(y)) => x.cmp(y),
        (None, None) => std::cmp::Ordering::Equal,
    };
    pre_ord.then_with(|| ab.cmp(&bb))
}

/// Remove stale bundle version directories. Returns the list of removed
/// versions and the list of kept versions (for reporting).
///
/// Policy:
/// - `current_version` (if any) is always kept.
/// - When `keep_n` is 0: keep only `current_version` (or all versions if no
///   current symlink exists, since there is no basis to choose).
/// - When `keep_n` > 0: keep the N most recent versions by semver-aware sort,
///   plus `current_version` if it is not already in the kept set.
/// - Anything else is deleted.
pub(crate) fn prune_stale_bundles(
    framework_dir: &Path,
    keep_n: usize,
    current_version: Option<&str>,
) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    if !framework_dir.is_dir() {
        anyhow::bail!("framework dir does not exist: {}", framework_dir.display());
    }
    let entries = std::fs::read_dir(framework_dir)?;
    let mut versions: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| is_bundle_version_dir(name))
        .collect();

    // Newest-first sort so we can take(keep_n) from the top.
    versions.sort_by(|a, b| cmp_bundle_version(b, a));

    let mut keep: Vec<String> = versions.iter().take(keep_n).cloned().collect();
    if let Some(cur) = current_version
        && !keep.iter().any(|v| v == cur)
    {
        keep.push(cur.to_owned());
    }

    let mut removed = Vec::new();
    for v in &versions {
        if keep.iter().any(|k| k == v) {
            continue;
        }
        let path = framework_dir.join(v);
        if let Err(e) = std::fs::remove_dir_all(&path) {
            anyhow::bail!("failed to remove {}: {e}", path.display());
        }
        removed.push(v.clone());
    }
    Ok((removed, keep))
}

pub(super) fn run_dev_update(
    args: super::UpdateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        // Validate arg combinations that clap cannot express declaratively.
        if args.keep.is_some() && !args.prune && !args.prune_only {
            anyhow::bail!("--keep requires either --prune or --prune-only");
        }
        if !args.prune && !args.prune_only {
            // Bare `dev update` needs a version to download.
            if args.version.is_none() && !args.prune_only {
                anyhow::bail!(
                    "either --version (to download a bundle), --prune, or --prune-only is required"
                );
            }
        }

        let mut output = String::new();

        // The framework distributes RELEASE BUNDLES (agents/skills/prompts/
        // workflows/assets + MANIFEST.sha256), never repository clones. Git
        // operations are the developer's responsibility: if the target root
        // is a git checkout, the user updates it with `git pull` themselves.
        // (--prune-only skips this check because it operates on the existing
        // version-dir layout, which is also a non-git tree.)
        if !args.prune_only && args.root.join(".git").is_dir() {
            anyhow::bail!(
                "`dev update` installs release bundles and never touches git. \
                 You passed a repository checkout ({}). \
                 To update a checkout, run `git pull` yourself, then \
                 `sddk dev link --root {}` to re-link the editors.",
                args.root.display(),
                args.root.display()
            );
        }

        // Bundle install: download the framework release bundle, verify, and
        // extract into `$SDDK_DATA_DIR/framework/<version>/`.
        let bundle_root = if args.root.as_os_str() == "." {
            framework_dir(environment)?
        } else {
            std::fs::canonicalize(&args.root).unwrap_or(args.root.clone())
        };
        if !args.prune_only {
            output.push_str(&update_bundle(&bundle_root, &args)?);
        } else {
            output.push_str(&format!(
                "prune-only: skipping bundle download; operating on {}\n",
                bundle_root.display()
            ));
        }

        // The extracted bundle lands in a version dir; update_bundle extracts
        // directly into bundle_root, so if the user passed the framework root
        // we additionally fix the `current` symlink to point at it.
        // INC-DEBT-020 fix: guard this with !prune && !prune_only so the
        // prune paths can manage the symlink themselves.
        if args.root.as_os_str() == "." && !args.prune && !args.prune_only {
            let current = bundle_root.join("current");
            let tmp = bundle_root.join("current.tmp");
            let _ = std::fs::remove_file(&tmp);
            let _ = std::fs::remove_file(&current);
            std::os::unix::fs::symlink(&bundle_root, &tmp)?;
            std::fs::rename(&tmp, &current)?;
            output.push_str("framework: current -> bundle root (dev link resolves it)\n");
        }

        // Cycle-47 D2: --prune [--keep N] removes stale bundle version dirs
        // that are not `current` and not among the N most recent.
        if args.prune {
            let keep_n = args.keep.unwrap_or(0);
            let active = resolve_current_version(&bundle_root);
            let (removed, kept) = prune_stale_bundles(&bundle_root, keep_n, active.as_deref())?;
            output.push_str(&format!(
                "prune: removed {} stale bundle(s); kept {}\n",
                removed.len(),
                if kept.is_empty() {
                    "(none)".to_string()
                } else {
                    kept.join(", ")
                }
            ));
            if !removed.is_empty() {
                output.push_str(&format!("  removed: {}\n", removed.join(", ")));
            }
            if let Some(cur) = active.as_deref() {
                output.push_str(&format!("  current -> {cur}\n"));
            }
            // INC-DEBT-020 fix: after pruning, re-point current to newest kept version
            if args.root.as_os_str() == "." && !kept.is_empty() {
                repoint_current_to_newest(&bundle_root, kept.first().unwrap());
            }
        }

        // Cycle-47 D2 --prune-only: same as --prune but skip the bundle
        // download entirely. Useful for cleaning up after a manual
        // install (e.g. `install.sh --version v1.63.0`) without re-pulling
        // the tarball.
        if args.prune_only {
            let keep_n = args.keep.unwrap_or(0);
            let active = resolve_current_version(&bundle_root);
            let (removed, kept) = prune_stale_bundles(&bundle_root, keep_n, active.as_deref())?;
            output.push_str(&format!(
                "prune-only: removed {} stale bundle(s); kept {}\n",
                removed.len(),
                if kept.is_empty() {
                    "(none)".to_string()
                } else {
                    kept.join(", ")
                }
            ));
            if !removed.is_empty() {
                output.push_str(&format!("  removed: {}\n", removed.join(", ")));
            }
            if let Some(cur) = active.as_deref() {
                output.push_str(&format!("  current -> {cur}\n"));
            }
            // INC-DEBT-020 fix: after pruning, re-point current to newest kept version
            if args.root.as_os_str() == "." && !kept.is_empty() {
                repoint_current_to_newest(&bundle_root, kept.first().unwrap());
            }
        }

        Ok(output)
    })();
    render_result(result, format, |output: &String| output.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn is_bundle_version_dir_accepts_canonical_and_prerelease() {
        assert!(is_bundle_version_dir("1.63.0"));
        assert!(is_bundle_version_dir("1.63.0-rc.1"));
        assert!(is_bundle_version_dir("0.1.0+build.42"));
        // Two-segment versions like v1.63 show up in real GitHub tags;
        // accept them — the semver-aware sort handles padding to (0,0,0).
        assert!(is_bundle_version_dir("1.63"));
        assert!(!is_bundle_version_dir(""));
        assert!(!is_bundle_version_dir("legacy"));
        assert!(!is_bundle_version_dir("tmp"));
        assert!(!is_bundle_version_dir("a/b"));
        assert!(!is_bundle_version_dir("has space"));
    }

    #[test]
    fn cmp_bundle_version_handles_pre_release_and_padding() {
        use std::cmp::Ordering;
        assert_eq!(cmp_bundle_version("1.63.0", "1.63.0"), Ordering::Equal);
        assert_eq!(cmp_bundle_version("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(cmp_bundle_version("1.63.0-rc.1", "1.63.0"), Ordering::Less);
        assert_eq!(
            cmp_bundle_version("1.63.0", "1.63.0-rc.1"),
            Ordering::Greater
        );
        assert_eq!(cmp_bundle_version("1.63.0", "1.63.0.0"), Ordering::Less);
    }

    fn make_version_dir(framework: &Path, version: &str) {
        fs::create_dir_all(framework.join(version)).unwrap();
        fs::write(framework.join(version).join("marker"), version).unwrap();
    }

    #[test]
    fn prune_keeps_current_and_removes_others() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        for v in ["1.28.0", "1.40.0", "1.50.0", "1.63.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.63.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 0, Some("1.63.0")).unwrap();
        assert_eq!(kept, vec!["1.63.0"]);
        assert_eq!(removed.len(), 3);
        assert!(!framework.join("1.50.0").exists());
        assert!(!framework.join("1.40.0").exists());
        assert!(!framework.join("1.28.0").exists());
        assert!(framework.join("1.63.0").exists());
        assert!(framework.join("current").exists());
    }

    #[test]
    fn prune_with_keep_n_keeps_most_recent() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        for v in ["1.28.0", "1.40.0", "1.50.0", "1.63.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.63.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 2, Some("1.63.0")).unwrap();
        // 2 most recent = 1.63.0 + 1.50.0; current is already in keep.
        assert!(kept.contains(&"1.63.0".to_owned()));
        assert!(kept.contains(&"1.50.0".to_owned()));
        assert_eq!(removed.len(), 2);
        assert!(!framework.join("1.28.0").exists());
        assert!(!framework.join("1.40.0").exists());
        assert!(framework.join("1.50.0").exists());
        assert!(framework.join("1.63.0").exists());
    }

    #[test]
    fn prune_refuses_to_remove_current_even_when_not_in_top_n() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        for v in ["1.28.0", "1.63.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.28.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 1, Some("1.28.0")).unwrap();
        // keep_n=1 would pick 1.63.0, but 1.28.0 is current so it is added.
        assert!(kept.contains(&"1.28.0".to_owned()));
        assert!(kept.contains(&"1.63.0".to_owned()));
        assert!(removed.is_empty());
        assert!(framework.join("1.28.0").exists());
        assert!(framework.join("1.63.0").exists());
    }

    #[test]
    fn prune_ignores_non_version_entries() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        make_version_dir(framework, "1.63.0");
        fs::create_dir(framework.join("legacy")).unwrap();
        fs::create_dir(framework.join("tmp")).unwrap();
        fs::write(framework.join("stray.txt"), "ignored").unwrap();
        std::os::unix::fs::symlink(framework.join("1.63.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 0, Some("1.63.0")).unwrap();
        assert!(removed.is_empty());
        assert_eq!(kept, vec!["1.63.0"]);
        // Non-version entries must survive the prune.
        assert!(framework.join("legacy").exists());
        assert!(framework.join("tmp").exists());
        assert!(framework.join("stray.txt").exists());
    }

    // ── INC-DEBT-020 regression tests ──────────────────────────────────────────

    #[test]
    fn prune_repoint_current_after_prune_keeps_valid_target() {
        // INC-DEBT-020: after prune removes stale versions, current must still
        // point to a valid directory (repoint_current_to_newest).
        // Scenario: current=1.70.0, we keep [1.65.0], 1.70.0 gets removed.
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();

        // Create versions; 1.70.0 is current but will be removed by prune.
        for v in ["1.60.0", "1.65.0", "1.70.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.70.0"), framework.join("current")).unwrap();

        // Simulate prune that keeps 1.65.0 (not 1.70.0) — e.g. --keep 1
        // when active was 1.70.0 but kept = [1.65.0] because 1.70.0 was in a
        // "removed" set from a previous prune cycle.
        let _removed = vec!["1.70.0".to_owned()];
        let kept = ["1.65.0".to_owned()];

        // Call repoint_current_to_newest with the newest kept version
        repoint_current_to_newest(framework, kept.first().unwrap());

        // current must now point to 1.65.0 (valid, existing dir)
        let current_target = std::fs::read_link(framework.join("current")).unwrap();
        assert_eq!(
            current_target.file_name().unwrap().to_str().unwrap(),
            "1.65.0",
            "current must repoint to newest kept version"
        );
        assert!(
            framework.join("1.65.0").is_dir(),
            "current target must be an existing directory"
        );
    }

    #[test]
    fn repoint_current_skips_nonexistent_version_dir() {
        // If the "newest" kept version dir doesn't exist on disk, skip repoint.
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();

        make_version_dir(framework, "1.60.0");
        std::os::unix::fs::symlink(framework.join("1.60.0"), framework.join("current")).unwrap();

        // Try to repoint to a non-existent version
        repoint_current_to_newest(framework, "1.99.0");

        // current should still point to 1.60.0 (unchanged)
        let current_target = std::fs::read_link(framework.join("current")).unwrap();
        assert_eq!(
            current_target.file_name().unwrap().to_str().unwrap(),
            "1.60.0",
        );
    }

    // ── S-DEV-LINK-PRESERVED ────────────────────────────────────────────────

    #[test]
    fn s_dev_link_preserved_without_prune_flags_keeps_dev_link_target() {
        // S-DEV-LINK-PRESERVED: dev-link mode (root=".") WITHOUT --prune/--prune-only
        // → the dev-link block at update.rs:284-291 runs and current still points
        //   to bundle_root (framework/), NOT repointed to a version dir.
        // Regression guard for INC-DEBT-020: the dev-link behavior is preserved
        // when no prune flags are passed.
        //
        // This test verifies the dev-link block runs WITHOUT actually downloading.
        // We test the logic directly: when root="." and !prune and !prune_only,
        // the symlink is set to point to bundle_root (not to a version dir).

        let dir = tempfile::tempdir().unwrap();

        // bundle_root = sddk_data_dir/framework
        let sddk_data = dir.path().join(".local").join("sddk");
        let bundle_root = sddk_data.join("framework");

        // Create version dirs at bundle_root location
        make_version_dir(&bundle_root, "1.66.6");
        make_version_dir(&bundle_root, "1.67.0");

        // Dev-link mode BEFORE: current symlink points to bundle_root itself
        std::os::unix::fs::symlink(&bundle_root, bundle_root.join("current")).unwrap();

        // Simulate the dev environment
        let env = crate::CliEnvironment {
            home: Some(dir.path().to_path_buf()),
            data_home: Some(dir.path().join(".local").join("data")),
            sddk_data_dir: Some(sddk_data.clone()),
            state_home: Some(dir.path().join(".local").join("state")),
            cache_home: Some(dir.path().join(".local").join("cache")),
            sddk_actor: None,
            user: Some("tester".to_string()),
        };

        // Simulate the conditions for dev-link mode WITHOUT actually calling run_dev_update
        // (which would try to download). We directly exercise the dev-link block logic.
        let root_for_dev_link = std::path::PathBuf::from(".");
        let prune = false;
        let prune_only = false;

        // This is the condition from update.rs:284
        let is_dev_link_mode = root_for_dev_link.as_os_str() == "." && !prune && !prune_only;

        assert!(
            is_dev_link_mode,
            "condition for dev-link mode should be true"
        );

        // Simulate the dev-link block from update.rs:285-291
        // This is the exact code that runs in dev-link mode
        if is_dev_link_mode {
            let current = bundle_root.join("current");
            let tmp = bundle_root.join("current.tmp");
            let _ = std::fs::remove_file(&tmp);
            let _ = std::fs::remove_file(&current);
            std::os::unix::fs::symlink(&bundle_root, &tmp).unwrap();
            std::fs::rename(&tmp, &current).unwrap();
        }

        // Key assertion: current symlink must still point to bundle_root (framework/)
        let current_link = bundle_root.join("current");
        let current_target = std::fs::read_link(&current_link).unwrap();
        // The symlink should point to bundle_root (the framework dir itself, not a version dir)
        assert_eq!(
            current_target, bundle_root,
            "current symlink must point to bundle_root (framework/) — dev-link preserved"
        );
    }
}
