//! Framework XDG data-root and version directory resolution.

use crate::CliEnvironment;
use std::path::PathBuf;

/// The canonical signing-keys directory.
///
/// All signing and verification operations use this single location:
/// `$SDDK_DATA_DIR/keys/` → `~/.local/share/sddk/keys/gate-signing.key`
///
/// This is intentionally NOT under `project_data` so that keys are shared
/// across projects and not tied to a specific cycle or project identity.
pub(crate) fn signing_keys_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    Ok(sddk_data_dir(environment)?.join("keys"))
}

pub(super) fn sddk_data_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    if let Some(dir) = &environment.sddk_data_dir {
        return Ok(dir.clone());
    }
    let data_home = match (&environment.data_home, &environment.home) {
        (Some(data), _) => data.clone(),
        (None, Some(home)) => home.join(".local/share"),
        (None, None) => dirs::data_dir().ok_or_else(|| {
            anyhow::anyhow!("no data root: set HOME, XDG_DATA_HOME or SDDK_DATA_DIR")
        })?,
    };
    Ok(data_home.join("sddk"))
}

/// The `framework/` dir inside the data root (bundles per version + `current`).
pub(super) fn framework_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    Ok(sddk_data_dir(environment)?.join("framework"))
}

/// Resolve the active framework root: `current` symlink target, else the
/// latest installed version, else the data dir (empty).
pub(super) fn resolve_active_framework_root(
    environment: &CliEnvironment,
) -> anyhow::Result<PathBuf> {
    let dir = framework_dir(environment)?;
    let current = dir.join("current");
    if let Ok(target) = std::fs::read_link(&current) {
        if target.is_absolute() {
            return Ok(target);
        }
        return Ok(dir.join(target));
    }
    // Fall back to the highest installed version.
    let mut versions: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name != "current")
                .collect()
        })
        .unwrap_or_default();
    versions.sort();
    versions
        .last()
        .map(|version| dir.join(version))
        .ok_or_else(|| {
            anyhow::anyhow!("no framework bundle installed; run `sddk dev update --root <dir>`")
        })
}

/// Resolve the static `assets/` directory of the active framework root
/// (ADR-0013: dashboard kit shipped in the bundle). Returns `None` when the
/// bundle has no assets (pre-1.5.0 bundles are still supported).
pub(crate) fn resolve_assets_dir(environment: &CliEnvironment) -> anyhow::Result<Option<PathBuf>> {
    let root = resolve_active_framework_root(environment)?;
    let assets = root.join("assets");
    if assets.is_dir() {
        return Ok(Some(assets));
    }
    // Dogfooding fallback: when running from the framework development repo
    // (which carries `manifest.toml` and an `assets/` tree), resolve there.
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("manifest.toml").is_file() {
        let repo_assets = cwd.join("assets");
        if repo_assets.is_dir() {
            return Ok(Some(repo_assets));
        }
    }
    Ok(None)
}
