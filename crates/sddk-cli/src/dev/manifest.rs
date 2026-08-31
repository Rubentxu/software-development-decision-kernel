//! `dev manifest` — generate or verify MANIFEST.sha256.

// `dead_code` allowed locally — `MANIFEST_FILE` and helpers are kept
// for the manifest subsystem API surface but not used by every caller.
// A future docs/hygiene cycle should remove the unused items.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use sddk_gateway::GitExecutor;
use sha2::Digest;

use crate::CommandOutput;

use super::common::{MANIFEST_SURFACES, atomic_write, sha256_hex};

/// Manifest file name, written at the framework root (and shipped in the
/// release bundle).
pub(crate) const MANIFEST_FILE: &str = "MANIFEST.sha256";

/// Run the `dev manifest` subcommand.
pub(super) fn run_dev_manifest(args: super::ManifestArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let root = args
            .root
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        if args.verify {
            let mismatches = verify_manifest(&root)?;
            if mismatches.is_empty() {
                return Ok(format!(
                    "manifest OK: {} verified against {}",
                    root.display(),
                    root.join(MANIFEST_FILE).display()
                ));
            }
            anyhow::bail!(
                "manifest verification FAILED ({}):\n  {}",
                mismatches.len(),
                mismatches.join("\n  ")
            );
        }
        let count = write_manifest(&root)?;
        let mut lines = vec![format!(
            "manifest written: {} ({} files hashed)",
            root.join(MANIFEST_FILE).display(),
            count
        )];
        if args.bundle {
            let bundle_version = args.bundle_version.as_deref().ok_or_else(|| {
                anyhow::anyhow!("--bundle-version is required when --bundle is set")
            })?;
            let min = args
                .binary_min_version
                .clone()
                .unwrap_or_else(|| bundle_version.to_owned());
            let max = args
                .binary_max_version
                .clone()
                .unwrap_or_else(|| bundle_version.to_owned());
            let mut contents = count_surface_entries(&root)?;
            // Record the manifest hash itself so the BUNDLE.toml carries a
            // cryptographic anchor to the per-file manifest.
            let manifest_bytes = std::fs::read(root.join(MANIFEST_FILE))?;
            contents.manifest_sha256 = Some(format!(
                "sha256:{:x}",
                sha2::Sha256::digest(&manifest_bytes)
            ));
            let bundle_path = crate::dev::bundle_manifest::write_bundle_manifest(
                &root,
                bundle_version,
                &min,
                &max,
                contents,
            )?;
            lines.push(format!(
                "bundle manifest written: {} (binary compat [{}, {}])",
                bundle_path.display(),
                min,
                max
            ));
        }
        Ok(lines.join("\n"))
    })();
    super::super::render_result(result, format, |t| t.clone())
}

/// Verify a framework root against its MANIFEST.sha256. Returns the list of
/// mismatches (empty = intact). A missing manifest is reported as a single
/// entry. Duplicate manifest entries are reported as mismatch.
pub(crate) fn verify_manifest(root: &Path) -> anyhow::Result<Vec<String>> {
    let manifest_path = root.join(MANIFEST_FILE);
    let raw = std::fs::read_to_string(&manifest_path)?;
    let mut mismatches = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    for line in raw.lines() {
        if line.is_empty() {
            continue;
        }
        let (expected, encoded_relative) = line
            .split_once("  ")
            .ok_or_else(|| anyhow::anyhow!("malformed manifest line: {line}"))?;
        let relative = decode_manifest_path(encoded_relative)?;

        // Detect duplicate manifest entries
        if !seen_paths.insert(relative.clone()) {
            mismatches.push(format!("{relative}: duplicate manifest entry"));
            continue;
        }

        let file = root.join(&relative);
        if !file.is_file() {
            mismatches.push(format!("{relative}: missing"));
            continue;
        }
        let actual = sha256_hex(&file)?;
        if actual != expected {
            mismatches.push(format!("{relative}: hash mismatch"));
        }
    }
    Ok(mismatches)
}

/// Compute the full list of (relative_path, sha256_hex) entries for all
/// framework surfaces.
fn manifest_entries(root: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let git = GitExecutor::new(root.to_path_buf());
    match git.is_inside_work_tree() {
        Ok(true) => git_tracked_manifest_entries(root, &git),
        Ok(false) => filesystem_manifest_entries(root),
        Err(error) => Err(anyhow::anyhow!(
            "cannot determine Git worktree membership: {error}"
        )),
    }
}

