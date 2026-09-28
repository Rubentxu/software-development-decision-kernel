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

/// The framework directory override (`$SDDK_FRAMEWORK_DIR`).
///
/// INC-A5-FWDIR: `scripts/install.sh` documents and honors this variable
/// (ADR-0011 names it as the override surface), and the CI smoke job sets it
/// to a non-default path. The CLI ignored it, so every install whose
/// framework dir did not resolve through the data root died at the
/// `dev use` stage ("bundle version X not installed") and the installer
/// rolled back. v2.2.11 (run 36477625442) hid this defect behind the
/// signature-verification failure; v2.2.12 (run 36482350538, job
/// "Smoke test installer (end-to-end)") exposed it on the real pipeline.
fn framework_dir_override(environment: &CliEnvironment) -> Option<PathBuf> {
    environment.framework_dir.clone()
}

/// The `framework/` dir inside the data root (bundles per version + `current`).
///
/// Resolution order: `$SDDK_FRAMEWORK_DIR` (installer contract, ADR-0011)
/// → `$SDDK_DATA_DIR/framework` → `$XDG_DATA_HOME/sddk/framework` →
/// `$HOME/.local/share/sddk/framework`.
pub(super) fn framework_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    if let Some(dir) = framework_dir_override(environment) {
        return Ok(dir);
    }
    Ok(sddk_data_dir(environment)?.join("framework"))
}

/// Resolve the active framework root: `current` symlink target, else the
/// latest installed version, else the data dir (empty).
pub(crate) fn resolve_active_framework_root(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// INC-A5-FWDIR: the installer contract (ADR-0011, scripts/install.sh
    /// line 40) honors `$SDDK_FRAMEWORK_DIR`; the CI smoke job sets it to a
    /// non-default path (release.yml). The CLI must resolve `dev use`,
    /// `dev doctor`, `dev link` and friends against that override, not
    /// silently against `$SDDK_DATA_DIR/../framework`.
    #[test]
    fn framework_dir_override_wins_over_data_root() {
        let environment = CliEnvironment {
            sddk_data_dir: Some(PathBuf::from("/tmp/data-root")),
            framework_dir: Some(PathBuf::from("/tmp/sddk-smoke-framework")),
            ..CliEnvironment::default()
        };
        assert_eq!(
            framework_dir(&environment).unwrap(),
            PathBuf::from("/tmp/sddk-smoke-framework")
        );
    }

    #[test]
    fn framework_dir_falls_back_to_data_root() {
        let environment = CliEnvironment {
            sddk_data_dir: Some(PathBuf::from("/tmp/data-root")),
            framework_dir: None,
            ..CliEnvironment::default()
        };
        assert_eq!(
            framework_dir(&environment).unwrap(),
            PathBuf::from("/tmp/data-root/framework")
        );
    }
}
