//! `BUNDLE.toml` — declarative per-bundle manifest declaring the bundle
//! version and binary compatibility range.
//!
//! Every release bundle (the versioned tree under
//! `~/.local/share/sddk/framework/<v>/` or the unified tarball section) MUST
//! carry a `BUNDLE.toml`. Without it, `sddk dev install` refuses to record
//! coherence metadata and `sddk dev doctor` fails the
//! `binary_bundle_coherence` check.
//!
//! Format example:
//!
//! ```toml
//! [bundle]
//! schema_version = 2
//! version = "1.62.0"
//! binary_min_version = "1.62.0"
//! binary_max_version = "1.62.0"     # exact match when equal to min
//!
//! [contents]
//! agents_count = 142
//! skills_count = 87
//! prompts_count = 18
//! assets_count = 5
//! manifest_sha256 = "608c6d9c..."   # sha256 of MANIFEST.sha256 itself
//! ```

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::Digest;

/// Canonical name of the bundle manifest file inside every bundle root.
pub(super) const BUNDLE_MANIFEST_FILE: &str = "BUNDLE.toml";

/// Current schema version understood by this CLI.
pub(super) const BUNDLE_MANIFEST_SCHEMA: u32 = 2;

/// Errors that can occur while parsing or validating a BUNDLE.toml.
#[derive(Debug, thiserror::Error)]
pub enum BundleManifestError {
    #[error("BUNDLE.toml not found at {0}")]
    NotFound(String),
    #[error("BUNDLE.toml at {path}: {message}")]
    Parse { path: String, message: String },
    #[error("BUNDLE.toml schema_version {found} is not supported (max supported: {max})")]
    UnsupportedSchema { found: u32, max: u32 },
    #[error(
        "binary version {binary} is not compatible with bundle {bundle}: required range [{min}, {max}]"
    )]
    IncompatibleBinary {
        bundle: String,
        binary: String,
        min: String,
        max: String,
    },
    /// The `manifest_sha256` declared by `BUNDLE.toml` does not match the
    /// actual sha256 of the bundle's `MANIFEST.sha256`.
    ///
    /// Added session-30 (INC-DEBT-025 part 2). The field was written and
    /// parsed but never compared, so it was inert: a bundle whose
    /// `MANIFEST.sha256` had been rewritten would still validate.
    #[error(
        "BUNDLE.toml manifest_sha256 mismatch: declares {declared} but MANIFEST.sha256 hashes to {actual}"
    )]
    ManifestShaMismatch { declared: String, actual: String },
    /// The bundle declares a `manifest_sha256` but ships no `MANIFEST.sha256`
    /// to compare it against.
    #[error("BUNDLE.toml declares manifest_sha256 {declared} but MANIFEST.sha256 is missing")]
    ManifestMissing { declared: String },
}

