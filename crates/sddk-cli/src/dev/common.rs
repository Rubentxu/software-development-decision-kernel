//! Shared I/O helpers and constants used across dev subcommands.

use std::path::{Path, PathBuf};

use crate::CommandOutput;
use sddk_gateway::PermissionPolicy;

pub(super) const RECEIPT_FILE: &str = "sddk-install.json";

pub(crate) const MANIFEST_SURFACES: [&str; 4] = ["agents", "skills", "prompts/sddk", "assets"];

pub(super) fn read_receipt(prefix: &Path) -> anyhow::Result<super::InstallReceipt> {
    let path = prefix.join(RECEIPT_FILE);
    if !path.exists() {
        anyhow::bail!("no installation receipt at {path:?}");
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

pub(super) fn tool_version(tool: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new(tool).arg("--version").output()?;
    if !output.status.success() {
        anyhow::bail!("{tool} exited {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub(crate) fn atomic_write(
    destination: &Path,
    bytes: &[u8],
    mode: Option<u32>,
) -> anyhow::Result<()> {
    use std::io::Write;
    let parent = destination.parent().expect("destination has a parent");
    std::fs::create_dir_all(parent)?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");

    // Linux error code for "text file busy" (ETXTBSY).
    // Returned by rename(2) when the destination is currently open for execution.
    #[cfg(unix)]
    const ETXTBSY: i32 = 26;

    let mut last_error = None;
    for attempt in 0..100 {
        let temporary = parent.join(format!(
            ".{file_name}.tmp-{}-{}",
            std::process::id(),
            attempt
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                let result = (|| -> std::io::Result<()> {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    drop(file);
                    // chmod BEFORE rename so the destination is born with
                    // the requested mode (no 0644 window). Unix-only.
                    #[cfg(unix)]
                    {
                        if let Some(bits) = mode {
                            use std::os::unix::fs::PermissionsExt;
                            std::fs::set_permissions(
                                &temporary,
                                std::fs::Permissions::from_mode(bits),
                            )?;
                        }
                    }
                    std::fs::rename(&temporary, destination)
                })();

                if result.is_ok() {
                    return Ok(());
                }

                let source = result.unwrap_err();

                // ETXTBSY: destination is open for execution — retry after a
                // short delay so the kernel can finish with it. The temp file is
                // removed before retry; a fresh temp is created in the next loop
                // iteration with an incremented attempt number.
                #[cfg(unix)]
                if source.raw_os_error() == Some(ETXTBSY) {
                    let _ = std::fs::remove_file(&temporary);
                    if attempt + 1 < 100 {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        last_error = None;
                        continue;
                    }
                }

                let _ = std::fs::remove_file(&temporary);
                return Err(source.into());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(error);
            }
            Err(source) => return Err(source.into()),
        }
    }
    Err(last_error
        .unwrap_or_else(|| std::io::Error::other("no temporary path available"))
        .into())
}

pub(super) fn failure_status(message: String) -> CommandOutput {
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}

/// Format an `InstallReceipt` as human-readable text.
pub(super) fn receipt_text(receipt: &super::InstallReceipt) -> String {
    format!(
        "version: {}\ncommit: {}\nbinary_sha256: {}\nchannel: {}\ninstalled_at: {}\nbinary_path: {}\nbundle: {}\n",
        receipt.version,
        receipt.commit,
        receipt.binary_sha256,
        receipt.channel,
        receipt.installed_at,
        receipt.binary_path,
        receipt.bundle
    )
}

pub(super) fn walk_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_dir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

/// Names of framework agents: declared in permissions.yaml AND present in agents/*.md.
pub(super) fn framework_agent_names(root: &Path) -> Vec<String> {
    let agents_dir = root.join("agents");
    // Prefer the permission policy when present; fall back to the actual
    // agent files (release bundles may omit permissions.yaml).
    if let Ok(policy) = PermissionPolicy::from_file(root.join("permissions.yaml")) {
        let mut names: Vec<String> = policy
            .agents()
            .filter(|name| agents_dir.join(format!("{name}.md")).exists())
            .map(str::to_owned)
            .collect();
        names.sort();
        return names;
    }
    let mut names: Vec<String> = std::fs::read_dir(&agents_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("md"))
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Compute the plain lowercase hex SHA-256 of a file.
pub(crate) fn sha256_hex(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// Download a URL to a destination via curl/wget, or copy from file://.
pub(super) fn download_to(url: &str, destination: &Path) -> anyhow::Result<()> {
    if let Some(source) = url.strip_prefix("file://") {
        std::fs::copy(source, destination)?;
        return Ok(());
    }
    let status = std::process::Command::new("curl")
        .args(["-fsSL", "--retry", "3", "-o"])
        .arg(destination)
        .arg(url)
        .status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => anyhow::bail!("curl exited {status} for {url}"),
        Err(_) => {
            let status = std::process::Command::new("wget")
                .args(["-qO"])
                .arg(destination)
                .arg(url)
                .status()?;
            if status.success() {
                Ok(())
            } else {
                anyhow::bail!("wget exited {status} for {url}")
            }
        }
    }
}

/// How [`copy_tree`] decides which files to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyMode {
    /// Copy every file, staging into a sibling temp dir and renaming into
    /// place (atomic swap when the target already exists).
    Always,
    /// Copy only files whose content differs from the destination
    /// (idempotent reinstall; preserves mtimes of unchanged files).
    IfChanged,
}

/// Copies the file tree under `source` into `target`.
///
/// `Always` stages the copy into `<target>.tmp-<pid>` and renames it into
/// place — a failure mid-copy leaves the previous target intact and cleans
/// the staging dir. If the target already exists it is swapped out to
/// `<target>.old-<pid>` first and removed after the swap.
pub(crate) fn copy_tree(source: &Path, target: &Path, mode: CopyMode) -> anyhow::Result<()> {
    match mode {
        CopyMode::IfChanged => {
            if !source.is_dir() {
                anyhow::bail!(
                    "copy_tree IfChanged called with non-directory source: {}",
                    source.display()
                );
            }
            for file in walk_dir(source) {
                if !file.is_file() {
                    continue;
                }
                let relative = file.strip_prefix(source)?;
                let destination = target.join(relative);
                let needs_copy = match (std::fs::read(&file), std::fs::read(&destination)) {
                    (Ok(src), Ok(dst)) => src != dst,
                    _ => true,
                };
                if needs_copy {
                    if let Some(parent) = destination.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(&file, &destination)?;
                }
            }
            Ok(())
        }
        CopyMode::Always => {
            if !source.is_dir() {
                anyhow::bail!(
                    "copy_tree Always called with non-directory source: {}",
                    source.display()
                );
            }
            let staging = target.with_file_name(format!(
                "{}.tmp-{}",
                target.file_name().unwrap_or_default().to_string_lossy(),
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&staging);
            copy_tree(source, &staging, CopyMode::IfChanged)?;
            // Swap: park the old target (if any), rename staging into place,
            // then remove the parked copy. Any failure before the final
            // rename leaves the original target intact.
            let parked = target.with_file_name(format!(
                "{}.old-{}",
                target.file_name().unwrap_or_default().to_string_lossy(),
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&parked);
            let had_target = target.exists();
            if had_target {
                std::fs::rename(target, &parked)?;
            }
            if let Err(error) = std::fs::rename(&staging, target) {
                // Roll back: restore the parked target before failing.
                if had_target {
                    std::fs::rename(&parked, target)?;
                }
                return Err(error.into());
            }
            let _ = std::fs::remove_dir_all(&parked);
            Ok(())
        }
    }
}

/// Count entries in a root's MANIFEST.sha256 (0 when absent).
pub(super) fn count_manifest_entries(root: &Path) -> anyhow::Result<usize> {
    let raw = std::fs::read_to_string(root.join(super::manifest::MANIFEST_FILE))?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).count())
}

#[cfg(test)]
#[path = "tests/copy_tree_tests.rs"]
mod copy_tree_tests;