/// Build manifest entries by walking the framework surfaces on the filesystem.
/// Used when the framework root is not inside a Git worktree.
fn filesystem_manifest_entries(root: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let mut entries = Vec::new();
    for surface in MANIFEST_SURFACES {
        let dir = root.join(surface);
        if !dir.is_dir() {
            continue;
        }
        for file in super::common::walk_dir(&dir) {
            if !file.is_file() {
                continue;
            }
            let relative = file
                .strip_prefix(root)
                .unwrap_or(file.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            let digest = sha256_hex(&file)?;
            entries.push((relative, digest));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

/// Build manifest entries by running `git ls-files` inside a worktree.
/// Files not tracked by Git are excluded. Fails closed if a tracked file
/// is missing from the working tree (was deleted but not `git rm`ed).
fn git_tracked_manifest_entries(
    root: &Path,
    git: &GitExecutor,
) -> anyhow::Result<Vec<(String, String)>> {
    let tracked = git
        .ls_files(&MANIFEST_SURFACES)
        .map_err(|error| anyhow::anyhow!("git ls-files failed: {error}"))?;
    let surfaces: Vec<_> = MANIFEST_SURFACES.iter().map(Path::new).collect();
    let mut entries = Vec::new();
    for path in tracked {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("git returned a non-UTF-8 tracked path"))?;
        let relative_path = Path::new(path_str);
        if !surfaces
            .iter()
            .any(|surface| relative_path.starts_with(surface))
        {
            continue;
        }
        let abs = root.join(&path);
        // Only regular files belong to the published manifest.
        match std::fs::symlink_metadata(&abs).map(|m| m.file_type().is_file()) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_) => anyhow::bail!("tracked file missing from working tree: {}", path_str),
        }
        let digest = sha256_hex(&abs)?;
        entries.push((path_str.to_owned(), digest));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.dedup_by(|a, b| a.0 == b.0);
    Ok(entries)
}

/// Encode a manifest path for storage in MANIFEST.sha256 (escape backslash, newline, carriage return).
fn encode_manifest_path(path: &str) -> String {
    path.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Decode a manifest path from storage.
fn decode_manifest_path(path: &str) -> anyhow::Result<String> {
    let mut decoded = String::with_capacity(path.len());
    let mut chars = path.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        match chars.next() {
            Some('\\') => decoded.push('\\'),
            Some('n') => decoded.push('\n'),
            Some('r') => decoded.push('\r'),
            Some(other) => anyhow::bail!("unsupported manifest path escape: \\{other}"),
            None => anyhow::bail!("unterminated manifest path escape"),
        }
    }
    Ok(decoded)
}

/// Serialize manifest entries as escaped `sha256  relative-path` lines.
fn manifest_lines(entries: &[(String, String)]) -> String {
    entries
        .iter()
        .map(|(path, digest)| format!("{digest}  {}", encode_manifest_path(path)))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Generate MANIFEST.sha256 at the framework root. Returns the number of
/// hashed files.
pub(super) fn write_manifest(root: &Path) -> anyhow::Result<usize> {
    let entries = manifest_entries(root)?;
    let content = manifest_lines(&entries);
    let target = root.join(MANIFEST_FILE);
    atomic_write(&target, content.as_bytes(), None)?;
    Ok(entries.len())
}

/// Tally the framework surface counts for the BUNDLE.toml `contents` section.
///
/// Walks the same `MANIFEST_SURFACES` roots as `manifest_entries` but counts
/// files per surface. Used by `write_bundle_manifest_for_root` to populate
/// `agents_count`, `skills_count`, `prompts_count`, `assets_count`.
pub(super) fn count_surface_entries(
    root: &Path,
) -> anyhow::Result<crate::dev::bundle_manifest::ContentsSection> {
    use crate::dev::bundle_manifest::ContentsSection;
    let mut counts = ContentsSection::default();
    for surface in super::common::MANIFEST_SURFACES {
        let dir = root.join(surface);
        if !dir.is_dir() {
            continue;
        }
        let count = count_files_recursive(&dir)?;
        match surface {
            "agents" => counts.agents_count = count,
            "skills" => counts.skills_count = count,
            "prompts" => counts.prompts_count = count,
            "assets" => counts.assets_count = count,
            _ => {}
        }
    }
    Ok(counts)
}

fn count_files_recursive(dir: &Path) -> anyhow::Result<u32> {
    let mut count = 0u32;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            count += 1;
        } else if file_type.is_dir() {
            count += count_files_recursive(&entry.path())?;
        }
    }
    Ok(count)
}

// ── Tests ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/manifest_tests.rs"]
mod manifest_tests;