/// Declarative bundle manifest (top-level shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifest {
    #[serde(rename = "bundle")]
    pub bundle: BundleSection,
    #[serde(default, rename = "contents")]
    pub contents: ContentsSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleSection {
    pub schema_version: u32,
    pub version: String,
    pub binary_min_version: String,
    pub binary_max_version: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentsSection {
    #[serde(default)]
    pub agents_count: u32,
    #[serde(default)]
    pub skills_count: u32,
    #[serde(default)]
    pub prompts_count: u32,
    #[serde(default)]
    pub assets_count: u32,
    #[serde(default)]
    pub manifest_sha256: Option<String>,
}

/// Parse a BUNDLE.toml at `path` and validate its schema version.
pub(super) fn parse_bundle_manifest(path: &Path) -> Result<BundleManifest, BundleManifestError> {
    if !path.is_file() {
        return Err(BundleManifestError::NotFound(path.display().to_string()));
    }
    let raw = std::fs::read_to_string(path).map_err(|e| BundleManifestError::Parse {
        path: path.display().to_string(),
        message: format!("read error: {e}"),
    })?;
    let parsed: BundleManifest = toml::from_str(&raw).map_err(|e| BundleManifestError::Parse {
        path: path.display().to_string(),
        message: format!("TOML parse error: {e}"),
    })?;
    if parsed.bundle.schema_version > BUNDLE_MANIFEST_SCHEMA {
        return Err(BundleManifestError::UnsupportedSchema {
            found: parsed.bundle.schema_version,
            max: BUNDLE_MANIFEST_SCHEMA,
        });
    }
    Ok(parsed)
}

/// Verify that `binary_version` is within the bundle's declared compatibility
/// range. Both `min` and `max` are inclusive. The comparison uses SemVer-like
/// ordering on the dotted-numeric prefix (e.g. `1.62.0` vs `1.62.0-rc.1`):
/// numeric components compared pairwise; a `0` trailing suffix is treated as
/// lower than any non-zero suffix.
pub(super) fn verify_bundle_compat(
    manifest: &BundleManifest,
    binary_version: &str,
) -> Result<(), BundleManifestError> {
    let binary = binary_version.trim_start_matches('v');
    let min = manifest.bundle.binary_min_version.trim_start_matches('v');
    let max = manifest.bundle.binary_max_version.trim_start_matches('v');
    if version_gte(binary, min) && version_lte(binary, max) {
        Ok(())
    } else {
        Err(BundleManifestError::IncompatibleBinary {
            bundle: manifest.bundle.version.clone(),
            binary: binary.to_owned(),
            min: min.to_owned(),
            max: max.to_owned(),
        })
    }
}

/// Bundles published before the anchor meant anything, whose declared
/// `manifest_sha256` is **not** a manifest digest at all.
///
/// Added session-30 retrospectiva. Verified against the real published
/// assets, not inferred: v1.172.0, v2.0.0 and v2.0.1 all declare
///
/// ```text
/// manifest_sha256 = "608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f"
/// ```
///
/// and that value is the sha256 of `agents/analytics-judge.md` — a single
/// *file* inside the bundle, because the buggy `awk 'NR==1 {print $1}'`
/// took the first line of the manifest. It is the same constant in all three
/// releases precisely because the same file sorted first each time. The real
/// manifest digests differ: v2.0.1's is `32cfd79f…`.
///
/// So these bundles cannot satisfy a manifest-digest check, and the fix for
/// the format/prefix divergence is deliberately **not** "accept any value" —
/// that would silently re-open the hole the check exists to close. Instead the
/// exemption is pinned to the exact (version, declared value) pairs that were
/// actually published, so:
/// - a new bundle gets a real anchor, checked normally;
/// - a legacy bundle still installs;
/// - a legacy *version* carrying any other value still fails, because a
///   tampered bundle would have to keep the published constant to slip past.
///
/// Removing an entry from this list is a deliberate act of deprecation, not
/// an accident.
const LEGACY_PUBLISHED_ANCHORS: &[(&str, &str)] = &[
    (
        "1.172.0",
        "608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f",
    ),
    (
        "2.0.0",
        "608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f",
    ),
    (
        "2.0.1",
        "608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f",
    ),
];

/// True when `(version, declared)` is exactly one of the published legacy
/// pairs in [`LEGACY_PUBLISHED_ANCHORS`].
fn is_legacy_published_anchor(version: &str, declared: &str) -> bool {
    let version = version.trim_start_matches('v');
    // Compare the bare hex so the exemption is not sensitive to whether a
    // given producer emitted the `sha256:` prefix.
    let declared_bare = declared.trim().trim_start_matches("sha256:");
    LEGACY_PUBLISHED_ANCHORS
        .iter()
        .any(|(v, sha)| *v == version && *sha == declared_bare)
}

/// Verify that `BUNDLE.toml`'s `manifest_sha256` really anchors the bundle's
/// `MANIFEST.sha256`.
///
/// Added session-30 (INC-DEBT-025 part 2). Until now the field was written by
/// `dev manifest --bundle` and parsed by [`parse_bundle_manifest`], but no code
/// ever compared the two, so the anchor was decorative: rewriting
/// `MANIFEST.sha256` inside a bundle left the bundle validating.
///
/// Fail-closed, consistent with the rest of schema v2:
///
/// - Declared and matching → `Ok`.
/// - Declared and mismatching → [`BundleManifestError::ManifestShaMismatch`],
///   unless the pair is an exactly-known legacy publication (see
///   [`LEGACY_PUBLISHED_ANCHORS`]).
/// - Declared but `MANIFEST.sha256` absent → [`BundleManifestError::ManifestMissing`].
/// - **Not declared** → `Ok`. A bundle that omits the field is not *claiming*
///   an anchor, so rejecting it would break every already-published release
///   whose `BUNDLE.toml` predates the anchor. This is the one case that stays
///   permissive on purpose; `verify_manifest` still checks every file against
///   whatever manifest is present.
///
/// Both the `sha256:`-prefixed form (written by `dev manifest --bundle`) and
/// the bare-hex form (written by `release.sh`, the actual publisher) are
/// accepted, because the field has two producers with divergent formats in
/// this very repo and requiring only one would reject the other's output.
pub(super) fn verify_manifest_anchor(
    bundle_root: &Path,
    manifest: &BundleManifest,
) -> Result<(), BundleManifestError> {
    let Some(declared) = manifest
        .contents
        .manifest_sha256
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    else {
        return Ok(());
    };
    let declared = declared.trim();

    let manifest_path = bundle_root.join(crate::dev::manifest::MANIFEST_FILE);
    if !manifest_path.is_file() {
        return Err(BundleManifestError::ManifestMissing {
            declared: declared.to_owned(),
        });
    }

    // Checked AFTER the manifest must exist, and before comparing: a legacy
    // exemption is an excuse for a *wrong value*, never for a missing
    // manifest. Ordering matters here — checking the exemption first would
    // let a 2.0.1 bundle ship with no MANIFEST.sha256 at all and validate.
    if is_legacy_published_anchor(&manifest.bundle.version, declared) {
        return Ok(());
    }

    let bytes = std::fs::read(&manifest_path).map_err(|e| BundleManifestError::Parse {
        path: manifest_path.display().to_string(),
        message: format!("read error: {e}"),
    })?;
    let actual = format!("sha256:{:x}", sha2::Sha256::digest(&bytes));

    if normalise_digest(declared) == normalise_digest(&actual) {
        Ok(())
    } else {
        Err(BundleManifestError::ManifestShaMismatch {
            declared: declared.to_owned(),
            actual,
        })
    }
}

/// Strip the optional `sha256:` prefix so both producer formats compare equal.
fn normalise_digest(value: &str) -> &str {
    value.trim().trim_start_matches("sha256:")
}

/// Write a BUNDLE.toml at `bundle_root/BUNDLE.toml` with the supplied metadata.
pub(super) fn write_bundle_manifest(
    bundle_root: &Path,
    version: &str,
    binary_min_version: &str,
    binary_max_version: &str,
    contents: ContentsSection,
) -> Result<std::path::PathBuf, BundleManifestError> {
    let manifest = BundleManifest {
        bundle: BundleSection {
            schema_version: BUNDLE_MANIFEST_SCHEMA,
            version: version.to_owned(),
            binary_min_version: binary_min_version.to_owned(),
            binary_max_version: binary_max_version.to_owned(),
        },
        contents,
    };
    let serialized = toml::to_string_pretty(&manifest).map_err(|e| BundleManifestError::Parse {
        path: bundle_root.display().to_string(),
        message: format!("serialization error: {e}"),
    })?;
    std::fs::create_dir_all(bundle_root).map_err(|e| BundleManifestError::Parse {
        path: bundle_root.display().to_string(),
        message: format!("create_dir_all error: {e}"),
    })?;
    let target = bundle_root.join(BUNDLE_MANIFEST_FILE);
    std::fs::write(&target, serialized).map_err(|e| BundleManifestError::Parse {
        path: target.display().to_string(),
        message: format!("write error: {e}"),
    })?;
    Ok(target)
}

// ── Version comparison helpers ────────────────────────────────────────────

/// Return true iff `a >= b` under SemVer-style ordering on dotted-numeric
/// components. Pre-release suffixes (`-rc.1`) sort lower than the same
/// version without suffix. A missing component is treated as `0`.
fn version_gte(a: &str, b: &str) -> bool {
    let ca = split_version(a);
    let cb = split_version(b);
    for i in 0..cmp::max(ca.len(), cb.len()) {
        let na = ca.get(i).copied().unwrap_or(0);
        let nb = cb.get(i).copied().unwrap_or(0);
        if na > nb {
            return true;
        }
        if na < nb {
            return false;
        }
    }
    // Numeric parts are equal: a pre-release suffix on either side lowers it.
    let sa = pre_release_rank(a);
    let sb = pre_release_rank(b);
    sa >= sb
}

fn version_lte(a: &str, b: &str) -> bool {
    let ca = split_version(a);
    let cb = split_version(b);
    for i in 0..cmp::max(ca.len(), cb.len()) {
        let na = ca.get(i).copied().unwrap_or(0);
        let nb = cb.get(i).copied().unwrap_or(0);
        if na < nb {
            return true;
        }
        if na > nb {
            return false;
        }
    }
    let sa = pre_release_rank(a);
    let sb = pre_release_rank(b);
    sa <= sb
}

fn split_version(v: &str) -> Vec<u64> {
    let head = v.split('-').next().unwrap_or(v);
    head.split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect()
}

/// Lower rank = pre-release; same numeric version with no suffix ranks higher
/// than one with `-rc.N` etc.
fn pre_release_rank(v: &str) -> u8 {
    if v.contains('-') { 0 } else { 1 }
}

// Minimal local `cmp::max` to avoid pulling `std::cmp` alias noise; the
// function is private and the import surface is intentionally narrow.
mod cmp {
    pub(super) fn max(a: usize, b: usize) -> usize {
        if a >= b { a } else { b }
    }
}

#[cfg(test)]
#[path = "tests/bundle_manifest_tests.rs"]
mod bundle_manifest_tests;
