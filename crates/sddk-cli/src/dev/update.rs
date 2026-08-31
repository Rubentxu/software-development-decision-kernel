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

pub(super) fn run_dev_update(
    args: super::UpdateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let mut output = String::new();

        // The framework distributes RELEASE BUNDLES (agents/skills/prompts/
        // workflows/assets + MANIFEST.sha256), never repository clones. Git
        // operations are the developer's responsibility: if the target root
        // is a checkout, the user updates it with `git pull` themselves.
        if args.root.join(".git").is_dir() {
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
        output.push_str(&update_bundle(&bundle_root, &args)?);

        // The extracted bundle lands in a version dir; update_bundle extracts
        // directly into bundle_root, so if the user passed the framework root
        // we additionally fix the `current` symlink to point at it.
        if args.root.as_os_str() == "." {
            let current = bundle_root.join("current");
            let tmp = bundle_root.join("current.tmp");
            let _ = std::fs::remove_file(&tmp);
            let _ = std::fs::remove_file(&current);
            std::os::unix::fs::symlink(&bundle_root, &tmp)?;
            std::fs::rename(&tmp, &current)?;
            output.push_str("framework: current -> bundle root (dev link resolves it)\n");
        }
        Ok(output)
    })();
    render_result(result, format, |output: &String| output.clone())
}
